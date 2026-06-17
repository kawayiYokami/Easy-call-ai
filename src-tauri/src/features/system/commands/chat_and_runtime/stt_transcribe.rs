fn candidate_stt_urls(base_url: &str) -> Vec<String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Vec::new();
    }
    let lower = base.to_ascii_lowercase();
    let mut urls = Vec::new();
    if lower.ends_with("/audio/transcriptions") {
        urls.push(base.to_string());
    } else if lower.ends_with("/v1") {
        urls.push(format!("{base}/audio/transcriptions"));
    } else {
        urls.push(format!("{base}/audio/transcriptions"));
        urls.push(format!("{base}/v1/audio/transcriptions"));
    }
    urls.sort();
    urls.dedup();
    urls
}

fn candidate_mimo_asr_urls(base_url: &str) -> Vec<String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Vec::new();
    }
    let lower = base.to_ascii_lowercase();
    let mut urls = Vec::new();
    if lower.ends_with("/chat/completions") {
        urls.push(base.to_string());
    } else if lower.ends_with("/v1") {
        urls.push(format!("{base}/chat/completions"));
    } else {
        urls.push(format!("{base}/chat/completions"));
        urls.push(format!("{base}/v1/chat/completions"));
    }
    urls.sort();
    urls.dedup();
    urls
}

async fn call_openai_stt_transcribe(
    api_config: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    mime: &str,
    audio_raw: Vec<u8>,
) -> Result<String, String> {
    let model = api_config.model.trim();
    if model.is_empty() {
        return Err("STT model is empty.".to_string());
    }
    let request_api_key = consume_api_key_for_request(resolved_api);
    if request_api_key.trim().is_empty() {
        return Err("STT API key is empty.".to_string());
    }
    let urls = candidate_stt_urls(&api_config.base_url);
    if urls.is_empty() {
        return Err("STT base URL is empty.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| format!("Build STT HTTP client failed: {err}"))?;

    let mut errors = Vec::new();
    for url in urls {
        let file_part = reqwest::multipart::Part::bytes(audio_raw.clone())
            .file_name("speech.webm")
            .mime_str(if mime.trim().is_empty() {
                "audio/webm"
            } else {
                mime.trim()
            })
            .map_err(|err| format!("Build STT mime part failed: {err}"))?;
        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", model.to_string());
        let resp = client
            .post(&url)
            .bearer_auth(request_api_key.trim())
            .multipart(form)
            .send()
            .await;
        let Ok(resp) = resp else {
            errors.push(format!("{url} -> request failed"));
            continue;
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();
            errors.push(format!(
                "{url} -> {status}: {}",
                raw.chars().take(220).collect::<String>()
            ));
            continue;
        }
        let body = resp
            .json::<Value>()
            .await
            .map_err(|err| format!("Parse STT response failed: {err}"))?;
        if let Some(text) = body.get("text").and_then(Value::as_str) {
            return Ok(text.trim().to_string());
        }
        if let Some(text) = body.get("transcript").and_then(Value::as_str) {
            return Ok(text.trim().to_string());
        }
        return Err(format!(
            "STT response does not contain text field: {}",
            body.to_string().chars().take(220).collect::<String>()
        ));
    }

    Err(format!(
        "STT request failed for all candidate URLs: {}",
        errors.join(" || ")
    ))
}

async fn call_mimo_asr_transcribe(
    api_config: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    mime: &str,
    audio_raw: Vec<u8>,
) -> Result<String, String> {
    let model = api_config.model.trim();
    if model.is_empty() {
        return Err("MiMo ASR model is empty.".to_string());
    }
    let request_api_key = consume_api_key_for_request(resolved_api);
    if request_api_key.trim().is_empty() {
        return Err("MiMo ASR API key is empty.".to_string());
    }
    let urls = candidate_mimo_asr_urls(&api_config.base_url);
    if urls.is_empty() {
        return Err("MiMo ASR base URL is empty.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| format!("Build MiMo ASR HTTP client failed: {err}"))?;

    let audio_base64 = B64.encode(audio_raw);
    let audio_mime = if mime.trim().is_empty() {
        "audio/webm"
    } else {
        mime.trim()
    };
    let audio_data_url = format!("data:{audio_mime};base64,{audio_base64}");
    let audio_format = mime
        .trim()
        .strip_prefix("audio/")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("webm");

    let mut errors = Vec::new();
    for url in urls {
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_audio",
                            "input_audio": {
                                "data": audio_data_url,
                                "format": audio_format
                            }
                        }
                    ]
                }
            ],
            "asr_options": {
                "language": "auto"
            }
        });
        let resp = client
            .post(&url)
            .bearer_auth(request_api_key.trim())
            .json(&body)
            .send()
            .await;
        let Ok(resp) = resp else {
            errors.push(format!("{url} -> request failed"));
            continue;
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();
            errors.push(format!(
                "{url} -> {status}: {}",
                raw.chars().take(220).collect::<String>()
            ));
            continue;
        }
        let body = resp
            .json::<Value>()
            .await
            .map_err(|err| format!("Parse MiMo ASR response failed: {err}"))?;
        if let Some(text) = body
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
        {
            return Ok(text.trim().to_string());
        }
        if let Some(text) = body.get("text").and_then(Value::as_str) {
            return Ok(text.trim().to_string());
        }
        if let Some(text) = body.get("transcript").and_then(Value::as_str) {
            return Ok(text.trim().to_string());
        }
        return Err(format!(
            "MiMo ASR response does not contain transcript text: {}",
            body.to_string().chars().take(220).collect::<String>()
        ));
    }

    Err(format!(
        "MiMo ASR request failed for all candidate URLs: {}",
        errors.join(" || ")
    ))
}

#[tauri::command]
async fn stt_transcribe(
    input: SttTranscribeInput,
    state: State<'_, AppState>,
) -> Result<SttTranscribeOutput, String> {
    if input.bytes_base64.trim().is_empty() {
        return Err("Audio payload is empty.".to_string());
    }

    let app_config = {
        let cfg = read_config(&state.config_path)?;
        cfg
    };

    let selected_id = input
        .stt_api_config_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or(app_config.stt_api_config_id.as_deref())
        .ok_or_else(|| "No STT API selected. Using local transcription only.".to_string())?;
    let api = app_config
        .api_configs
        .iter()
        .find(|a| a.id == selected_id)
        .cloned()
        .ok_or_else(|| "Selected STT API config not found.".to_string())?;
    if !api.request_format.is_stt() {
        return Err(
            "Selected STT API must use request_format='openai_stt' or 'mimo_asr'."
                .to_string(),
        );
    }
    let resolved = resolve_api_config(&app_config, Some(api.id.as_str()))?;

    let audio_raw = B64
        .decode(input.bytes_base64.trim())
        .map_err(|err| format!("Decode audio base64 failed: {err}"))?;
    let text_result = match api.request_format {
        RequestFormat::OpenAIStt => {
            call_openai_stt_transcribe(&api, &resolved, &input.mime, audio_raw).await
        }
        RequestFormat::MimoAsr => {
            call_mimo_asr_transcribe(&api, &resolved, &input.mime, audio_raw).await
        }
        _ => {
            return Err(
                "Selected STT API must use request_format='openai_stt' or 'mimo_asr'."
                    .to_string(),
            );
        }
    };
    let text = text_result?;
    Ok(SttTranscribeOutput { text })
}


