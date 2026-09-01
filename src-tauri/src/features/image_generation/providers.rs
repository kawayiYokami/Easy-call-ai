fn image_provider_key_cursor_state() -> &'static Mutex<std::collections::HashMap<String, usize>> {
    static CURSORS: OnceLock<Mutex<std::collections::HashMap<String, usize>>> = OnceLock::new();
    CURSORS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn select_image_generation_api_key(provider: &ImageGenerationProviderConfig) -> String {
    let keys = provider
        .api_keys
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return String::new();
    }
    let Ok(mut guard) = image_provider_key_cursor_state().lock() else {
        return keys[(provider.key_cursor as usize) % keys.len()].to_string();
    };
    let cursor = guard
        .entry(provider.id.clone())
        .or_insert((provider.key_cursor as usize) % keys.len());
    let selected = keys[*cursor % keys.len()].to_string();
    *cursor = (*cursor + 1) % keys.len();
    selected
}

fn trimmed_image_generation_option(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn effective_image_generation_size(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Option<String> {
    trimmed_image_generation_option(&request.size)
        .or_else(|| trimmed_image_generation_option(&model.default_size))
}

fn effective_image_generation_aspect_ratio(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Option<String> {
    trimmed_image_generation_option(&request.aspect_ratio)
        .or_else(|| trimmed_image_generation_option(&model.default_aspect_ratio))
}

fn effective_image_generation_quality(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Option<String> {
    trimmed_image_generation_option(&request.quality)
        .or_else(|| trimmed_image_generation_option(&model.default_quality))
}

fn effective_image_generation_prompt(request: &ImageGenerationRequest) -> String {
    let prompt = request.prompt.trim();
    let Some(negative_prompt) = request
        .negative_prompt
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return prompt.to_string();
    };
    format!("{prompt}\n\n请避免出现以下内容：{negative_prompt}")
}

fn parse_pixel_size(value: &str) -> Option<(u32, u32)> {
    let normalized = value.trim().to_ascii_lowercase().replace('×', "x");
    let (width, height) = normalized.split_once('x')?;
    let width = width.trim().parse::<u32>().ok()?;
    let height = height.trim().parse::<u32>().ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

fn greatest_common_divisor(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn aspect_ratio_from_dimensions(width: u32, height: u32) -> String {
    let divisor = greatest_common_divisor(width, height);
    format!("{}:{}", width / divisor, height / divisor)
}

fn parse_aspect_ratio(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.trim().split_once(':')?;
    let width = width.trim().parse::<u32>().ok()?;
    let height = height.trim().parse::<u32>().ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

fn openai_size_from_aspect_ratio(value: &str, arbitrary_size: bool) -> Option<String> {
    let (width, height) = parse_aspect_ratio(value)?;
    let ratio = f64::from(width) / f64::from(height);
    if arbitrary_size {
        if !(1.0 / 3.0..=3.0).contains(&ratio) {
            return None;
        }
        if (0.87..=1.15).contains(&ratio) {
            return Some("1024x1024".to_string());
        }
        let rounded_to_16 = |value: f64| -> u32 {
            (((value / 16.0).round() as u32).max(1) * 16).clamp(512, 1536)
        };
        return if ratio > 1.0 {
            Some(format!("1536x{}", rounded_to_16(1536.0 / ratio)))
        } else {
            Some(format!("{}x1536", rounded_to_16(1536.0 * ratio)))
        };
    }
    Some(if ratio > 1.15 {
        "1536x1024".to_string()
    } else if ratio < 0.87 {
        "1024x1536".to_string()
    } else {
        "1024x1024".to_string()
    })
}

fn append_image_generation_endpoint(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let suffix = suffix.trim();
    let normalized_suffix = if suffix.starts_with('/') {
        suffix.to_string()
    } else {
        format!("/{suffix}")
    };
    if base
        .to_ascii_lowercase()
        .ends_with(&normalized_suffix.to_ascii_lowercase())
    {
        base.to_string()
    } else {
        format!("{base}{normalized_suffix}")
    }
}

fn openai_image_generation_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Value {
    let mut payload = serde_json::json!({
        "model": model.model,
        "prompt": effective_image_generation_prompt(request),
        "n": 1
    });
    let is_gpt_image = model.model.trim().to_ascii_lowercase().starts_with("gpt-image");
    let supports_arbitrary_size = model
        .model
        .trim()
        .to_ascii_lowercase()
        .starts_with("gpt-image-2");
    if let Some(object) = payload.as_object_mut() {
        if is_gpt_image {
            object.insert("output_format".to_string(), Value::String("png".to_string()));
        } else {
            object.insert(
                "response_format".to_string(),
                Value::String("b64_json".to_string()),
            );
        }
        let size = trimmed_image_generation_option(&request.size)
            .or_else(|| {
                trimmed_image_generation_option(&request.aspect_ratio)
                    .as_deref()
                    .and_then(|value| openai_size_from_aspect_ratio(value, supports_arbitrary_size))
            })
            .or_else(|| trimmed_image_generation_option(&model.default_size))
            .or_else(|| {
                trimmed_image_generation_option(&model.default_aspect_ratio)
                    .as_deref()
                    .and_then(|value| openai_size_from_aspect_ratio(value, supports_arbitrary_size))
            });
        if let Some(size) = size {
            object.insert("size".to_string(), Value::String(size));
        }
        if let Some(quality) = effective_image_generation_quality(request, model) {
            object.insert("quality".to_string(), Value::String(quality));
        }
    }
    payload
}

fn xai_resolution_from_request(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Option<String> {
    for candidate in [
        effective_image_generation_size(request, model),
        effective_image_generation_quality(request, model),
    ]
    .into_iter()
    .flatten()
    {
        let normalized = candidate.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "1k" | "2k") {
            return Some(normalized);
        }
        if let Some((width, height)) = parse_pixel_size(&normalized) {
            return Some(if width.max(height) > 1536 { "2k" } else { "1k" }.to_string());
        }
    }
    None
}

fn xai_image_generation_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Value {
    let mut payload = serde_json::json!({
        "model": model.model,
        "prompt": effective_image_generation_prompt(request),
        "n": 1,
        "response_format": "b64_json"
    });
    if let Some(object) = payload.as_object_mut() {
        let aspect_ratio = effective_image_generation_aspect_ratio(request, model).or_else(|| {
            effective_image_generation_size(request, model)
                .as_deref()
                .and_then(parse_pixel_size)
                .map(|(width, height)| aspect_ratio_from_dimensions(width, height))
        });
        if let Some(aspect_ratio) = aspect_ratio {
            object.insert("aspect_ratio".to_string(), Value::String(aspect_ratio));
        }
        if let Some(resolution) = xai_resolution_from_request(request, model) {
            object.insert("resolution".to_string(), Value::String(resolution));
        }
    }
    payload
}

fn seedream_image_generation_payload(
    request: &ImageGenerationRequest,
    provider: &ImageGenerationProviderConfig,
    model: &ImageGenerationModelConfig,
) -> Value {
    let model_name = model.model.trim().to_ascii_lowercase();
    let aspect_ratio = effective_image_generation_aspect_ratio(request, model);
    let size = trimmed_image_generation_option(&request.size).or_else(|| {
        let default_size = trimmed_image_generation_option(&model.default_size)?;
        if aspect_ratio.is_none() || parse_pixel_size(&default_size).is_none() {
            return Some(default_size);
        }
        if model_name.contains("seedream-5-0-pro") {
            return Some("2K".to_string());
        }
        let longest_edge = parse_pixel_size(&default_size)
            .map(|(width, height)| width.max(height))
            .unwrap_or(2048);
        Some(if longest_edge > 3072 {
            "4K"
        } else if longest_edge > 2048 {
            "3K"
        } else {
            "2K"
        }
        .to_string())
    });
    let mut prompt = effective_image_generation_prompt(request);
    if size.as_deref().and_then(parse_pixel_size).is_none() {
        if let Some(aspect_ratio) = aspect_ratio {
            prompt.push_str(&format!("\n\n画面宽高比：{aspect_ratio}"));
        }
    }
    let mut payload = serde_json::json!({
        "model": model.model,
        "prompt": prompt,
        "response_format": "b64_json",
        "watermark": provider.watermark
    });
    if let Some(object) = payload.as_object_mut() {
        if model_name.contains("seedream-5-0") {
            object.insert(
                "output_format".to_string(),
                Value::String("png".to_string()),
            );
        }
        if let Some(size) = size {
            object.insert("size".to_string(), Value::String(size));
        }
        if let Some(quality) = effective_image_generation_quality(request, model) {
            let mode = quality.trim().to_ascii_lowercase();
            let supports_prompt_optimization = model_name.contains("seedream-5-0")
                || model_name.contains("seedream-4-5")
                || model_name.contains("seedream-4-0");
            let supports_fast = model_name.contains("seedream-5-0-pro")
                || model_name.contains("seedream-4-0");
            let supports_mode = supports_prompt_optimization
                && (mode == "standard" || (mode == "fast" && supports_fast));
            if supports_mode {
                object.insert(
                    "optimize_prompt_options".to_string(),
                    serde_json::json!({ "mode": mode }),
                );
            }
        }
    }
    payload
}

fn gemini_image_generation_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Value {
    let mut response_format = serde_json::Map::<String, Value>::new();
    response_format.insert("type".to_string(), Value::String("image".to_string()));
    response_format.insert(
        "mime_type".to_string(),
        Value::String("image/png".to_string()),
    );
    let size = effective_image_generation_size(request, model);
    let aspect_ratio = effective_image_generation_aspect_ratio(request, model).or_else(|| {
        size.as_deref()
            .and_then(parse_pixel_size)
            .map(|(width, height)| aspect_ratio_from_dimensions(width, height))
    });
    if let Some(aspect_ratio) = aspect_ratio {
        response_format.insert("aspect_ratio".to_string(), Value::String(aspect_ratio));
    }
    if let Some(size) = size {
        let normalized = size.trim().to_ascii_uppercase();
        if matches!(normalized.as_str(), "512" | "1K" | "2K" | "4K") {
            response_format.insert("image_size".to_string(), Value::String(normalized));
        }
    }
    serde_json::json!({
        "model": model.model,
        "input": [{
            "type": "text",
            "text": effective_image_generation_prompt(request)
        }],
        "response_format": Value::Object(response_format)
    })
}

fn image_generation_value_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| value.get(*key).and_then(Value::as_str))
}

fn parse_openai_style_image_response(value: &Value) -> Result<ProviderImageGenerationOutput, String> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "供应商响应缺少 data 图片数组".to_string())?;
    let mut images = Vec::<PendingGeneratedImage>::new();
    let mut item_errors = Vec::<String>::new();
    for item in data {
        if let Some(error) = item.get("error") {
            let message = image_generation_value_string(error, &["message", "code"])
                .unwrap_or("图片生成失败")
                .to_string();
            item_errors.push(message);
            continue;
        }
        let revised_prompt = image_generation_value_string(item, &["revised_prompt", "revisedPrompt"])
            .map(ToOwned::to_owned);
        if let Some(encoded) = image_generation_value_string(item, &["b64_json", "b64Json"]) {
            images.push(PendingGeneratedImage {
                source: PendingImageSource::Bytes(decode_generated_image_base64(encoded)?),
                mime_hint: None,
                remote_url: None,
                revised_prompt,
            });
            continue;
        }
        if let Some(url) = item.get("url").and_then(Value::as_str) {
            let url = url.trim().to_string();
            if !url.is_empty() {
                images.push(PendingGeneratedImage {
                    source: PendingImageSource::RemoteUrl(url.clone()),
                    mime_hint: None,
                    remote_url: Some(url),
                    revised_prompt,
                });
            }
        }
    }
    if images.is_empty() {
        return Err(if item_errors.is_empty() {
            "供应商响应中没有可用图片".to_string()
        } else {
            format!("供应商未返回可用图片：{}", item_errors.join("；"))
        });
    }
    Ok(ProviderImageGenerationOutput { images, text: None })
}

fn parse_gemini_image_response(value: &Value) -> Result<ProviderImageGenerationOutput, String> {
    let mut images = Vec::<PendingGeneratedImage>::new();
    let mut text_parts = Vec::<String>::new();
    if let Some(steps) = value.get("steps").and_then(Value::as_array) {
        for step in steps {
            let Some(contents) = step.get("content").and_then(Value::as_array) else {
                continue;
            };
            for content in contents {
                if let Some(text) = content.get("text").and_then(Value::as_str) {
                    let text = text.trim();
                    if !text.is_empty() {
                        text_parts.push(text.to_string());
                    }
                }
                if content.get("type").and_then(Value::as_str) != Some("image") {
                    continue;
                }
                let Some(encoded) = content.get("data").and_then(Value::as_str) else {
                    continue;
                };
                let mime_hint = image_generation_value_string(content, &["mime_type", "mimeType"])
                    .map(ToOwned::to_owned);
                images.push(PendingGeneratedImage {
                    source: PendingImageSource::Bytes(decode_generated_image_base64(encoded)?),
                    mime_hint,
                    remote_url: None,
                    revised_prompt: None,
                });
            }
        }
        if !images.is_empty() {
            return Ok(ProviderImageGenerationOutput {
                images,
                text: (!text_parts.is_empty()).then(|| text_parts.join("\n")),
            });
        }
    }

    let candidates = value
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "Gemini 响应缺少 steps 或 candidates".to_string())?;
    for candidate in candidates {
        let Some(parts) = candidate
            .get("content")
            .and_then(|content| content.get("parts"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                let text = text.trim();
                if !text.is_empty() {
                    text_parts.push(text.to_string());
                }
            }
            let inline_data = part.get("inlineData").or_else(|| part.get("inline_data"));
            let Some(inline_data) = inline_data else {
                continue;
            };
            let Some(encoded) = inline_data.get("data").and_then(Value::as_str) else {
                continue;
            };
            let mime_hint = image_generation_value_string(inline_data, &["mimeType", "mime_type"])
                .map(ToOwned::to_owned);
            images.push(PendingGeneratedImage {
                source: PendingImageSource::Bytes(decode_generated_image_base64(encoded)?),
                mime_hint,
                remote_url: None,
                revised_prompt: None,
            });
        }
    }
    if images.is_empty() {
        let provider_text = text_parts.join("\n");
        return Err(if provider_text.is_empty() {
            "Gemini 未返回图片数据".to_string()
        } else {
            format!("Gemini 未返回图片数据：{provider_text}")
        });
    }
    Ok(ProviderImageGenerationOutput {
        images,
        text: (!text_parts.is_empty()).then(|| text_parts.join("\n")),
    })
}

async fn post_bearer_image_generation_json(
    state: &AppState,
    provider: &ImageGenerationProviderConfig,
    api_key: &str,
    endpoint: &str,
    payload: &Value,
) -> Result<Value, String> {
    let response = state
        .shared_http_client
        .post(endpoint)
        .bearer_auth(api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(payload)
        .timeout(std::time::Duration::from_secs(u64::from(provider.timeout_seconds)))
        .send()
        .await
        .map_err(|err| format!("{} 请求失败：{err}", provider.name))?;
    parse_image_generation_json_response(response, &provider.name).await
}

async fn generate_openai_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/generations");
    let payload = openai_image_generation_payload(request, &resolved.model);
    let value = post_bearer_image_generation_json(
        state,
        &resolved.provider,
        api_key,
        &endpoint,
        &payload,
    )
    .await?;
    parse_openai_style_image_response(&value)
}

async fn generate_xai_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/generations");
    let payload = xai_image_generation_payload(request, &resolved.model);
    let value = post_bearer_image_generation_json(
        state,
        &resolved.provider,
        api_key,
        &endpoint,
        &payload,
    )
    .await?;
    parse_openai_style_image_response(&value)
}

async fn generate_seedream_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/generations");
    let payload = seedream_image_generation_payload(request, &resolved.provider, &resolved.model);
    let value = post_bearer_image_generation_json(
        state,
        &resolved.provider,
        api_key,
        &endpoint,
        &payload,
    )
    .await?;
    parse_openai_style_image_response(&value)
}

async fn post_gemini_image_interactions(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    api_key: &str,
    payload: &Value,
) -> Result<Value, String> {
    let endpoint = append_image_generation_endpoint(
        &resolved.provider.base_url,
        "/interactions",
    );
    let response = state
        .shared_http_client
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .header("Api-Revision", "2026-05-20")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(payload)
        .timeout(std::time::Duration::from_secs(u64::from(
            resolved.provider.timeout_seconds,
        )))
        .send()
        .await
        .map_err(|err| format!("{} 请求失败：{err}", resolved.provider.name))?;
    parse_image_generation_json_response(response, &resolved.provider.name).await
}

async fn generate_gemini_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    let payload = gemini_image_generation_payload(request, &resolved.model);
    let value = post_gemini_image_interactions(state, resolved, api_key, &payload).await?;
    parse_gemini_image_response(&value)
}

async fn generate_sensenova_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    // SenseNova 生图完全兼容 OpenAI images/generations 协议，直接复用
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/generations");
    let payload = openai_image_generation_payload(request, &resolved.model);
    let value = post_bearer_image_generation_json(
        state,
        &resolved.provider,
        api_key,
        &endpoint,
        &payload,
    )
    .await?;
    parse_openai_style_image_response(&value)
}

#[cfg(test)]
mod image_generation_provider_tests {
    use super::*;

    fn request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "一只猫".to_string(),
            aspect_ratio: Some("16:9".to_string()),
            ..ImageGenerationRequest::default()
        }
    }

    #[test]
    fn openai_payload_should_use_current_gpt_image_fields() {
        let payload = openai_image_generation_payload(
            &request(),
            &ImageGenerationModelConfig::default(),
        );
        assert_eq!(payload.get("model").and_then(Value::as_str), Some("gpt-image-2"));
        assert_eq!(payload.get("output_format").and_then(Value::as_str), Some("png"));
        assert_eq!(payload.get("size").and_then(Value::as_str), Some("1536x864"));
    }

    #[test]
    fn openai_payload_should_prefer_explicit_size_over_aspect_ratio() {
        let mut request = request();
        request.size = Some("1024x1536".to_string());
        let payload = openai_image_generation_payload(
            &request,
            &ImageGenerationModelConfig::default(),
        );
        assert_eq!(payload.get("size").and_then(Value::as_str), Some("1024x1536"));
    }

    #[test]
    fn xai_payload_should_keep_aspect_ratio_and_resolution() {
        let mut model = ImageGenerationModelConfig::default();
        model.model = "grok-imagine-image-quality".to_string();
        model.default_size = Some("2k".to_string());
        let payload = xai_image_generation_payload(&request(), &model);
        assert_eq!(payload.get("aspect_ratio").and_then(Value::as_str), Some("16:9"));
        assert_eq!(payload.get("resolution").and_then(Value::as_str), Some("2k"));
    }

    #[test]
    fn seedream_pro_payload_should_use_current_output_format() {
        let provider = ImageGenerationProviderConfig::default();
        let mut model = ImageGenerationModelConfig::default();
        model.model = "doubao-seedream-5-0-pro-260628".to_string();
        let payload = seedream_image_generation_payload(&request(), &provider, &model);
        assert_eq!(payload.get("output_format").and_then(Value::as_str), Some("png"));
        assert_eq!(payload.get("size").and_then(Value::as_str), Some("2K"));
        assert!(payload
            .get("prompt")
            .and_then(Value::as_str)
            .is_some_and(|value| value.contains("画面宽高比：16:9")));
        assert_eq!(
            payload.get("response_format").and_then(Value::as_str),
            Some("b64_json")
        );
    }

    #[test]
    fn seedream_payload_should_only_send_fast_prompt_optimization_for_supported_models() {
        let provider = ImageGenerationProviderConfig::default();
        let mut fast_request = request();
        fast_request.quality = Some("fast".to_string());

        for model_name in [
            "doubao-seedream-5-0-lite-260128",
            "doubao-seedream-5-0-260128",
            "doubao-seedream-4-5-251128",
        ] {
            let mut model = ImageGenerationModelConfig::default();
            model.model = model_name.to_string();
            let payload = seedream_image_generation_payload(&fast_request, &provider, &model);
            assert!(payload.get("optimize_prompt_options").is_none());
        }

        let mut pro_model = ImageGenerationModelConfig::default();
        pro_model.model = "doubao-seedream-5-0-pro-260628".to_string();
        let pro_payload =
            seedream_image_generation_payload(&fast_request, &provider, &pro_model);
        assert_eq!(
            pro_payload
                .get("optimize_prompt_options")
                .and_then(|value| value.get("mode"))
                .and_then(Value::as_str),
            Some("fast")
        );

        let mut legacy_model = ImageGenerationModelConfig::default();
        legacy_model.model = "doubao-seedream-4-0-250828".to_string();
        let legacy_payload =
            seedream_image_generation_payload(&fast_request, &provider, &legacy_model);
        assert_eq!(
            legacy_payload
                .get("optimize_prompt_options")
                .and_then(|value| value.get("mode"))
                .and_then(Value::as_str),
            Some("fast")
        );

        let mut standard_request = request();
        standard_request.quality = Some("standard".to_string());
        let mut lite_model = ImageGenerationModelConfig::default();
        lite_model.model = "doubao-seedream-5-0-lite-260128".to_string();
        let lite_payload =
            seedream_image_generation_payload(&standard_request, &provider, &lite_model);
        assert_eq!(
            lite_payload
                .get("optimize_prompt_options")
                .and_then(|value| value.get("mode"))
                .and_then(Value::as_str),
            Some("standard")
        );
    }

    #[test]
    fn gemini_payload_should_use_current_interactions_schema() {
        let mut model = ImageGenerationModelConfig::default();
        model.model = "gemini-3.1-flash-image".to_string();
        model.default_size = Some("2K".to_string());
        let payload = gemini_image_generation_payload(&request(), &model);
        assert_eq!(
            payload.get("model").and_then(Value::as_str),
            Some("gemini-3.1-flash-image")
        );
        assert_eq!(
            payload
                .get("response_format")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str),
            Some("image")
        );
        assert_eq!(
            payload
                .get("response_format")
                .and_then(|value| value.get("aspect_ratio"))
                .and_then(Value::as_str),
            Some("16:9")
        );
        assert_eq!(
            payload
                .get("response_format")
                .and_then(|value| value.get("image_size"))
                .and_then(Value::as_str),
            Some("2K")
        );
    }

    #[test]
    fn gemini_parser_should_accept_interactions_steps() {
        let value = serde_json::json!({
            "status": "completed",
            "steps": [{
                "type": "model_output",
                "content": [{
                    "type": "image",
                    "mime_type": "image/png",
                    "data": "aGVsbG8="
                }]
            }]
        });
        let parsed = parse_gemini_image_response(&value).unwrap_or_default();
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].mime_hint.as_deref(), Some("image/png"));
    }

    #[test]
    fn gemini_parser_should_accept_legacy_snake_case_inline_data() {
        let value = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{
                    "inline_data": {
                        "mime_type": "image/png",
                        "data": "aGVsbG8="
                    }
                }]}
            }]
        });
        let parsed = parse_gemini_image_response(&value).unwrap_or_default();
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].mime_hint.as_deref(), Some("image/png"));
    }

    #[test]
    fn sensenova_payload_should_reuse_openai_protocol() {
        let payload = openai_image_generation_payload(
            &request(),
            &ImageGenerationModelConfig {
                model: "sensenova-u1-fast".to_string(),
                ..ImageGenerationModelConfig::default()
            },
        );
        assert_eq!(payload.get("model").and_then(Value::as_str), Some("sensenova-u1-fast"));
        assert_eq!(payload.get("size").and_then(Value::as_str), Some("1536x1024"));
    }
}
