const LOCAL_IMAGE_THUMBNAIL_MAX_EDGE: u32 = 1080;
const LOCAL_IMAGE_REMOTE_MAX_EDGE: u32 = 2160;
const LOCAL_IMAGE_WEBP_QUALITY: f32 = 82.0;
const LOCAL_IMAGE_FALLBACK_MIME: &str = "image/webp";
const LOCAL_IMAGE_MAX_SOURCE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum PersistedInlineMessageSegment {
    Text { text: String },
    Meme {
        name: String,
        category: String,
        mime: String,
        relative_path: String,
        bytes_base64: String,
    },
    LocalImage {
        path: String,
        file_name: String,
        mime: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
}

#[derive(Debug, Clone)]
struct LocalImageRenderData {
    mime: String,
    bytes: Vec<u8>,
    original_width: u32,
    original_height: u32,
    output_width: u32,
    output_height: u32,
}

#[derive(Debug, Clone)]
struct LocalImageFileInfo {
    mime: String,
    width: u32,
    height: u32,
}

fn local_image_workspace_root(state: &AppState) -> PathBuf {
    configured_workspace_root_path(state).unwrap_or_else(|_| state.llm_workspace_path.clone())
}

fn local_image_resolve_path(state: &AppState, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    let direct = PathBuf::from(trimmed);
    if direct.is_absolute() {
        direct
    } else {
        local_image_workspace_root(state).join(direct)
    }
}

fn local_image_file_name(path: &std::path::Path, alt: Option<&str>) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            alt.map(str::trim)
                .filter(|value| !value.is_empty())
                .map(sanitize_download_file_name)
        })
        .unwrap_or_else(|| "image.webp".to_string())
}

fn local_image_mime_from_format(format: image::ImageFormat) -> Option<&'static str> {
    match format {
        image::ImageFormat::Png => Some("image/png"),
        image::ImageFormat::Jpeg => Some("image/jpeg"),
        image::ImageFormat::Gif => Some("image/gif"),
        image::ImageFormat::WebP => Some("image/webp"),
        image::ImageFormat::Bmp => Some("image/bmp"),
        _ => None,
    }
}

fn local_image_format_from_mime(mime: &str) -> Option<image::ImageFormat> {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/png" => Some(image::ImageFormat::Png),
        "image/jpeg" | "image/jpg" => Some(image::ImageFormat::Jpeg),
        "image/gif" => Some(image::ImageFormat::Gif),
        "image/webp" => Some(image::ImageFormat::WebP),
        "image/bmp" => Some(image::ImageFormat::Bmp),
        _ => None,
    }
}

fn local_image_guess_mime_from_path(path: &std::path::Path) -> String {
    media_mime_from_path(path)
        .and_then(local_image_format_from_mime)
        .and_then(local_image_mime_from_format)
        .unwrap_or(LOCAL_IMAGE_FALLBACK_MIME)
        .to_string()
}

fn local_image_detect_format(raw: &[u8], path: &std::path::Path) -> Result<(image::ImageFormat, String), String> {
    let format = image::guess_format(raw).map_err(|err| {
        format!(
            "识别本地图片格式失败: path={}, err={err}",
            path.to_string_lossy()
        )
    })?;
    let Some(mime) = local_image_mime_from_format(format) else {
        return Err(format!(
            "本地图片格式不支持: path={}, format={format:?}",
            path.to_string_lossy()
        ));
    };
    Ok((format, mime.to_string()))
}

fn local_image_read_raw(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("本地图片不存在或无法读取元数据: path={}, err={err}", path.to_string_lossy()))?;
    if !metadata.is_file() {
        return Err(format!("本地图片路径不是文件: {}", path.to_string_lossy()));
    }
    if metadata.len() > LOCAL_IMAGE_MAX_SOURCE_BYTES {
        return Err(format!(
            "本地图片过大: path={}, bytes={}, max_bytes={}",
            path.to_string_lossy(),
            metadata.len(),
            LOCAL_IMAGE_MAX_SOURCE_BYTES
        ));
    }
    std::fs::read(path)
        .map_err(|err| format!("读取本地图片失败: path={}, err={err}", path.to_string_lossy()))
}

fn local_image_decode_dynamic(raw: &[u8], path: &std::path::Path) -> Result<(image::DynamicImage, String), String> {
    let (format, mime) = local_image_detect_format(raw, path)?;
    let image = image::load_from_memory_with_format(raw, format).map_err(|err| {
        format!(
            "解码本地图片失败: path={}, format={format:?}, err={err}",
            path.to_string_lossy()
        )
    })?;
    Ok((image, mime))
}

fn local_image_file_info(path: &std::path::Path) -> Result<LocalImageFileInfo, String> {
    let raw = local_image_read_raw(path)?;
    let (image, mime) = local_image_decode_dynamic(&raw, path)?;
    Ok(LocalImageFileInfo {
        mime,
        width: image.width(),
        height: image.height(),
    })
}

fn local_image_resized_dimensions(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_edge || longest == 0 {
        return (width.max(1), height.max(1));
    }
    let new_width = ((width as u64 * max_edge as u64 + longest as u64 / 2) / longest as u64)
        .max(1) as u32;
    let new_height = ((height as u64 * max_edge as u64 + longest as u64 / 2) / longest as u64)
        .max(1) as u32;
    (new_width, new_height)
}

fn local_image_encode_webp(image: image::DynamicImage, max_edge: u32) -> Result<LocalImageRenderData, String> {
    let original_width = image.width();
    let original_height = image.height();
    let (target_width, target_height) =
        local_image_resized_dimensions(original_width, original_height, max_edge);
    let resized = if (target_width, target_height) == (original_width, original_height) {
        image
    } else {
        image.resize_exact(target_width, target_height, image::imageops::FilterType::Lanczos3)
    };
    let encoder = webp::Encoder::from_image(&resized)
        .map_err(|err| format!("初始化本地图片 WebP 编码器失败: {err}"))?;
    let encoded = encoder.encode(LOCAL_IMAGE_WEBP_QUALITY);
    Ok(LocalImageRenderData {
        mime: LOCAL_IMAGE_FALLBACK_MIME.to_string(),
        bytes: (&*encoded).to_vec(),
        original_width,
        original_height,
        output_width: resized.width(),
        output_height: resized.height(),
    })
}

fn local_image_read_for_display(path: &std::path::Path, max_edge: u32) -> Result<LocalImageRenderData, String> {
    let raw = local_image_read_raw(path)?;
    let (image, mime) = local_image_decode_dynamic(&raw, path)?;
    let original_width = image.width();
    let original_height = image.height();
    if original_width.max(original_height) <= max_edge {
        return Ok(LocalImageRenderData {
            mime,
            bytes: raw,
            original_width,
            original_height,
            output_width: original_width,
            output_height: original_height,
        });
    }
    local_image_encode_webp(image, max_edge)
}

fn local_image_read_original(path: &std::path::Path) -> Result<LocalImageRenderData, String> {
    let raw = local_image_read_raw(path)?;
    let (image, mime) = local_image_decode_dynamic(&raw, path)?;
    Ok(LocalImageRenderData {
        mime,
        bytes: raw,
        original_width: image.width(),
        original_height: image.height(),
        output_width: image.width(),
        output_height: image.height(),
    })
}

#[derive(Debug, Clone)]
struct LocalImageReference {
    start: usize,
    end: usize,
    raw_path: String,
    alt: Option<String>,
}

fn local_image_markdown_is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut cursor = index;
    let mut backslash_count = 0usize;
    while cursor > 0 {
        cursor -= 1;
        if bytes.get(cursor) != Some(&b'\\') {
            break;
        }
        backslash_count += 1;
    }
    backslash_count % 2 == 1
}

fn local_image_skip_ascii_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while let Some(value) = bytes.get(cursor) {
        if !matches!(value, b' ' | b'\t' | b'\r' | b'\n') {
            break;
        }
        cursor += 1;
    }
    cursor
}

fn local_image_unescape_markdown_text(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.peek().copied() {
                if matches!(
                    next,
                    '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#'
                        | '+' | '-' | '.' | '!' | '<' | '>' | '|'
                ) {
                    out.push(next);
                    let _ = chars.next();
                    continue;
                }
            }
        }
        out.push(ch);
    }
    out
}

fn local_image_strip_markdown_destination_title(value: &str, quote: char) -> Option<String> {
    if !value.ends_with(quote) {
        return None;
    }
    let marker = format!(" {quote}");
    value
        .rsplit_once(marker.as_str())
        .map(|(path, _title)| path.trim().to_string())
        .filter(|path| !path.is_empty())
}

fn local_image_clean_markdown_destination_path(value: &str) -> String {
    let trimmed = value.trim();
    if local_image_is_windows_drive_path(trimmed) || trimmed.starts_with("\\\\") {
        return trimmed.to_string();
    }
    local_image_unescape_markdown_text(trimmed)
}

fn local_image_extract_markdown_destination_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix('<') {
        if let Some(end) = rest.find('>') {
            return local_image_clean_markdown_destination_path(&rest[..end]);
        }
    }
    let without_title = local_image_strip_markdown_destination_title(trimmed, '"')
        .or_else(|| local_image_strip_markdown_destination_title(trimmed, '\''))
        .unwrap_or_else(|| trimmed.to_string());
    local_image_clean_markdown_destination_path(&without_title)
}

fn local_image_is_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn local_image_uri_scheme(value: &str) -> Option<&str> {
    let colon = value.find(':')?;
    let scheme = &value[..colon];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return None;
    }
    Some(scheme)
}

fn local_image_path_from_file_url(raw: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(raw).ok()?;
    if parsed.scheme() != "file" {
        return None;
    }
    let decoded_path = urlencoding::decode(parsed.path()).ok()?.to_string();
    let host = parsed.host_str().map(str::trim).unwrap_or_default();
    if !host.is_empty() && host != "localhost" {
        if cfg!(windows) {
            return Some(format!(
                "\\\\{}{}",
                host,
                decoded_path.replace('/', "\\")
            ));
        }
        return Some(format!("//{}{}", host, decoded_path));
    }
    if decoded_path.len() >= 3 {
        let bytes = decoded_path.as_bytes();
        if bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
            return Some(decoded_path[1..].to_string());
        }
    }
    Some(decoded_path)
}

fn local_image_path_from_markdown_destination(raw: &str) -> Option<String> {
    let value = local_image_extract_markdown_destination_path(raw);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower.starts_with("mailto:")
        || lower.starts_with("javascript:")
    {
        return None;
    }
    if lower.starts_with("file:") {
        return local_image_path_from_file_url(trimmed);
    }
    if local_image_uri_scheme(trimmed).is_some() && !local_image_is_windows_drive_path(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

fn local_image_find_markdown_alt_end(text: &str, cursor: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = cursor;
    while index < bytes.len() {
        if bytes[index] == b']' && !local_image_markdown_is_escaped(bytes, index) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn local_image_find_angle_destination_end(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut index = cursor + 1;
    while index < bytes.len() {
        if bytes[index] == b'>' && !local_image_markdown_is_escaped(bytes, index) {
            let close = local_image_skip_ascii_whitespace(bytes, index + 1);
            if bytes.get(close) == Some(&b')') {
                return Some((index + 1, close + 1));
            }
            return None;
        }
        index += 1;
    }
    None
}

fn local_image_find_plain_destination_end(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = cursor;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 1 < bytes.len() {
            index += 2;
            continue;
        }
        if bytes[index] == b'(' {
            depth += 1;
        } else if bytes[index] == b')' {
            if depth == 0 {
                return Some((index, index + 1));
            }
            depth = depth.saturating_sub(1);
        }
        index += 1;
    }
    None
}

fn local_image_find_markdown_destination_end(text: &str, open_paren: usize) -> Option<(usize, usize, usize)> {
    let bytes = text.as_bytes();
    let start = local_image_skip_ascii_whitespace(bytes, open_paren + 1);
    if bytes.get(start) == Some(&b'<') {
        let (end, token_end) = local_image_find_angle_destination_end(text, start)?;
        return Some((start, end, token_end));
    }
    let (mut end, token_end) = local_image_find_plain_destination_end(text, start)?;
    while end > start && matches!(bytes[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
        end -= 1;
    }
    Some((start, end, token_end))
}

fn local_image_find_next_markdown_reference(text: &str, cursor: usize) -> Option<LocalImageReference> {
    let bytes = text.as_bytes();
    let mut search_from = cursor;
    while search_from + 2 <= text.len() {
        let start = text.get(search_from..)?.find("![").map(|idx| search_from + idx)?;
        let alt_start = start + 2;
        let Some(alt_end) = local_image_find_markdown_alt_end(text, alt_start) else {
            return None;
        };
        let open_paren = local_image_skip_ascii_whitespace(bytes, alt_end + 1);
        if bytes.get(open_paren) != Some(&b'(') {
            search_from = start + 2;
            continue;
        }
        let Some((dest_start, dest_end, token_end)) =
            local_image_find_markdown_destination_end(text, open_paren)
        else {
            search_from = start + 2;
            continue;
        };
        let raw_destination = &text[dest_start..dest_end];
        let Some(raw_path) = local_image_path_from_markdown_destination(raw_destination) else {
            search_from = start + 2;
            continue;
        };
        let alt = local_image_unescape_markdown_text(&text[alt_start..alt_end])
            .trim()
            .to_string();
        return Some(LocalImageReference {
            start,
            end: token_end,
            raw_path,
            alt: (!alt.is_empty()).then_some(alt),
        });
    }
    None
}

fn local_image_find_next_reference(text: &str, cursor: usize) -> Option<LocalImageReference> {
    local_image_find_next_markdown_reference(text, cursor)
}

fn local_image_segment_from_reference(
    state: &AppState,
    reference: &LocalImageReference,
) -> PersistedInlineMessageSegment {
    let resolved = local_image_resolve_path(state, &reference.raw_path);
    let info = local_image_file_info(&resolved).ok();
    PersistedInlineMessageSegment::LocalImage {
        path: resolved.to_string_lossy().to_string(),
        file_name: local_image_file_name(&resolved, reference.alt.as_deref()),
        mime: info
            .as_ref()
            .map(|value| value.mime.clone())
            .unwrap_or_else(|| local_image_guess_mime_from_path(&resolved)),
        alt: reference.alt.clone(),
        width: info.as_ref().map(|value| value.width),
        height: info.as_ref().map(|value| value.height),
    }
}

fn resolve_text_to_local_image_segments(
    state: &AppState,
    text: &str,
) -> Vec<PersistedInlineMessageSegment> {
    let mut segments = Vec::<PersistedInlineMessageSegment>::new();
    let mut cursor = 0usize;
    let mut text_cursor = 0usize;
    while cursor < text.len() {
        let Some(reference) = local_image_find_next_reference(text, cursor) else {
            break;
        };
        if reference.start > text_cursor {
            segments.push(PersistedInlineMessageSegment::Text {
                text: text[text_cursor..reference.start].to_string(),
            });
        }
        segments.push(local_image_segment_from_reference(state, &reference));
        cursor = reference.end;
        text_cursor = reference.end;
    }
    if text_cursor < text.len() {
        segments.push(PersistedInlineMessageSegment::Text {
            text: text[text_cursor..].to_string(),
        });
    }
    if segments.is_empty() {
        segments.push(PersistedInlineMessageSegment::Text {
            text: text.to_string(),
        });
    }
    segments
}

fn inline_segment_from_meme_segment(segment: PersistedMemeSegment) -> PersistedInlineMessageSegment {
    match segment {
        PersistedMemeSegment::Text { text } => PersistedInlineMessageSegment::Text { text },
        PersistedMemeSegment::Meme {
            name,
            category,
            mime,
            relative_path,
            bytes_base64,
        } => PersistedInlineMessageSegment::Meme {
            name,
            category,
            mime,
            relative_path,
            bytes_base64,
        },
    }
}

fn resolve_text_to_persisted_inline_segments(
    state: &AppState,
    text: &str,
    seed_source: &str,
) -> Result<Option<Vec<PersistedInlineMessageSegment>>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let local_segments = resolve_text_to_local_image_segments(state, text);
    let has_local_image = local_segments
        .iter()
        .any(|segment| matches!(segment, PersistedInlineMessageSegment::LocalImage { .. }));
    let mut has_meme = false;
    let mut out = Vec::<PersistedInlineMessageSegment>::new();
    for segment in local_segments {
        match segment {
            PersistedInlineMessageSegment::Text { text } => {
                if let Some(meme_segments) =
                    resolve_text_to_persisted_meme_segments(state, &text, seed_source)?
                {
                    if meme_segments
                        .iter()
                        .any(|item| matches!(item, PersistedMemeSegment::Meme { .. }))
                    {
                        has_meme = true;
                    }
                    out.extend(meme_segments.into_iter().map(inline_segment_from_meme_segment));
                } else if !text.is_empty() {
                    out.push(PersistedInlineMessageSegment::Text { text });
                }
            }
            other => out.push(other),
        }
    }
    if has_local_image || has_meme {
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

fn provider_meta_inline_segments(meta: Option<&Value>) -> Option<Vec<PersistedInlineMessageSegment>> {
    let raw = meta?.as_object()?.get("inlineSegments")?.clone();
    serde_json::from_value::<Vec<PersistedInlineMessageSegment>>(raw).ok()
}

#[cfg(test)]
mod local_image_reference_tests {
    use super::*;

    #[test]
    fn markdown_image_reference_should_parse_windows_path_and_alt() {
        let input = "看这张：![结果图](E:/tmp/result.png) 好了";
        let reference = local_image_find_next_markdown_reference(input, 0)
            .expect("markdown image reference");

        assert_eq!(reference.raw_path, "E:/tmp/result.png");
        assert_eq!(reference.alt.as_deref(), Some("结果图"));
        assert_eq!(&input[reference.start..reference.end], "![结果图](E:/tmp/result.png)");
    }

    #[test]
    fn markdown_image_reference_should_skip_remote_images() {
        let input = "![badge](https://example.com/a.png) ![local](outputs/a.png)";
        let reference = local_image_find_next_markdown_reference(input, 0)
            .expect("local markdown image reference");

        assert_eq!(reference.raw_path, "outputs/a.png");
        assert_eq!(reference.alt.as_deref(), Some("local"));
    }

    #[test]
    fn custom_image_token_should_not_be_parsed() {
        assert!(local_image_find_next_reference("{{image:E:/tmp/a.png|图}}", 0).is_none());
        assert!(local_image_find_next_reference("{{img:E:/tmp/a.png|图}}", 0).is_none());
    }

    #[test]
    fn markdown_image_destination_should_support_file_url_and_angle_path() {
        assert_eq!(
            local_image_path_from_markdown_destination("<E:/tmp/result image.png>").as_deref(),
            Some("E:/tmp/result image.png")
        );
        assert_eq!(
            local_image_path_from_markdown_destination("file:///E:/tmp/result%20image.png").as_deref(),
            Some("E:/tmp/result image.png")
        );
    }

    #[test]
    fn markdown_image_destination_should_preserve_windows_backslash_separators() {
        assert_eq!(
            local_image_path_from_markdown_destination(r"E:\[tmp]\result.png").as_deref(),
            Some(r"E:\[tmp]\result.png")
        );
    }
}
