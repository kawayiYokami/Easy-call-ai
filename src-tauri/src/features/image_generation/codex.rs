struct CodexImageAuth {
    access_token: String,
    account_id: Option<String>,
    base_url: String,
}

const CODEX_IMAGE_TOOL_OUTPUT_FORMAT: &str = "png";

async fn resolve_codex_image_auth(
    state: &AppState,
    provider: &ImageGenerationProviderConfig,
) -> Result<CodexImageAuth, String> {
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    let requested_id = provider
        .codex_api_provider_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let api_provider = requested_id
        .and_then(|id| config.api_providers.iter().find(|item| item.id == id))
        .or_else(|| {
            config
                .api_providers
                .iter()
                .find(|item| item.request_format.is_codex() && !item.deprecated)
        })
        .ok_or_else(|| "未找到可复用的 Codex API 供应商，请先在 API 设置中配置并登录 Codex。".to_string())?;
    if api_provider.deprecated || !api_provider.request_format.is_codex() {
        return Err(format!(
            "关联的 API 供应商不是可用的 Codex 供应商：{}",
            api_provider.name
        ));
    }

    let auth_mode = normalize_codex_auth_mode(&api_provider.codex_auth_mode);
    let (access_token, account_id) = if auth_mode == CODEX_AUTH_MODE_CUSTOM_URL {
        (
            api_provider
                .codex_custom_api_key
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
            None,
        )
    } else {
        let snapshot = read_codex_runtime_auth_snapshot(
            &api_provider.id,
            &auth_mode,
            &api_provider.codex_local_auth_path,
        )?;
        let fresh = ensure_codex_runtime_auth_fresh(&snapshot).await?;
        (fresh.access_token.clone(), fresh.account_id.clone())
    };
    if access_token.trim().is_empty() {
        return Err(format!(
            "Codex 供应商“{}”尚未登录或凭证已失效，请先在 API 设置中完成登录。",
            api_provider.name
        ));
    }
    let base_url = if auth_mode == CODEX_AUTH_MODE_CUSTOM_URL {
        api_provider
            .codex_custom_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&api_provider.base_url)
            .trim()
            .to_string()
    } else {
        api_provider.base_url.trim().to_string()
    };
    let base_url = if base_url.is_empty() {
        "https://chatgpt.com/backend-api/codex".to_string()
    } else {
        base_url
    };
    Ok(CodexImageAuth {
        access_token,
        account_id,
        base_url,
    })
}

fn codex_image_generation_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
    edit_inputs: &ImageEditInputs,
) -> Value {
    let mut tool = serde_json::json!({
        "type": "image_generation",
        "model": CODEX_IMAGE_TOOL_MODEL,
        "output_format": CODEX_IMAGE_TOOL_OUTPUT_FORMAT,
    });
    if let Some(size) = effective_image_generation_size(request, model) {
        tool["size"] = Value::String(size);
    }
    if let Some(quality) = effective_image_generation_quality(request, model) {
        tool["quality"] = Value::String(quality);
    }
    // Responses API 的 mask 通过 image_generation 工具配置传入，而非消息内容。
    if let Some(mask) = &edit_inputs.mask {
        tool["input_image_mask"] = serde_json::json!({ "image_url": image_edit_data_url(mask) });
    }
    let mut content = vec![serde_json::json!({
        "type": "input_text",
        "text": effective_image_generation_prompt(request)
    })];
    for input in &edit_inputs.images {
        content.push(serde_json::json!({
            "type": "input_image",
            "image_url": image_edit_data_url(input)
        }));
    }
    serde_json::json!({
        "instructions": "",
        "stream": true,
        "reasoning": {"effort": "medium", "summary": "auto"},
        "store": false,
        "parallel_tool_calls": true,
        "include": ["reasoning.encrypted_content"],
        "model": CODEX_IMAGE_MAIN_MODEL,
        "input": [{
            "type": "message",
            "role": "user",
            "content": content
        }],
        "tools": [tool],
        "tool_choice": { "type": "image_generation" }
    })
}

fn codex_image_output_mime(output_format: Option<&str>) -> &'static str {
    match output_format
        .unwrap_or(CODEX_IMAGE_TOOL_OUTPUT_FORMAT)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

fn collect_codex_image_value(
    value: &Value,
    final_images: &mut Vec<PendingGeneratedImage>,
    partial_images: &mut Vec<PendingGeneratedImage>,
    final_seen: &mut std::collections::HashSet<String>,
    partial_seen: &mut std::collections::HashSet<String>,
    text_parts: &mut Vec<String>,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            let event_type = object.get("type").and_then(Value::as_str).unwrap_or_default();
            if event_type == "output_text" {
                if let Some(text) = object.get("text").and_then(Value::as_str) {
                    let text = text.trim();
                    if !text.is_empty() && !text_parts.iter().any(|item| item == text) {
                        text_parts.push(text.to_string());
                    }
                }
            }
            let encoded = if event_type == "image_generation_call" {
                object.get("result").and_then(Value::as_str)
            } else if event_type == "response.image_generation_call.partial_image" {
                object.get("partial_image_b64").and_then(Value::as_str)
            } else {
                None
            };
            if let Some(encoded) = encoded.map(str::trim).filter(|value| !value.is_empty()) {
                let pending = PendingGeneratedImage {
                    source: PendingImageSource::Bytes(decode_generated_image_base64(encoded)?),
                    mime_hint: Some(
                        codex_image_output_mime(
                            object.get("output_format").and_then(Value::as_str),
                        )
                        .to_string(),
                    ),
                    remote_url: None,
                    revised_prompt: None,
                };
                if event_type == "image_generation_call" {
                    if final_seen.insert(encoded.to_string()) {
                        final_images.push(pending);
                    }
                } else if partial_seen.insert(encoded.to_string()) {
                    partial_images.push(pending);
                }
            }
            for child in object.values() {
                collect_codex_image_value(
                    child,
                    final_images,
                    partial_images,
                    final_seen,
                    partial_seen,
                    text_parts,
                )?;
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_codex_image_value(
                    item,
                    final_images,
                    partial_images,
                    final_seen,
                    partial_seen,
                    text_parts,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_codex_image_generation_response(data: &[u8]) -> Result<ProviderImageGenerationOutput, String> {
    let mut final_images = Vec::new();
    let mut partial_images = Vec::new();
    let mut final_seen = std::collections::HashSet::new();
    let mut partial_seen = std::collections::HashSet::new();
    let mut text_parts = Vec::new();
    let mut parsed_event = false;
    for line in data.split(|byte| *byte == b'\n') {
        let line = line.strip_prefix(b"data:").unwrap_or(line);
        let line = line.strip_prefix(b" ").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let line = line.trim_ascii();
        if line.is_empty() || line == b"[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        parsed_event = true;
        collect_codex_image_value(
            &value,
            &mut final_images,
            &mut partial_images,
            &mut final_seen,
            &mut partial_seen,
            &mut text_parts,
        )?;
    }
    if !parsed_event {
        if let Ok(value) = serde_json::from_slice::<Value>(data) {
            collect_codex_image_value(
                &value,
                &mut final_images,
                &mut partial_images,
                &mut final_seen,
                &mut partial_seen,
                &mut text_parts,
            )?;
        }
    }
    let images = if final_images.is_empty() {
        partial_images.into_iter().last().into_iter().collect()
    } else {
        final_images
    };
    if images.is_empty() {
        let text = text_parts.join("\n");
        return Err(if text.is_empty() {
            "Codex 未返回图片数据".to_string()
        } else {
            format!("Codex 未返回图片数据：{text}")
        });
    }
    Ok(ProviderImageGenerationOutput {
        images,
        text: (!text_parts.is_empty()).then(|| text_parts.join("\n")),
    })
}

async fn generate_codex_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    edit_inputs: &ImageEditInputs,
) -> Result<ProviderImageGenerationOutput, String> {
    if matches!(request.operation, ImageGenerationOperation::Edit) {
        ensure_image_edit_input_limits(
            &resolved.provider.name,
            edit_inputs,
            CODEX_IMAGE_EDIT_MAX_IMAGES,
            true,
        )?;
    }
    let auth = resolve_codex_image_auth(state, &resolved.provider).await?;
    let endpoint = append_image_generation_endpoint(&auth.base_url, "/responses");
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("text/event-stream"),
    );
    if let Some(account_id) = auth
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.insert(
            "ChatGPT-Account-Id",
            HeaderValue::from_str(account_id)
                .map_err(|err| format!("Codex 生图请求头构造失败：{err}"))?,
        );
    }
    let response = state
        .shared_http_client
        .post(endpoint)
        .headers(headers)
        .bearer_auth(auth.access_token)
        .json(&codex_image_generation_payload(request, &resolved.model, edit_inputs))
        .timeout(std::time::Duration::from_secs(u64::from(
            resolved.provider.timeout_seconds,
        )))
        .send()
        .await
        .map_err(|err| format!("{} 请求失败：{err}", resolved.provider.name))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| format!("读取 Codex 生图响应失败：{err}"))?;
    if !status.is_success() {
        let message = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .or_else(|| value.get("message").and_then(Value::as_str))
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| String::from_utf8_lossy(&body).trim().to_string());
        return Err(format!("Codex 生图请求失败（HTTP {}）：{}", status.as_u16(), message));
    }
    parse_codex_image_generation_response(&body)
}

#[cfg(test)]
mod codex_image_generation_tests {
    use super::*;

    fn test_request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "一只戴红围巾的猫".to_string(),
            ..ImageGenerationRequest::default()
        }
    }

    #[test]
    fn payload_should_force_codex_image_generation_tool() {
        let mut model = ImageGenerationModelConfig::default();
        model.model = "user-configured-model-should-be-ignored".to_string();
        let payload =
            codex_image_generation_payload(&test_request(), &model, &ImageEditInputs::default());

        assert_eq!(payload["model"], CODEX_IMAGE_MAIN_MODEL);
        assert_eq!(payload["tools"][0]["type"], "image_generation");
        assert_eq!(payload["tools"][0]["model"], CODEX_IMAGE_TOOL_MODEL);
        assert_eq!(payload["tools"][0]["output_format"], "png");
        assert_eq!(payload["tool_choice"]["type"], "image_generation");
        assert_eq!(payload["reasoning"]["effort"], "medium");
    }

    #[test]
    fn payload_should_attach_edit_images_and_mask() {
        let inputs = ImageEditInputs {
            images: vec![ImageEditInputImage {
                bytes: b"img".to_vec(),
                mime: "image/png".to_string(),
            }],
            mask: Some(ImageEditInputImage {
                bytes: b"mask".to_vec(),
                mime: "image/png".to_string(),
            }),
        };
        let payload = codex_image_generation_payload(
            &test_request(),
            &ImageGenerationModelConfig::default(),
            &inputs,
        );

        assert_eq!(payload["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(payload["input"][0]["content"][1]["type"], "input_image");
        assert!(payload["input"][0]["content"][1]["image_url"]
            .as_str()
            .unwrap_or_default()
            .starts_with("data:image/png;base64,"));
        assert!(payload["tools"][0]["input_image_mask"]["image_url"]
            .as_str()
            .unwrap_or_default()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn parser_should_prefer_final_result_over_partial_preview() {
        let response = concat!(
            "data: {\"type\":\"response.image_generation_call.partial_image\",\"output_format\":\"png\",\"partial_image_b64\":\"cHJldmlldw==\"}\n\n",
            "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"image_generation_call\",\"output_format\":\"png\",\"result\":\"ZmluYWw=\"}}\n\n",
            "data: [DONE]\n\n"
        );
        let output = parse_codex_image_generation_response(response.as_bytes())
            .expect("Codex image response should parse");

        assert_eq!(output.images.len(), 1);
        match &output.images[0].source {
            PendingImageSource::Bytes(bytes) => assert_eq!(bytes, b"final"),
            PendingImageSource::RemoteUrl(_) => panic!("expected inline image bytes"),
        }
    }

    #[test]
    fn parser_should_accept_partial_image_when_no_final_result_exists() {
        let response = b"data: {\"type\":\"response.image_generation_call.partial_image\",\"output_format\":\"png\",\"partial_image_b64\":\"cHJldmlldw==\"}\n\n";
        let output = parse_codex_image_generation_response(response)
            .expect("Codex partial image response should parse");

        assert_eq!(output.images.len(), 1);
        match &output.images[0].source {
            PendingImageSource::Bytes(bytes) => assert_eq!(bytes, b"preview"),
            PendingImageSource::RemoteUrl(_) => panic!("expected inline image bytes"),
        }
    }
}
