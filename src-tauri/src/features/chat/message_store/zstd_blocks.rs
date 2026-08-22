// ========== zstd 压缩原语（V4 块格式：追加=帧拼接，重写=整块单帧） ==========
//
// V4 存储模型（定案 C，见 .pai/plan/storage/20260822_消息块瘦身压缩规则计划.md）：
// - 追加（新消息、组内子行）= 新内容单独压一个 zstd 帧 → append 到文件尾；追加不解压不重压
// - 重写（replace/归档/迁移/compaction）= 整块单帧
// - 读取 = Decoder 连续解压所有帧至 EOF（zstd crate 原生支持 concatenated frames）
// - 追加前扫帧验证尾部：torn 帧（结构不完整）与孤儿完整帧必须截断，防止坏帧夹在文件中间

/// zstd level 3（实测压缩率 419MB→88MB，4.75x）
const ZSTD_COMPRESSION_LEVEL: i32 = 3;

/// 整块压缩为单帧（重写路径：replace/归档/迁移/compaction）
/// 统一用 bulk::compress：帧头写 content size，追加前扫帧才能不解压对账孤儿帧。
pub(super) fn zstd_compress_block(plain: &[u8]) -> Result<Vec<u8>, String> {
    zstd::bulk::compress(plain, ZSTD_COMPRESSION_LEVEL)
        .map_err(|err| format!("zstd 整块压缩失败: {err}"))
}

/// 整块解压（单帧或多帧拼接；Decoder 默认连续解压所有帧至 EOF，帧边界由 crate 内部处理）
pub(super) fn zstd_decompress_block(compressed: &[u8]) -> Result<Vec<u8>, String> {
    zstd::stream::decode_all(compressed)
        .map_err(|err| format!("zstd 整块解压失败: {err}"))
}

/// 追加帧：新行单独压成 zstd 帧（追加不解压不重压，只压新行 + 文件尾写）
/// 统一用 bulk::compress：帧头写 content size，扫帧可拿到每帧明文长度做孤儿对账。
pub(super) fn zstd_compress_frame(line: &[u8]) -> Result<Vec<u8>, String> {
    zstd::bulk::compress(line, ZSTD_COMPRESSION_LEVEL)
        .map_err(|err| format!("zstd 追加帧压缩失败: {err}"))
}

/// 扫描块文件的所有完整帧边界，返回 (offset, len) 列表。
/// 遇到 torn 帧（结构不完整）或尾部垃圾即停止扫描，调用方据此截断。
/// 只解析帧结构（不解压数据），O(帧数)。
pub(super) fn zstd_scan_frames(data: &[u8]) -> Vec<(u64, u64)> {
    let mut frames = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        match zstd::zstd_safe::find_frame_compressed_size(&data[pos..]) {
            Ok(len) if len > 0 && pos + len <= data.len() => {
                frames.push((pos as u64, len as u64));
                pos += len;
            }
            _ => break,
        }
    }
    frames
}

/// 帧追加前验证块文件尾部，返回应保留的字节长度（截断点）。
///
/// - 结构验证：逐帧扫描，torn 帧（结构不完整）或垃圾尾之后的字节必须截掉，
///   否则坏帧夹在文件中间会导致其后帧读不到。
/// - 孤儿对账：完整帧的明文长度累计（帧头 content size）必须等于
///   `expected_plain_len`（sqlite locator 期望的明文末尾）。若明文超出，
///   说明存在孤儿完整帧（上次追加写成功但 sqlite 未提交），截断到对账帧边界。
///
/// 返回截断点字节长度；全文件有效时返回 data.len()。
pub(super) fn zstd_validate_tail_for_append(
    data: &[u8],
    expected_plain_len: usize,
) -> Result<usize, String> {
    if data.is_empty() {
        if expected_plain_len == 0 {
            return Ok(0);
        }
        return Err(format!(
            "zstd 追加前验证失败：文件为空但期望明文长度={expected_plain_len}"
        ));
    }
    let frames = zstd_scan_frames(data);
    let Some((last_offset, last_len)) = frames.last().copied() else {
        return Err(format!(
            "zstd 追加前验证失败：文件中没有完整 zstd 帧，len={}",
            data.len()
        ));
    };
    let frames_end = (last_offset + last_len) as usize;
    // torn 帧 / 垃圾尾：先把有效区间限制到最后一个完整帧末尾，
    // 再对完整帧区间做明文对账，避免「孤儿完整帧 + 半截 torn 帧」并存时留下孤儿
    let structural_keep = frames_end;
    // 所有帧结构完整；对账明文长度（逐帧 content size 累计，不解压）
    let mut plain_len = 0usize;
    let mut truncate_at = structural_keep;
    let mut reached_expected = false;
    for (offset, len) in &frames {
        let frame = &data[*offset as usize..(*offset + *len) as usize];
        let frame_plain = zstd::zstd_safe::get_frame_content_size(frame)
            .map_err(|err| format!("zstd 追加前验证失败：解析帧明文长度失败: {err}"))?
            .ok_or_else(|| {
                format!("zstd 追加前验证失败：帧缺少 content size，offset={offset}")
            })?;
        plain_len = plain_len
            .checked_add(frame_plain as usize)
            .ok_or_else(|| "zstd 追加前验证失败：明文长度溢出".to_string())?;
        if !reached_expected && plain_len >= expected_plain_len {
            // 明文达到期望的帧末尾是候选截断点（其后的完整帧都是孤儿）
            truncate_at = (*offset + *len) as usize;
            reached_expected = true;
        }
    }
    if plain_len == expected_plain_len {
        Ok(structural_keep)
    } else if plain_len > expected_plain_len {
        Ok(truncate_at)
    } else {
        Err(format!(
            "zstd 追加前验证失败：明文长度不足，actual={plain_len}，expected={expected_plain_len}"
        ))
    }
}

#[cfg(test)]
mod zstd_blocks_tests {
    use super::*;

    fn plain_lines() -> Vec<String> {
        vec![
            "{\"kind\":\"message\",\"message\":{\"id\":\"m1\"}}\n".to_string(),
            "{\"kind\":\"tool\",\"id\":\"m2\",\"call\":{},\"result\":{}}\n".to_string(),
            "{\"kind\":\"assistant\",\"id\":\"m2\",\"parts\":[]}\n".to_string(),
        ]
    }

    #[test]
    fn compress_decompress_roundtrip_should_preserve_plain_bytes() {
        let plain = plain_lines().concat();
        let compressed = zstd_compress_block(plain.as_bytes()).expect("compress");
        let restored = zstd_decompress_block(&compressed).expect("decompress");
        assert_eq!(String::from_utf8(restored).expect("utf8"), plain);
        assert!(compressed.len() < plain.len(), "压缩应显著小于明文");
    }

    #[test]
    fn concatenated_frames_should_decompress_to_concatenated_plain() {
        let lines = plain_lines();
        let mut multi_frame = Vec::new();
        for line in &lines {
            multi_frame.extend_from_slice(&zstd_compress_frame(line.as_bytes()).expect("frame"));
        }
        let restored = zstd_decompress_block(&multi_frame).expect("decompress multi-frame");
        assert_eq!(String::from_utf8(restored).expect("utf8"), lines.concat());
    }

    #[test]
    fn scan_frames_should_find_all_complete_frame_boundaries() {
        let lines = plain_lines();
        let mut multi_frame = Vec::new();
        let mut expected = Vec::new();
        for line in &lines {
            let frame = zstd_compress_frame(line.as_bytes()).expect("frame");
            expected.push((multi_frame.len() as u64, frame.len() as u64));
            multi_frame.extend_from_slice(&frame);
        }
        let frames = zstd_scan_frames(&multi_frame);
        assert_eq!(frames, expected);
        // 扫描的帧尾应正好是文件尾
        let last_end = frames.last().map(|(off, len)| off + len).unwrap_or(0);
        assert_eq!(last_end as usize, multi_frame.len());
    }

    #[test]
    fn scan_frames_should_stop_at_torn_frame_at_tail() {
        let mut multi_frame = Vec::new();
        let line = plain_lines()[0].clone();
        multi_frame.extend_from_slice(&zstd_compress_frame(line.as_bytes()).expect("frame"));
        // 注入半截帧（截断的第二个帧）
        let torn = zstd_compress_frame(plain_lines()[1].as_bytes()).expect("frame");
        multi_frame.extend_from_slice(&torn[..torn.len() / 2]);

        let frames = zstd_scan_frames(&multi_frame);
        assert_eq!(frames.len(), 1, "只应扫到完整帧，torn 帧被排除");
    }

    #[test]
    fn scan_frames_should_stop_at_garbage_tail() {
        let mut multi_frame = Vec::new();
        let line = plain_lines()[0].clone();
        multi_frame.extend_from_slice(&zstd_compress_frame(line.as_bytes()).expect("frame"));
        multi_frame.extend_from_slice(b"not-a-zstd-frame");

        let frames = zstd_scan_frames(&multi_frame);
        assert_eq!(frames.len(), 1, "垃圾尾部不应被识别为帧");
    }

    #[test]
    fn scan_frames_on_single_frame_block_should_find_one_frame() {
        let plain = plain_lines().concat();
        let compressed = zstd_compress_block(plain.as_bytes()).expect("compress");
        let frames = zstd_scan_frames(&compressed);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], (0, compressed.len() as u64));
    }

    #[test]
    fn validate_tail_should_accept_clean_single_frame_block() {
        let plain = plain_lines().concat();
        let compressed = zstd_compress_block(plain.as_bytes()).expect("compress");
        let keep = zstd_validate_tail_for_append(&compressed, plain.len())
            .expect("clean block passes");
        assert_eq!(keep, compressed.len());
    }

    #[test]
    fn validate_tail_should_accept_clean_multi_frame_append() {
        let lines = plain_lines();
        let mut multi_frame = Vec::new();
        let mut plain_total = 0usize;
        for line in &lines {
            multi_frame.extend_from_slice(&zstd_compress_frame(line.as_bytes()).expect("frame"));
            plain_total += line.len();
        }
        let keep = zstd_validate_tail_for_append(&multi_frame, plain_total)
            .expect("clean multi-frame passes");
        assert_eq!(keep, multi_frame.len());
    }

    #[test]
    fn validate_tail_should_truncate_torn_frame_at_tail() {
        let line = plain_lines()[0].clone();
        let mut data = zstd_compress_frame(line.as_bytes()).expect("frame");
        let torn = zstd_compress_frame(plain_lines()[1].as_bytes()).expect("frame");
        data.extend_from_slice(&torn[..torn.len() / 2]);
        let keep = zstd_validate_tail_for_append(&data, line.len())
            .expect("torn tail handled");
        assert_eq!(keep, data.len() - torn.len() / 2, "torn 帧被截断");
    }

    #[test]
    fn validate_tail_should_truncate_orphan_frame_beyond_locator() {
        let lines = plain_lines();
        let mut data = Vec::new();
        let mut covered_plain = 0usize;
        // 前两个帧是 locator 覆盖的（明文 2 行），第三个帧是孤儿（sqlite 未提交）
        for line in &lines[..2] {
            data.extend_from_slice(&zstd_compress_frame(line.as_bytes()).expect("frame"));
            covered_plain += line.len();
        }
        data.extend_from_slice(&zstd_compress_frame(lines[2].as_bytes()).expect("orphan frame"));
        let keep = zstd_validate_tail_for_append(&data, covered_plain)
            .expect("orphan frame handled");
        assert_eq!(
            keep,
            data.len() - zstd_compress_frame(lines[2].as_bytes()).expect("frame len").len(),
            "孤儿帧被截断"
        );
    }

    #[test]
    fn validate_tail_should_reject_missing_plain_data() {
        let line = plain_lines()[0].clone();
        let data = zstd_compress_frame(line.as_bytes()).expect("frame");
        let err = zstd_validate_tail_for_append(&data, line.len() + 100)
            .expect_err("missing plain should fail");
        assert!(err.contains("明文长度不足"), "err={err}");
    }

    #[test]
    fn compress_frame_should_write_content_size_for_orphan_accounting() {
        let line = plain_lines()[0].clone();
        let frame = zstd_compress_frame(line.as_bytes()).expect("frame");
        let content_size = zstd::zstd_safe::get_frame_content_size(&frame)
            .expect("content size readable")
            .expect("content size present");
        assert_eq!(content_size as usize, line.len());
    }
}
