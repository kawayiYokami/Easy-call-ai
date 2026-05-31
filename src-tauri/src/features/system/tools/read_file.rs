const READ_FILE_TEXT_LIMIT_CHARS: usize = 30_000;
const READ_TOOL_NAME: &str = "read";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadFileDetectedType {
    Text,
    Image,
    Pdf,
    Doc,
    Docx,
    Xls,
    Xlsx,
    Xlsb,
    Ppt,
    Pptx,
    Ods,
    Odp,
    Rtf,
    Numbers,
    Pages,
    Keynote,
    Unknown,
}

impl ReadFileDetectedType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Pdf => "pdf",
            Self::Doc => "doc",
            Self::Docx => "docx",
            Self::Xls => "xls",
            Self::Xlsx => "xlsx",
            Self::Xlsb => "xlsb",
            Self::Ppt => "ppt",
            Self::Pptx => "pptx",
            Self::Ods => "ods",
            Self::Odp => "odp",
            Self::Rtf => "rtf",
            Self::Numbers => "numbers",
            Self::Pages => "pages",
            Self::Keynote => "keynote",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReadFileRequest {
    #[serde(alias = "absolute_path", alias = "absolutePath")]
    path: String,
    #[serde(default)]
    #[serde(alias = "start")]
    offset: Option<usize>,
    #[serde(default)]
    #[serde(alias = "count")]
    limit: Option<usize>,
}

trait ReadFileReader {
    fn reader_kind(&self) -> &'static str;
    fn supports(&self, detected: ReadFileDetectedType) -> bool;
    fn read(
        &self,
        state: &AppState,
        session_id: &str,
        api_config_id: &str,
        request: &ReadFileRequest,
        detected: ReadFileDetectedType,
    ) -> Result<Value, String>;
}

fn read_file_ext(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default()
}

fn detect_read_file_type(path: &std::path::Path) -> ReadFileDetectedType {
    match read_file_ext(path).as_str() {
        "txt" | "md" | "rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "toml" | "yaml" | "yml"
        | "vue" | "html" | "css" | "scss" | "less" | "xml" | "csv" | "log" | "ini" | "conf"
        | "bat" | "cmd" | "ps1" | "sh" | "sql" | "py" | "java" | "kt" | "go" | "c" | "cpp"
        | "h" | "hpp" | "cs" | "swift" | "rb" | "php" | "svg" => ReadFileDetectedType::Text,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => ReadFileDetectedType::Image,
        "pdf" => ReadFileDetectedType::Pdf,
        "doc" => ReadFileDetectedType::Doc,
        "docx" => ReadFileDetectedType::Docx,
        "xls" => ReadFileDetectedType::Xls,
        "xlsx" => ReadFileDetectedType::Xlsx,
        "xlsb" => ReadFileDetectedType::Xlsb,
        "ppt" => ReadFileDetectedType::Ppt,
        "pptx" => ReadFileDetectedType::Pptx,
        "ods" => ReadFileDetectedType::Ods,
        "odp" => ReadFileDetectedType::Odp,
        "rtf" => ReadFileDetectedType::Rtf,
        "numbers" => ReadFileDetectedType::Numbers,
        "pages" => ReadFileDetectedType::Pages,
        "key" => ReadFileDetectedType::Keynote,
        _ => ReadFileDetectedType::Unknown,
    }
}

fn read_file_conversation_cache_key(session_id: &str) -> String {
    session_id
        .split_once("::")
        .and_then(|(_, conversation_id)| {
            let trimmed = conversation_id.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| session_id.trim().to_string())
}

fn read_file_log_target(path: &std::path::Path) -> String {
    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or_default();
    if !file_name.is_empty() {
        if ext.is_empty() {
            return format!("file_name={}", file_name);
        }
        return format!("file_name={}，ext={}", file_name, ext);
    }
    if ext.is_empty() {
        "file_name=(unknown)".to_string()
    } else {
        format!("file_name=(unknown)，ext={}", ext)
    }
}

fn ensure_absolute_file_path(request: &ReadFileRequest) -> Result<std::path::PathBuf, String> {
    let trimmed = request.path.trim();
    if trimmed.is_empty() {
        return Err("path 不能为空".to_string());
    }
    if matches!(request.limit, Some(0)) {
        return Err("limit 必须大于等于 1".to_string());
    }
    let path = std::path::PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("path 必须是绝对路径".to_string());
    }
    if !path.exists() {
        return Err(format!("文件不存在：{}", path.display()));
    }
    let metadata = std::fs::metadata(&path).map_err(|err| format!("读取文件信息失败: {err}"))?;
    if !metadata.is_file() {
        return Err(format!("目标不是文件：{}", path.display()));
    }
    Ok(path)
}

fn paginate_lines(lines: &[String], start: usize, count: Option<usize>) -> (Vec<String>, Option<usize>) {
    if start >= lines.len() {
        return (Vec::new(), None);
    }
    let end = count
        .map(|size| start.saturating_add(size).min(lines.len()))
        .unwrap_or(lines.len());
    let chunk = lines[start..end].to_vec();
    let next_start = if end < lines.len() { Some(end) } else { None };
    (chunk, next_start)
}

fn paginate_window(total: usize, start: usize, count: Option<usize>) -> (usize, usize, Option<usize>) {
    if start >= total {
        return (start, start, None);
    }
    let end = count
        .map(|size| start.saturating_add(size).min(total))
        .unwrap_or(total);
    let next_start = if end < total { Some(end) } else { None };
    (start, end, next_start)
}

fn truncate_text_for_read_file(text: &str) -> (String, bool) {
    let total = text.chars().count();
    if total <= READ_FILE_TEXT_LIMIT_CHARS {
        return (text.to_string(), false);
    }
    (
        text.chars().take(READ_FILE_TEXT_LIMIT_CHARS).collect::<String>(),
        true,
    )
}

fn detect_read_file_line_ending(text: &str) -> &'static str {
    let has_crlf = text.contains("\r\n");
    let without_crlf = text.replace("\r\n", "");
    let has_cr = without_crlf.contains('\r');
    let has_lf = without_crlf.contains('\n') || has_crlf;
    match (has_crlf, has_cr, has_lf) {
        (true, false, true) if !without_crlf.contains('\n') => "crlf",
        (false, false, true) => "lf",
        (false, true, false) => "cr",
        (false, false, false) => "none",
        _ => "mixed",
    }
}

fn normalize_text_line_endings_for_read_file(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn normalize_office_text_for_read_file(input: &str) -> String {
    let normalized = normalize_text_line_endings_for_read_file(input);
    let mut out = String::with_capacity(normalized.len());
    let mut last_was_newline = false;
    for ch in normalized.chars() {
        if ch == '\n' {
            if !last_was_newline {
                out.push('\n');
            }
            last_was_newline = true;
            continue;
        }
        if ch == '\t' {
            out.push('\t');
            last_was_newline = false;
            continue;
        }
        if ch.is_control() {
            continue;
        }
        out.push(ch);
        last_was_newline = false;
    }
    out.trim().to_string()
}

fn build_text_read_result(
    path: &std::path::Path,
    detected: ReadFileDetectedType,
    reader_kind: &str,
    text: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    extra_metadata: Value,
) -> Value {
    let source_line_ending = detect_read_file_line_ending(text);
    let normalized_text = normalize_text_line_endings_for_read_file(text);
    let lines = normalized_text.split('\n').map(|v| v.to_string()).collect::<Vec<_>>();
    let applied_offset = offset.unwrap_or(0);
    let (selected_lines, next_offset_by_lines) = paginate_lines(&lines, applied_offset, limit);
    let joined = selected_lines.join("\n");
    let (truncated_text, char_truncated) = truncate_text_for_read_file(&joined);
    let next_offset = if char_truncated {
        next_offset_by_lines.or(Some(applied_offset + selected_lines.len()))
    } else {
        next_offset_by_lines
    };
    let mut output = String::new();
    if char_truncated {
        let continue_offset = next_offset.unwrap_or(applied_offset + selected_lines.len());
        output.push_str("Content was truncated to fit within 30000 character limit.\n");
        output.push_str(&format!(
            "To continue reading, use offset={} in the next read call.\n\n",
            continue_offset
        ));
    }
    output.push_str(&truncated_text);
    serde_json::json!({
        "ok": true,
        "path": path.to_string_lossy().to_string(),
        "detectedType": detected.as_str(),
        "readerKind": reader_kind,
        "truncated": char_truncated,
        "nextOffset": next_offset,
        "content": output,
        "metadata": {
            "kind": "text",
            "offset": applied_offset,
            "limit": limit,
            "returnedCount": selected_lines.len(),
            "totalCount": lines.len(),
            "returnedCharCount": joined.chars().count().min(READ_FILE_TEXT_LIMIT_CHARS),
            "charLimit": READ_FILE_TEXT_LIMIT_CHARS,
            "sourceLineEnding": source_line_ending,
            "contentLineEnding": "lf",
            "lineEndingNote": "content 已统一使用 LF(\\n) 返回；apply_patch 可用该内容作为 old_string，工具会兼容目标文件的 CRLF/LF。",
            "fileName": path.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
            "extra": extra_metadata
        }
    })
}

fn resolve_pdf_image_mode(state: &AppState, api_config_id: &str) -> Result<bool, String> {
    let app_config = state_read_config_cached(state)?;
    let runtime = state_read_runtime_state_cached(state)?;
    let selected_api = resolve_selected_api_config(&app_config, Some(api_config_id))
        .or_else(|| resolve_selected_api_config(&app_config, None))
        .ok_or_else(|| "当前未找到可用聊天模型配置。".to_string())?;
    Ok(runtime.pdf_read_mode == "image" && selected_api.enable_image)
}

fn build_pdf_image_read_result(
    path: &std::path::Path,
    detected: ReadFileDetectedType,
    structured: &PdfExtractStructuredResult,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Value {
    let applied_offset = offset.unwrap_or(0);
    let total_pages = structured.total_pages as usize;
    let (window_start, end, next_offset) = paginate_window(total_pages, applied_offset, limit);
    let selected_pages = if window_start >= total_pages {
        Vec::new()
    } else {
        structured.pages[window_start..end].to_vec()
    };
    let parts = selected_pages
        .iter()
        .flat_map(|page| {
            page.images.iter().map(move |image| {
                serde_json::json!({
                    "type": "image",
                    "mimeType": image.mime,
                    "data": image.bytes_base64,
                    "pageIndex": page.page_index,
                    "pageNumber": page.page_index + 1,
                    "width": image.width,
                    "height": image.height
                })
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "ok": true,
        "path": path.to_string_lossy().to_string(),
        "detectedType": detected.as_str(),
        "readerKind": "pdf_image_direct",
        "truncated": false,
        "nextOffset": next_offset,
        "parts": parts,
        "response": {
            "ok": true,
            "path": path.to_string_lossy().to_string(),
            "detectedType": detected.as_str(),
            "readerKind": "pdf_image_direct",
            "fileName": structured.file_name,
            "offset": applied_offset,
            "limit": limit,
            "returnedPageCount": selected_pages.len(),
            "returnedImageCount": selected_pages.iter().map(|page| page.images.len()).sum::<usize>(),
            "totalPages": structured.total_pages,
            "nextOffset": next_offset
        },
        "metadata": {
            "kind": "image",
            "fileName": structured.file_name,
            "offset": applied_offset,
            "limit": limit,
            "returnedPageCount": selected_pages.len(),
            "returnedImageCount": selected_pages.iter().map(|page| page.images.len()).sum::<usize>(),
            "totalPages": structured.total_pages,
            "includeImages": true
        }
    })
}

async fn read_image_file_result(
    state: &AppState,
    session_id: &str,
    api_config_id: &str,
    request: &ReadFileRequest,
    detected: ReadFileDetectedType,
) -> Result<Value, String> {
    let path = ensure_absolute_file_path(request)?;
    eprintln!(
        "[read] 开始，任务=read_image_file，session_id={}，api_config_id={}，{}，detected_type={}",
        session_id,
        api_config_id,
        read_file_log_target(&path),
        detected.as_str()
    );
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|err| format!("读取图片文件失败: {err}"))?;
    let mime = image_mime_from_bytes(&bytes)
        .or_else(|| media_mime_from_path(&path))
        .unwrap_or("application/octet-stream")
        .to_string();
    let normalized = match normalize_image_bytes_for_llm_request(&bytes, Some(&mime)) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "[read] 图片规范化失败，降级为文本提示，session_id={}，api_config_id={}，{}，err={}",
                session_id,
                api_config_id,
                read_file_log_target(&path),
                err
            );
            return Ok(build_image_read_fallback_text_result(
                &path, detected, request, &mime, &err,
            ));
        }
    };

    let app_config = state_read_config_cached(state)?;
    let selected_api = resolve_selected_api_config(&app_config, Some(api_config_id))
        .or_else(|| resolve_selected_api_config(&app_config, None))
        .ok_or_else(|| "当前未找到可用聊天模型配置。".to_string())?;

    if selected_api.enable_image {
        return read_image_direct(
            state,
            session_id,
            &path,
            detected,
            &normalized,
            api_config_id,
        )
        .await;
    }

    eprintln!(
        "[read] 完成，任务=read_image_file，session_id={}，api_config_id={}，reader=image_vision_fallback，detected_type={}，action=使用图转文回退",
        session_id,
        api_config_id,
        detected.as_str()
    );
    match read_image_via_vision(
        state,
        session_id,
        &path,
        request,
        detected,
        &normalized,
        &app_config,
    )
    .await
    {
        Ok(value) => Ok(value),
        Err(err) => {
            eprintln!(
                "[read] 图片视觉回退失败，降级为文本提示，session_id={}，api_config_id={}，{}，err={}",
                session_id,
                api_config_id,
                read_file_log_target(&path),
                err
            );
            Ok(build_image_read_fallback_text_result(
                &path,
                detected,
                request,
                &normalized.mime,
                &err,
            ))
        }
    }
}

fn build_direct_image_read_result(
    path: &std::path::Path,
    detected: ReadFileDetectedType,
    mime: &str,
    image_base64: String,
    byte_size: u64,
    original_width: u32,
    original_height: u32,
    output_width: u32,
    output_height: u32,
) -> Value {
    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
    serde_json::json!({
        "ok": true,
        "path": path.to_string_lossy().to_string(),
        "detectedType": detected.as_str(),
        "readerKind": "image_direct",
        "truncated": false,
        "nextOffset": Value::Null,
        "imageMime": mime,
        "imageBase64": image_base64,
        "response": {
            "ok": true,
            "path": path.to_string_lossy().to_string(),
            "detectedType": detected.as_str(),
            "readerKind": "image_direct",
            "imageMime": mime,
            "fileName": file_name,
            "byteSize": byte_size,
            "originalWidth": original_width,
            "originalHeight": original_height,
            "outputWidth": output_width,
            "outputHeight": output_height
        },
        "metadata": {
            "fileName": file_name,
            "byteSize": byte_size,
            "originalWidth": original_width,
            "originalHeight": original_height,
            "outputWidth": output_width,
            "outputHeight": output_height
        }
    })
}

fn build_image_read_fallback_text_result(
    path: &std::path::Path,
    detected: ReadFileDetectedType,
    request: &ReadFileRequest,
    mime: &str,
    reason: &str,
) -> Value {
    let text = format!(
        "该文件被识别为图片，但本次未能作为图片输入直接提供给模型。\n原因：{}\n文件路径：{}\n原始 MIME：{}\n请按普通附件理解该文件；如需继续处理，可用 shell 或后续 read 查看相关信息。",
        reason.trim(),
        path.display(),
        mime.trim()
    );
    build_text_read_result(
        path,
        detected,
        "image_fallback_notice",
        &text,
        request.offset,
        request.limit,
        serde_json::json!({
            "imageDeliveredAsTextNotice": true,
            "imageMime": mime,
            "reason": reason.trim(),
        }),
    )
}

async fn read_image_direct(
    _state: &AppState,
    session_id: &str,
    path: &std::path::Path,
    detected: ReadFileDetectedType,
    normalized: &LlmRequestNormalizedImage,
    api_config_id: &str,
) -> Result<Value, String> {
    let byte_size = normalized.bytes.len() as u64;
    eprintln!(
        "[read] 完成，任务=read_image_file，session_id={}，api_config_id={}，reader=image_direct，detected_type={}，action=直接返回图片，byte_size={}",
        session_id,
        api_config_id,
        detected.as_str(),
        byte_size
    );
    Ok(build_direct_image_read_result(
        path,
        detected,
        &normalized.mime,
        B64.encode(&normalized.bytes),
        byte_size,
        normalized.original_width,
        normalized.original_height,
        normalized.output_width,
        normalized.output_height,
    ))
}

async fn read_image_via_vision(
    state: &AppState,
    session_id: &str,
    path: &std::path::Path,
    request: &ReadFileRequest,
    detected: ReadFileDetectedType,
    normalized: &LlmRequestNormalizedImage,
    app_config: &AppConfig,
) -> Result<Value, String> {
    let vision_api = resolve_vision_api_config(&app_config).map_err(|_| {
        "Image reading is not supported: current model has image input disabled and no vision fallback model is configured.".to_string()
    })?;
    let vision_resolved = resolve_api_config(&app_config, Some(vision_api.id.as_str()))?;
    if !vision_resolved.request_format.is_chat_text() {
        return Err(format!(
            "Image reading is not supported: configured vision fallback request format '{}' is not implemented.",
            vision_resolved.request_format
        ));
    }

    eprintln!(
        "[read] 开始，任务=read_image_via_vision，session_id={}，vision_api_id={}，{}，detected_type={}",
        session_id,
        vision_api.id,
        read_file_log_target(path),
        detected.as_str()
    );
    let image = BinaryPart {
        mime: normalized.mime.clone(),
        bytes_base64: B64.encode(&normalized.bytes),
        saved_path: None,
    };
    let hash = compute_image_hash_hex(&image)?;
    let cached = {
        let runtime = state_read_runtime_state_cached(state)?;
        runtime
            .image_text_cache
            .iter()
            .find(|entry| entry.hash == hash && entry.vision_api_id == vision_api.id)
            .map(|entry| entry.text.clone())
    };
    let described = if let Some(text) = cached {
        text
    } else {
        let converted = describe_image_with_vision_api(state, &vision_resolved, &vision_api, &image)
            .await?
            .trim()
            .to_string();
        if converted.is_empty() {
            return Err("Vision fallback returned empty text for the image.".to_string());
        }
        let mut runtime = state_read_runtime_state_cached(state)?;
        if let Some(entry) = runtime
            .image_text_cache
            .iter_mut()
            .find(|entry| entry.hash == hash && entry.vision_api_id == vision_api.id)
        {
            entry.text = converted.clone();
            entry.updated_at = now_iso();
        } else {
            runtime.image_text_cache.push(ImageTextCacheEntry {
                hash: hash.clone(),
                vision_api_id: vision_api.id.clone(),
                text: converted.clone(),
                updated_at: now_iso(),
            });
            if runtime.image_text_cache.len() > MAX_IMAGE_TEXT_CACHE_ENTRIES {
                if let Some((oldest_idx, _)) = runtime
                    .image_text_cache
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.updated_at.cmp(&b.updated_at))
                {
                    runtime.image_text_cache.remove(oldest_idx);
                }
            }
        }
        state_write_runtime_state_cached(state, &runtime)?;
        converted
    };

    Ok(build_text_read_result(
        path,
        detected,
        "image_vision_fallback",
        &described,
        request.offset,
        request.limit,
        serde_json::json!({
            "imageConvertedToText": true,
            "visionApiId": vision_api.id,
            "experimental": false
        }),
    ))
}

struct TextFileReader;

impl ReadFileReader for TextFileReader {
    fn reader_kind(&self) -> &'static str {
        "text"
    }

    fn supports(&self, detected: ReadFileDetectedType) -> bool {
        matches!(detected, ReadFileDetectedType::Text)
    }

    fn read(
        &self,
        _state: &AppState,
        _session_id: &str,
        _api_config_id: &str,
        request: &ReadFileRequest,
        detected: ReadFileDetectedType,
    ) -> Result<Value, String> {
        let path = ensure_absolute_file_path(request)?;
        let decoded = decode_text_file_from_path(&path)
            .map_err(|err| format!("读取文本文件失败：{err}"))?;
        Ok(build_text_read_result(
            &path,
            detected,
            self.reader_kind(),
            &decoded.text,
            request.offset,
            request.limit,
            serde_json::json!({}),
        ))
    }
}

struct PdfFileReader;

impl ReadFileReader for PdfFileReader {
    fn reader_kind(&self) -> &'static str {
        "pdf_builtin"
    }

    fn supports(&self, detected: ReadFileDetectedType) -> bool {
        matches!(detected, ReadFileDetectedType::Pdf)
    }

    fn read(
        &self,
        state: &AppState,
        session_id: &str,
        api_config_id: &str,
        request: &ReadFileRequest,
        detected: ReadFileDetectedType,
    ) -> Result<Value, String> {
        let path = ensure_absolute_file_path(request)?;
        let conversation_id = read_file_conversation_cache_key(session_id);
        let include_images = resolve_pdf_image_mode(state, api_config_id)?;
        let structured = match get_or_extract_pdf_structured(
            state,
            &conversation_id,
            &path.to_string_lossy(),
            include_images,
        ) {
            Ok(value) => value,
            Err(err) if include_images && !is_pdf_page_limit_exceeded_error(&err) => {
                eprintln!(
                    "[read] PDF 页图提取失败，降级为文本读取，file={}，err={}",
                    path.display(),
                    err
                );
                let mut fallback = get_or_extract_pdf_structured(
                    state,
                    &conversation_id,
                    &path.to_string_lossy(),
                    false,
                )?;
                if let Some(first_page) = fallback.pages.first_mut() {
                    if first_page.text.trim().is_empty() {
                        first_page.text = format!(
                            "[系统提示] PDF 页图未能成功提供给模型，已自动回退为文本读取。\n原因：{}",
                            err.trim()
                        );
                    } else {
                        first_page.text = format!(
                            "[系统提示] PDF 页图未能成功提供给模型，已自动回退为文本读取。\n原因：{}\n\n{}",
                            err.trim(),
                            first_page.text
                        );
                    }
                } else {
                    fallback.pages.push(PdfPageExtractBlock {
                        page_index: 0,
                        text: format!(
                            "[系统提示] PDF 页图未能成功提供给模型，已自动回退为文本读取。\n原因：{}",
                            err.trim()
                        ),
                        images: Vec::new(),
                    });
                }
                fallback
            }
            Err(err) => return Err(err),
        };
        if include_images {
            return Ok(build_pdf_image_read_result(
                &path,
                detected,
                &structured,
                request.offset,
                request.limit,
            ));
        }
        let text = structured
            .pages
            .iter()
            .map(|page| format!("[第 {} 页]\n{}", page.page_index + 1, page.text))
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(build_text_read_result(
            &path,
            detected,
            self.reader_kind(),
            &text,
            request.offset,
            request.limit,
            serde_json::json!({
                "totalPages": structured.total_pages,
                "includeImages": structured.include_images
            }),
        ))
    }
}

struct OfficeLitchiReader;

impl ReadFileReader for OfficeLitchiReader {
    fn reader_kind(&self) -> &'static str {
        "litchi"
    }

    fn supports(&self, detected: ReadFileDetectedType) -> bool {
        matches!(
            detected,
            ReadFileDetectedType::Doc
                | ReadFileDetectedType::Docx
                | ReadFileDetectedType::Xls
                | ReadFileDetectedType::Xlsx
                | ReadFileDetectedType::Xlsb
                | ReadFileDetectedType::Ppt
                | ReadFileDetectedType::Pptx
                | ReadFileDetectedType::Ods
                | ReadFileDetectedType::Odp
                | ReadFileDetectedType::Rtf
                | ReadFileDetectedType::Numbers
                | ReadFileDetectedType::Pages
                | ReadFileDetectedType::Keynote
        )
    }

    fn read(
        &self,
        _state: &AppState,
        _session_id: &str,
        _api_config_id: &str,
        request: &ReadFileRequest,
        detected: ReadFileDetectedType,
    ) -> Result<Value, String> {
        let path = ensure_absolute_file_path(request)?;
        let path_for_read = path.clone();
        let detected_for_read = detected;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || -> Result<String, String> {
            match detected_for_read {
                ReadFileDetectedType::Doc
                | ReadFileDetectedType::Docx
                | ReadFileDetectedType::Rtf
                | ReadFileDetectedType::Pages => {
                    let document = litchi::Document::open(&path_for_read)
                        .map_err(|err| format!("litchi 打开文档失败: {err}"))?;
                    document
                        .text()
                        .map_err(|err| format!("litchi 提取文档文本失败: {err}"))
                }
                ReadFileDetectedType::Ppt
                | ReadFileDetectedType::Pptx
                | ReadFileDetectedType::Odp
                | ReadFileDetectedType::Keynote => {
                    let presentation = litchi::Presentation::open(&path_for_read)
                        .map_err(|err| format!("litchi 打开演示文稿失败: {err}"))?;
                    presentation
                        .text()
                        .map_err(|err| format!("litchi 提取演示文稿文本失败: {err}"))
                }
                ReadFileDetectedType::Xls
                | ReadFileDetectedType::Xlsx
                | ReadFileDetectedType::Xlsb
                | ReadFileDetectedType::Ods
                | ReadFileDetectedType::Numbers => {
                    let workbook = litchi::sheet::Workbook::open(&path_for_read)
                        .map_err(|err| format!("litchi 打开表格失败: {err}"))?;
                    workbook
                        .text()
                        .map_err(|err| format!("litchi 提取表格文本失败: {err}"))
                }
                _ => Err("当前 Office 类型尚未接入 litchi reader".to_string()),
            }
        }));
        let text = match caught {
            Ok(result) => result?,
            Err(_) => {
                return Err(format!(
                    "实验性 Office reader 解析失败并触发 panic：{}",
                    path.display()
                ))
            }
        };
        let text = normalize_office_text_for_read_file(&text);
        Ok(build_text_read_result(
            &path,
            detected,
            self.reader_kind(),
            &text,
            request.offset,
            request.limit,
            serde_json::json!({
                "experimental": true
            }),
        ))
    }
}

async fn builtin_read_file(
    state: &AppState,
    session_id: &str,
    api_config_id: &str,
    request: ReadFileRequest,
) -> Result<Value, String> {
    let started = std::time::Instant::now();
    let path = ensure_absolute_file_path(&request)?;
    let detected = detect_read_file_type(&path);
    eprintln!(
        "[read] 开始，任务=read，session_id={}，api_config_id={}，{}，detected_type={}",
        session_id,
        api_config_id,
        read_file_log_target(&path),
        detected.as_str()
    );
    if matches!(detected, ReadFileDetectedType::Unknown) {
        return Err(format!(
            "暂不支持该文件类型：{}",
            path.extension().and_then(|v| v.to_str()).unwrap_or_default()
        ));
    }
    if matches!(detected, ReadFileDetectedType::Image) {
        let result = read_image_file_result(state, session_id, api_config_id, &request, detected).await;
        let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        match &result {
            Ok(value) => eprintln!(
                "[read] 完成，任务=read，session_id={}，api_config_id={}，reader={}，detected_type={}，elapsed_ms={}",
                session_id,
                api_config_id,
                value.get("readerKind").and_then(Value::as_str).unwrap_or("image"),
                detected.as_str(),
                elapsed_ms
            ),
            Err(err) => eprintln!(
                "[read] 失败，任务=read，session_id={}，api_config_id={}，detected_type={}，elapsed_ms={}，error={}",
                session_id,
                api_config_id,
                detected.as_str(),
                elapsed_ms,
                err
            ),
        }
        return result;
    }
    let readers: [&dyn ReadFileReader; 3] = [
        &TextFileReader,
        &PdfFileReader,
        &OfficeLitchiReader,
    ];
    let reader = readers
        .into_iter()
        .find(|item| item.supports(detected))
        .ok_or_else(|| format!("未找到可用读取器：{}", detected.as_str()))?;
    let result = reader.read(state, session_id, api_config_id, &request, detected);
    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match &result {
        Ok(_) => eprintln!(
            "[read] 完成，任务=read，session_id={}，api_config_id={}，reader={}，detected_type={}，elapsed_ms={}",
            session_id,
            api_config_id,
            reader.reader_kind(),
            detected.as_str(),
            elapsed_ms
        ),
        Err(err) => eprintln!(
            "[read] 失败，任务=read，session_id={}，api_config_id={}，reader={}，detected_type={}，elapsed_ms={}，error={}",
            session_id,
            api_config_id,
            reader.reader_kind(),
            detected.as_str(),
            elapsed_ms,
            err
        ),
    }
    result
}

#[cfg(test)]
fn test_read_file_state() -> AppState {
        let root = std::env::temp_dir().join(format!("eca-read-file-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp test root");
        std::fs::create_dir_all(root.join("llm-workspace")).expect("create temp llm workspace");
        AppState {
            app_handle: Arc::new(Mutex::new(None)),
            config_path: root.join("app_config.toml"),
            data_path: root.join("app_data.json"),
            llm_workspace_path: root.join("llm-workspace"),
            shared_http_client: reqwest::Client::new(),
            terminal_shell: detect_default_terminal_shell(),
            terminal_shell_candidates: detect_terminal_shell_candidates(),
            conversation_lock: Arc::new(ConversationDomainLock::new()),
            memory_lock: Arc::new(Mutex::new(())),
            cached_config: Arc::new(Mutex::new(None)),
            cached_config_mtime: Arc::new(Mutex::new(None)),
            cached_agents: Arc::new(Mutex::new(None)),
            cached_agents_mtime: Arc::new(Mutex::new(None)),
            cached_runtime_state: Arc::new(Mutex::new(None)),
            cached_runtime_state_mtime: Arc::new(Mutex::new(None)),
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_chat_index_mtime: Arc::new(Mutex::new(None)),
            cached_conversations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_pending: Arc::new(Mutex::new(None)),
            app_data_persist_notify: Arc::new(tokio::sync::Notify::new()),
            app_data_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            conversation_persist_pending: Arc::new(Mutex::new(None)),
            conversation_persist_notify: Arc::new(tokio::sync::Notify::new()),
            conversation_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conversation_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_conversation_dirty_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_deleted_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_chat_index_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_write_lock: Arc::new(Mutex::new(())),
            last_panic_snapshot: Arc::new(Mutex::new(None)),
            inflight_chat_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_tool_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_completed_tool_history: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_session_roots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            conversation_index_repair_gates: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new("github".to_string())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

#[cfg(test)]
#[test]
fn read_file_request_should_accept_new_and_legacy_argument_names() {
        let current: ReadFileRequest = serde_json::from_str(
            r#"{"path":"E:\\docs\\a.md","offset":2,"limit":5}"#,
        )
        .expect("parse current read args");
        assert_eq!(current.path, "E:\\docs\\a.md");
        assert_eq!(current.offset, Some(2));
        assert_eq!(current.limit, Some(5));

        let legacy: ReadFileRequest = serde_json::from_str(
            r#"{"absolute_path":"E:\\docs\\b.md","start":3,"count":7}"#,
        )
        .expect("parse legacy read_file args");
        assert_eq!(legacy.path, "E:\\docs\\b.md");
        assert_eq!(legacy.offset, Some(3));
        assert_eq!(legacy.limit, Some(7));
}

#[cfg(test)]
#[test]
fn detect_read_file_type_should_classify_common_formats() {
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.txt")),
            ReadFileDetectedType::Text
        );
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.svg")),
            ReadFileDetectedType::Text
        );
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.pdf")),
            ReadFileDetectedType::Pdf
        );
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.doc")),
            ReadFileDetectedType::Doc
        );
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.xlsx")),
            ReadFileDetectedType::Xlsx
        );
        assert_eq!(
            detect_read_file_type(std::path::Path::new("a.ppt")),
            ReadFileDetectedType::Ppt
        );
    }

#[cfg(test)]
#[test]
fn image_mime_from_bytes_should_detect_common_images_without_extension() {
        assert_eq!(
            image_mime_from_bytes(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            Some("image/png")
        );
        assert_eq!(
            image_mime_from_bytes(&[0xff, 0xd8, 0xff, 0xe0, 0, 0x10, b'J', b'F', b'I', b'F', 0]),
            Some("image/jpeg")
        );
        assert_eq!(image_mime_from_bytes(b"hello world"), None);
    }

#[cfg(test)]
#[test]
fn normalize_office_text_should_drop_control_chars() {
        let input = "a\u{0001}b\r\n\r\nc\t\u{0004}d";
        let output = normalize_office_text_for_read_file(input);
        assert_eq!(output, "ab\nc\td");
    }

#[cfg(test)]
#[test]
fn build_text_read_result_should_normalize_crlf_to_lf_and_report_source_line_ending() {
        let path = std::path::Path::new("sample.txt");
        let value = build_text_read_result(
            path,
            ReadFileDetectedType::Text,
            "text",
            "line1\r\nline2\r\n",
            None,
            None,
            serde_json::json!({}),
        );
        assert_eq!(
            value.get("content").and_then(Value::as_str),
            Some("line1\nline2\n")
        );
        let metadata = value.get("metadata").expect("metadata");
        assert_eq!(
            metadata.get("sourceLineEnding").and_then(Value::as_str),
            Some("crlf")
        );
        assert_eq!(
            metadata.get("contentLineEnding").and_then(Value::as_str),
            Some("lf")
        );
    }

#[cfg(test)]
#[test]
fn build_text_read_result_should_normalize_lone_cr_to_lf() {
        let path = std::path::Path::new("sample.txt");
        let value = build_text_read_result(
            path,
            ReadFileDetectedType::Text,
            "text",
            "line1\rline2\r",
            None,
            None,
            serde_json::json!({}),
        );
        assert_eq!(
            value.get("content").and_then(Value::as_str),
            Some("line1\nline2\n")
        );
        let metadata = value.get("metadata").expect("metadata");
        assert_eq!(
            metadata.get("sourceLineEnding").and_then(Value::as_str),
            Some("cr")
        );
    }

#[cfg(test)]
#[test]
fn builtin_read_file_should_paginate_text_file() {
        let root = std::env::temp_dir().join(format!("eca-read-file-page-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("sample.txt");
        std::fs::write(&file, "line1\nline2\nline3\nline4\n").expect("write sample text");
        let state = test_read_file_state();
        let value = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(builtin_read_file(
            &state,
            "chat::conv-1",
            "__frontend_tool_preview__",
            ReadFileRequest {
                path: file.to_string_lossy().to_string(),
                offset: Some(1),
                limit: Some(2),
            },
        ))
        .expect("read text");
        assert_eq!(value.get("detectedType").and_then(Value::as_str), Some("text"));
        assert_eq!(
            value.get("content").and_then(Value::as_str),
            Some("line2\nline3")
        );
        assert_eq!(value.get("nextOffset").and_then(Value::as_u64), Some(3));
}

#[cfg(test)]
#[test]
fn builtin_read_file_should_decode_gbk_text_file() {
        let root = std::env::temp_dir().join(format!("eca-read-file-gbk-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("sample.txt");
        std::fs::write(&file, [0xd6, 0xd0, 0xce, 0xc4, b'\n']).expect("write gbk text");
        let state = test_read_file_state();
        let value = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(builtin_read_file(
            &state,
            "chat::conv-1",
            "__frontend_tool_preview__",
            ReadFileRequest {
                path: file.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            },
        ))
        .expect("read gbk text");
        assert_eq!(value.get("content").and_then(Value::as_str), Some("中文\n"));
}

#[cfg(test)]
#[test]
fn builtin_read_file_should_return_root_image_payload_when_model_supports_image() {
        let root = std::env::temp_dir().join(format!("eca-read-file-image-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("sample.png");
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([255, 0, 0, 255]),
        ))
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("encode png");
        std::fs::write(&file, cursor.into_inner()).expect("write sample image");
        let state = test_read_file_state();
        let config = AppConfig {
            selected_api_config_id: "vision-a".to_string(),
            assistant_department_api_config_id: "vision-a".to_string(),
            api_configs: vec![ApiConfig {
                id: "vision-a".to_string(),
                name: "vision-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: true,
                enable_audio: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://example.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                model: "gpt-image".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 0.7,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            api_providers: Vec::new(),
            ..AppConfig::default()
        };
        state_write_config_cached(&state, &config).expect("write config");

        let value = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime")
            .block_on(async {
                builtin_read_file(
                    &state,
                    "assistant::conversation-a",
                    "vision-a",
                    ReadFileRequest {
                        path: file.to_string_lossy().to_string(),
                        offset: None,
                        limit: None,
                    },
                )
                .await
            })
            .expect("read image");

        assert_eq!(value.get("readerKind").and_then(Value::as_str), Some("image_direct"));
        assert_eq!(value.get("imageMime").and_then(Value::as_str), Some("image/webp"));
        assert!(value.get("imageBase64").and_then(Value::as_str).is_some());
        assert!(value.get("content").is_none());
    }

#[cfg(test)]
#[test]
fn builtin_read_file_should_downgrade_bad_image_to_text_notice() {
        let root = std::env::temp_dir().join(format!("eca-read-file-image-bad-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("sample.png");
        std::fs::write(&file, b"not-a-real-png").expect("write bad image");
        let state = test_read_file_state();
        let config = AppConfig {
            selected_api_config_id: "vision-a".to_string(),
            assistant_department_api_config_id: "vision-a".to_string(),
            api_configs: vec![ApiConfig {
                id: "vision-a".to_string(),
                name: "vision-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: true,
                enable_audio: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://example.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                model: "gpt-image".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 0.7,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            ..AppConfig::default()
        };
        state_write_config_cached(&state, &config).expect("write config");

        let value = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(builtin_read_file(
            &state,
            "assistant::conversation-a",
            "vision-a",
            ReadFileRequest {
                path: file.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            },
        ))
        .expect("read image fallback");

        assert_eq!(
            value.get("readerKind").and_then(Value::as_str),
            Some("image_fallback_notice")
        );
        assert!(
            value.get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("未能作为图片输入直接提供给模型")
        );
    }

#[cfg(test)]
#[test]
fn builtin_read_file_should_prefix_truncation_notice_only_when_truncated() {
        let root = std::env::temp_dir().join(format!("eca-read-file-trunc-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("big.txt");
        let long_line = "a".repeat(31_000);
        std::fs::write(&file, long_line).expect("write big text");
        let state = test_read_file_state();
        let value = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(builtin_read_file(
            &state,
            "chat::conv-1",
            "__frontend_tool_preview__",
            ReadFileRequest {
                path: file.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            },
        ))
        .expect("read truncated text");
        let text = value.get("content").and_then(Value::as_str).unwrap_or_default();
        assert!(text.starts_with("Content was truncated to fit within 30000 character limit.\nTo continue reading, use offset="));
    }

#[cfg(test)]
#[test]
fn build_pdf_image_read_result_should_paginate_by_page_start() {
        let path = std::path::PathBuf::from("E:\\docs\\sample.pdf");
        let structured = PdfExtractStructuredResult {
            file_name: "sample.pdf".to_string(),
            total_pages: 3,
            include_images: true,
            pages: vec![
                PdfPageExtractBlock {
                    page_index: 0,
                    text: String::new(),
                    images: vec![PdfRenderedImage {
                        page_index: 0,
                        width: 10,
                        height: 20,
                        bytes_base64: "img0".to_string(),
                        mime: "image/webp".to_string(),
                    }],
                },
                PdfPageExtractBlock {
                    page_index: 1,
                    text: String::new(),
                    images: vec![PdfRenderedImage {
                        page_index: 1,
                        width: 11,
                        height: 21,
                        bytes_base64: "img1".to_string(),
                        mime: "image/webp".to_string(),
                    }],
                },
                PdfPageExtractBlock {
                    page_index: 2,
                    text: String::new(),
                    images: vec![PdfRenderedImage {
                        page_index: 2,
                        width: 12,
                        height: 22,
                        bytes_base64: "img2".to_string(),
                        mime: "image/webp".to_string(),
                    }],
                },
            ],
        };

        let value = build_pdf_image_read_result(
            &path,
            ReadFileDetectedType::Pdf,
            &structured,
            Some(1),
            Some(1),
        );

        assert_eq!(value.get("readerKind").and_then(Value::as_str), Some("pdf_image_direct"));
        assert_eq!(value.get("nextOffset").and_then(Value::as_u64), Some(2));
        let parts = value.get("parts").and_then(Value::as_array).expect("parts");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].get("pageIndex").and_then(Value::as_u64), Some(1));
}
