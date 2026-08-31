// ==================== 图像编辑输入与云端供应商适配 ====================
// 输入图片引用支持 {Assistant Space} 相对路径、绝对路径与 data URL；
// 各供应商编辑能力差异（张数上限、mask 支持）在此统一显式表达，不静默降级。

const IMAGE_EDIT_MAX_INPUT_IMAGES: usize = 16;
const OPENAI_IMAGE_EDIT_MAX_IMAGES: usize = 16;
const XAI_IMAGE_EDIT_MAX_IMAGES: usize = 3;
const SEEDREAM_IMAGE_EDIT_MAX_IMAGES: usize = 10;
const GEMINI_IMAGE_EDIT_MAX_IMAGES: usize = 14;
const CODEX_IMAGE_EDIT_MAX_IMAGES: usize = 16;
// xAI 编辑接口对参考图有严格限制，超限直接返回 400，需要预压缩。
const XAI_IMAGE_EDIT_MAX_REF_BYTES: usize = 400 * 1024;
const XAI_IMAGE_EDIT_MAX_REF_EDGE: u32 = 768;
const XAI_IMAGE_EDIT_JPEG_QUALITIES: [u8; 4] = [80, 65, 50, 35];

fn image_edit_file_extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "png",
    }
}

fn image_edit_data_url(input: &ImageEditInputImage) -> String {
    format!("data:{};base64,{}", input.mime, B64.encode(&input.bytes))
}

async fn load_image_edit_reference(
    state: &AppState,
    reference: &str,
    field_name: &str,
) -> Result<ImageEditInputImage, String> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name}引用为空"));
    }
    let bytes = if trimmed.starts_with("data:") {
        decode_generated_image_base64(trimmed)?
    } else {
        let path = resolve_local_chat_image_path(state, trimmed)?;
        if !path.is_absolute() {
            return Err(format!(
                "{field_name}路径必须是 {{Assistant Space}} 相对路径、绝对路径或 data URL：{trimmed}"
            ));
        }
        tokio::fs::read(&path)
            .await
            .map_err(|err| format!("读取{field_name}失败：{}（{err}）", path.display()))?
    };
    let (_format, mime, _extension, _width, _height) = generated_image_format_info(&bytes)
        .map_err(|err| format!("{field_name}不是有效图片：{err}"))?;
    Ok(ImageEditInputImage {
        bytes,
        mime: mime.to_string(),
    })
}

async fn load_image_edit_inputs(
    state: &AppState,
    request: &ImageGenerationRequest,
) -> Result<ImageEditInputs, String> {
    if !matches!(request.operation, ImageGenerationOperation::Edit) {
        return Ok(ImageEditInputs::default());
    }
    if request.images.is_empty() {
        return Err("图像编辑至少需要一张输入图片".to_string());
    }
    if request.images.len() > IMAGE_EDIT_MAX_INPUT_IMAGES {
        return Err(format!(
            "图像编辑最多支持 {IMAGE_EDIT_MAX_INPUT_IMAGES} 张输入图片，当前 {} 张",
            request.images.len()
        ));
    }
    let mut images = Vec::with_capacity(request.images.len());
    for (index, reference) in request.images.iter().enumerate() {
        let field_name = format!("输入图片 #{}", index + 1);
        images.push(load_image_edit_reference(state, reference, &field_name).await?);
    }
    let mask = match request.mask.as_deref() {
        Some(reference) => Some(load_image_edit_reference(state, reference, "mask 图片").await?),
        None => None,
    };
    Ok(ImageEditInputs { images, mask })
}

fn ensure_image_edit_input_limits(
    provider_label: &str,
    inputs: &ImageEditInputs,
    max_images: usize,
    supports_mask: bool,
) -> Result<(), String> {
    if inputs.images.is_empty() {
        return Err("图像编辑至少需要一张输入图片".to_string());
    }
    if inputs.images.len() > max_images {
        return Err(format!(
            "{provider_label} 图像编辑最多支持 {max_images} 张输入图片，当前 {} 张",
            inputs.images.len()
        ));
    }
    if !supports_mask && inputs.mask.is_some() {
        return Err(format!(
            "{provider_label} 图像编辑暂不支持 mask，请改用文字描述需要修改的区域"
        ));
    }
    Ok(())
}

// 编辑路径的尺寸只采用请求显式值，不落模型默认值，避免破坏原图比例。
fn image_edit_explicit_aspect_ratio(request: &ImageGenerationRequest) -> Option<String> {
    trimmed_image_generation_option(&request.aspect_ratio).or_else(|| {
        trimmed_image_generation_option(&request.size)
            .as_deref()
            .and_then(parse_pixel_size)
            .map(|(width, height)| aspect_ratio_from_dimensions(width, height))
    })
}

// ==================== OpenAI /images/edits（multipart） ====================

fn openai_image_edit_scalar_fields(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Vec<(String, String)> {
    let mut fields = vec![
        ("model".to_string(), model.model.clone()),
        ("prompt".to_string(), effective_image_generation_prompt(request)),
        ("n".to_string(), "1".to_string()),
    ];
    let model_name = model.model.trim().to_ascii_lowercase();
    let is_gpt_image = model_name.starts_with("gpt-image");
    let supports_arbitrary_size = model_name.starts_with("gpt-image-2");
    if is_gpt_image {
        fields.push(("output_format".to_string(), "png".to_string()));
    } else {
        fields.push(("response_format".to_string(), "b64_json".to_string()));
    }
    let size = trimmed_image_generation_option(&request.size).or_else(|| {
        trimmed_image_generation_option(&request.aspect_ratio)
            .as_deref()
            .and_then(|value| openai_size_from_aspect_ratio(value, supports_arbitrary_size))
    });
    if let Some(size) = size {
        fields.push(("size".to_string(), size));
    }
    if let Some(quality) = effective_image_generation_quality(request, model) {
        fields.push(("quality".to_string(), quality));
    }
    fields
}

fn image_edit_multipart_part(
    input: &ImageEditInputImage,
    file_stem: &str,
) -> Result<reqwest::multipart::Part, String> {
    reqwest::multipart::Part::bytes(input.bytes.clone())
        .file_name(format!("{file_stem}.{}", image_edit_file_extension(&input.mime)))
        .mime_str(&input.mime)
        .map_err(|err| format!("构造图片上传分段失败：{err}"))
}

async fn edit_openai_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        OPENAI_IMAGE_EDIT_MAX_IMAGES,
        true,
    )?;
    let mut form = reqwest::multipart::Form::new();
    for (name, value) in openai_image_edit_scalar_fields(request, &resolved.model) {
        form = form.text(name, value);
    }
    for (index, input) in inputs.images.iter().enumerate() {
        form = form.part("image[]", image_edit_multipart_part(input, &format!("image-{index}"))?);
    }
    if let Some(mask) = &inputs.mask {
        form = form.part("mask", image_edit_multipart_part(mask, "mask")?);
    }
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/edits");
    let response = state
        .shared_http_client
        .post(endpoint)
        .bearer_auth(api_key)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(u64::from(
            resolved.provider.timeout_seconds,
        )))
        .send()
        .await
        .map_err(|err| format!("{} 请求失败：{err}", resolved.provider.name))?;
    let value = parse_image_generation_json_response(response, &resolved.provider.name).await?;
    parse_openai_style_image_response(&value)
}

// ==================== xAI /images/edits（JSON + data URL） ====================

fn compress_image_for_xai_edit(input: &ImageEditInputImage) -> Result<ImageEditInputImage, String> {
    if input.bytes.len() <= XAI_IMAGE_EDIT_MAX_REF_BYTES
        && matches!(input.mime.as_str(), "image/jpeg" | "image/png")
    {
        return Ok(input.clone());
    }
    let decoded = image::load_from_memory(&input.bytes)
        .map_err(|err| format!("解码输入图片失败：{err}"))?;
    let resized = if decoded.width().max(decoded.height()) > XAI_IMAGE_EDIT_MAX_REF_EDGE {
        decoded.resize(
            XAI_IMAGE_EDIT_MAX_REF_EDGE,
            XAI_IMAGE_EDIT_MAX_REF_EDGE,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        decoded
    };
    let rgb = resized.to_rgb8();
    for quality in XAI_IMAGE_EDIT_JPEG_QUALITIES {
        let mut buffer = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        if rgb.write_with_encoder(encoder).is_err() {
            continue;
        }
        if buffer.len() <= XAI_IMAGE_EDIT_MAX_REF_BYTES {
            return Ok(ImageEditInputImage {
                bytes: buffer,
                mime: "image/jpeg".to_string(),
            });
        }
    }
    Err("输入图片过大，压缩后仍超过 xAI 图像编辑接口限制".to_string())
}

fn xai_image_edit_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
    image_data_urls: &[String],
) -> Value {
    let mut payload = serde_json::json!({
        "model": model.model,
        "prompt": effective_image_generation_prompt(request),
        "n": 1,
        "response_format": "b64_json"
    });
    if let Some(object) = payload.as_object_mut() {
        if let [single] = image_data_urls {
            // 单图编辑输出跟随原图比例，官方会忽略 aspect_ratio。
            object.insert("image".to_string(), serde_json::json!({ "url": single }));
        } else {
            object.insert(
                "images".to_string(),
                Value::Array(
                    image_data_urls
                        .iter()
                        .map(|url| serde_json::json!({ "url": url }))
                        .collect(),
                ),
            );
            object.insert(
                "aspect_ratio".to_string(),
                Value::String(
                    image_edit_explicit_aspect_ratio(request)
                        .unwrap_or_else(|| "auto".to_string()),
                ),
            );
        }
    }
    payload
}

async fn edit_xai_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        XAI_IMAGE_EDIT_MAX_IMAGES,
        false,
    )?;
    let mut data_urls = Vec::with_capacity(inputs.images.len());
    for input in &inputs.images {
        data_urls.push(image_edit_data_url(&compress_image_for_xai_edit(input)?));
    }
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/edits");
    let payload = xai_image_edit_payload(request, &resolved.model, &data_urls);
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

// ==================== Seedream 图生图（单图/多图） ====================

fn seedream_image_edit_payload(
    request: &ImageGenerationRequest,
    provider: &ImageGenerationProviderConfig,
    model: &ImageGenerationModelConfig,
    image_data_urls: &[String],
) -> Value {
    let mut prompt = effective_image_generation_prompt(request);
    let explicit_size = trimmed_image_generation_option(&request.size);
    if explicit_size.is_none() {
        if let Some(aspect_ratio) = trimmed_image_generation_option(&request.aspect_ratio) {
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
        if model.model.trim().to_ascii_lowercase().contains("seedream-5-0") {
            object.insert(
                "output_format".to_string(),
                Value::String("png".to_string()),
            );
        }
        let image_value = if let [single] = image_data_urls {
            Value::String(single.clone())
        } else {
            Value::Array(
                image_data_urls
                    .iter()
                    .map(|url| Value::String(url.clone()))
                    .collect(),
            )
        };
        object.insert("image".to_string(), image_value);
        if let Some(size) = explicit_size {
            object.insert("size".to_string(), Value::String(size));
        }
    }
    payload
}

async fn edit_seedream_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        SEEDREAM_IMAGE_EDIT_MAX_IMAGES,
        false,
    )?;
    let data_urls = inputs
        .images
        .iter()
        .map(image_edit_data_url)
        .collect::<Vec<_>>();
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/generations");
    let payload = seedream_image_edit_payload(request, &resolved.provider, &resolved.model, &data_urls);
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

// ==================== Gemini 多模态编辑（inlineData + text） ====================

fn gemini_image_edit_payload(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
    inputs: &ImageEditInputs,
) -> Value {
    let mut response_format = serde_json::Map::<String, Value>::new();
    response_format.insert("type".to_string(), Value::String("image".to_string()));
    response_format.insert(
        "mime_type".to_string(),
        Value::String("image/png".to_string()),
    );
    if let Some(aspect_ratio) = image_edit_explicit_aspect_ratio(request) {
        response_format.insert("aspect_ratio".to_string(), Value::String(aspect_ratio));
    }
    if let Some(size) = trimmed_image_generation_option(&request.size) {
        let normalized = size.trim().to_ascii_uppercase();
        if matches!(normalized.as_str(), "512" | "1K" | "2K" | "4K") {
            response_format.insert("image_size".to_string(), Value::String(normalized));
        }
    }
    let mut input_items = vec![serde_json::json!({
        "type": "text",
        "text": effective_image_generation_prompt(request)
    })];
    for input in &inputs.images {
        input_items.push(serde_json::json!({
            "type": "image",
            "mime_type": input.mime,
            "data": B64.encode(&input.bytes)
        }));
    }
    serde_json::json!({
        "model": model.model,
        "input": input_items,
        "response_format": Value::Object(response_format)
    })
}

async fn edit_gemini_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        GEMINI_IMAGE_EDIT_MAX_IMAGES,
        false,
    )?;
    let payload = gemini_image_edit_payload(request, &resolved.model, inputs);
    let value = post_gemini_image_interactions(state, resolved, api_key, &payload).await?;
    parse_gemini_image_response(&value)
}

async fn edit_sensenova_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    // SenseNova 暂未开放稳定的多模态编辑端点，按 OpenAI 兼容的 images/edits 复用
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        OPENAI_IMAGE_EDIT_MAX_IMAGES,
        true,
    )?;
    let mut form = reqwest::multipart::Form::new();
    for (name, value) in openai_image_edit_scalar_fields(request, &resolved.model) {
        form = form.text(name, value);
    }
    for (index, input) in inputs.images.iter().enumerate() {
        form = form.part("image[]", image_edit_multipart_part(input, &format!("image-{index}"))?);
    }
    if let Some(mask) = &inputs.mask {
        form = form.part("mask", image_edit_multipart_part(mask, "mask")?);
    }
    let endpoint = append_image_generation_endpoint(&resolved.provider.base_url, "/images/edits");
    let response = state
        .shared_http_client
        .post(endpoint)
        .bearer_auth(api_key)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(u64::from(
            resolved.provider.timeout_seconds,
        )))
        .send()
        .await
        .map_err(|err| format!("{} 请求失败：{err}", resolved.provider.name))?;
    let value = parse_image_generation_json_response(response, &resolved.provider.name).await?;
    parse_openai_style_image_response(&value)
}

#[cfg(test)]
mod image_edit_provider_tests {
    use super::*;

    fn edit_request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "把背景换成雪山".to_string(),
            operation: ImageGenerationOperation::Edit,
            images: vec!["{Assistant Space}/downloads/a.png".to_string()],
            ..ImageGenerationRequest::default()
        }
    }

    fn tiny_png_input() -> ImageEditInputImage {
        let mut bytes = Vec::new();
        let buffer = image::RgbImage::new(1, 1);
        image::DynamicImage::ImageRgb8(buffer)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encode tiny png");
        ImageEditInputImage {
            bytes,
            mime: "image/png".to_string(),
        }
    }

    fn edit_inputs(count: usize) -> ImageEditInputs {
        ImageEditInputs {
            images: (0..count).map(|_| tiny_png_input()).collect(),
            mask: None,
        }
    }

    #[test]
    fn openai_edit_fields_should_use_gpt_image_conventions() {
        let mut request = edit_request();
        request.aspect_ratio = Some("16:9".to_string());
        let fields = openai_image_edit_scalar_fields(&request, &ImageGenerationModelConfig::default());
        let value = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(value("model"), Some("gpt-image-2"));
        assert_eq!(value("output_format"), Some("png"));
        assert_eq!(value("size"), Some("1536x864"));
        assert_eq!(value("response_format"), None);
    }

    #[test]
    fn openai_edit_fields_should_skip_size_without_explicit_request() {
        let fields = openai_image_edit_scalar_fields(
            &edit_request(),
            &ImageGenerationModelConfig::default(),
        );
        assert!(!fields.iter().any(|(key, _)| key == "size"));
    }

    #[test]
    fn xai_edit_payload_should_switch_single_and_multi_image_shape() {
        let model = ImageGenerationModelConfig::default();
        let single = xai_image_edit_payload(
            &edit_request(),
            &model,
            &["data:image/png;base64,aGVsbG8=".to_string()],
        );
        assert!(single.get("image").and_then(|value| value.get("url")).is_some());
        assert!(single.get("images").is_none());
        assert!(single.get("aspect_ratio").is_none());

        let mut multi_request = edit_request();
        multi_request.aspect_ratio = Some("16:9".to_string());
        let multi = xai_image_edit_payload(
            &multi_request,
            &model,
            &[
                "data:image/png;base64,YQ==".to_string(),
                "data:image/png;base64,Yg==".to_string(),
            ],
        );
        assert_eq!(
            multi.get("images").and_then(Value::as_array).map(Vec::len),
            Some(2)
        );
        assert_eq!(multi.get("aspect_ratio").and_then(Value::as_str), Some("16:9"));
    }

    #[test]
    fn seedream_edit_payload_should_keep_single_string_and_multi_array() {
        let provider = ImageGenerationProviderConfig::default();
        let mut model = ImageGenerationModelConfig::default();
        model.model = "doubao-seedream-5-0-pro-260628".to_string();
        let single = seedream_image_edit_payload(
            &edit_request(),
            &provider,
            &model,
            &["data:image/png;base64,YQ==".to_string()],
        );
        assert!(single.get("image").and_then(Value::as_str).is_some());
        assert_eq!(single.get("output_format").and_then(Value::as_str), Some("png"));
        // 编辑默认跟随原图，不落模型默认尺寸。
        assert!(single.get("size").is_none());

        let multi = seedream_image_edit_payload(
            &edit_request(),
            &provider,
            &model,
            &[
                "data:image/png;base64,YQ==".to_string(),
                "data:image/png;base64,Yg==".to_string(),
            ],
        );
        assert_eq!(
            multi.get("image").and_then(Value::as_array).map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn gemini_edit_payload_should_carry_text_and_inline_images() {
        let inputs = edit_inputs(2);
        let payload = gemini_image_edit_payload(
            &edit_request(),
            &ImageGenerationModelConfig::default(),
            &inputs,
        );
        let items = payload.get("input").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].get("type").and_then(Value::as_str), Some("text"));
        assert_eq!(items[1].get("type").and_then(Value::as_str), Some("image"));
        assert_eq!(
            items[1].get("mime_type").and_then(Value::as_str),
            Some("image/png")
        );
    }

    #[test]
    fn edit_limits_should_reject_unsupported_mask_and_too_many_images() {
        let mut masked = edit_inputs(1);
        masked.mask = Some(tiny_png_input());
        let mask_error = ensure_image_edit_input_limits("xAI", &masked, 3, false)
            .err()
            .unwrap_or_default();
        assert!(mask_error.contains("mask"));

        let overflow = edit_inputs(4);
        let count_error = ensure_image_edit_input_limits("xAI", &overflow, 3, true)
            .err()
            .unwrap_or_default();
        assert!(count_error.contains("最多支持 3 张"));
    }

    #[test]
    fn xai_compression_should_pass_through_small_png() {
        let input = tiny_png_input();
        let compressed = compress_image_for_xai_edit(&input).expect("compress tiny png");
        assert_eq!(compressed.mime, "image/png");
        assert_eq!(compressed.bytes, input.bytes);
    }
}
