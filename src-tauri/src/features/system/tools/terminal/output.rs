const APPROX_BYTES_PER_TOKEN: usize = 4;
const DEFAULT_TOOL_OUTPUT_TOKENS: usize = 10_000;
const NON_SHELL_TOOL_OUTPUT_POLICY_MULTIPLIER: f64 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum TruncationPolicy {
    Bytes(usize),
    Tokens(usize),
}

impl TruncationPolicy {
    fn byte_budget(self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Tokens(tokens) => approx_bytes_for_tokens(tokens),
        }
    }
}

impl std::ops::Mul<f64> for TruncationPolicy {
    type Output = Self;

    fn mul(self, multiplier: f64) -> Self::Output {
        match self {
            Self::Bytes(bytes) => Self::Bytes((bytes as f64 * multiplier).ceil() as usize),
            Self::Tokens(tokens) => Self::Tokens((tokens as f64 * multiplier).ceil() as usize),
        }
    }
}

fn default_tool_output_truncation_policy() -> TruncationPolicy {
    TruncationPolicy::Tokens(DEFAULT_TOOL_OUTPUT_TOKENS)
}

fn default_non_shell_tool_output_truncation_policy() -> TruncationPolicy {
    default_tool_output_truncation_policy() * NON_SHELL_TOOL_OUTPUT_POLICY_MULTIPLIER
}

fn prefix_cap(mut buffer: Vec<u8>, max_bytes: Option<usize>) -> Vec<u8> {
    if let Some(max_bytes) = max_bytes {
        if buffer.len() > max_bytes {
            buffer.truncate(max_bytes);
        }
    }
    buffer
}

fn aggregate_output(stdout: &[u8], stderr: &[u8], max_bytes: Option<usize>) -> Vec<u8> {
    let Some(max_bytes) = max_bytes else {
        let mut output = Vec::with_capacity(stdout.len().saturating_add(stderr.len()));
        output.extend_from_slice(stdout);
        output.extend_from_slice(stderr);
        return output;
    };

    let total = stdout.len().saturating_add(stderr.len());
    if total <= max_bytes {
        let mut output = Vec::with_capacity(total);
        output.extend_from_slice(stdout);
        output.extend_from_slice(stderr);
        return output;
    }

    let want_stdout = stdout.len().min(max_bytes / 3);
    let stderr_take = stderr.len().min(max_bytes.saturating_sub(want_stdout));
    let remaining = max_bytes.saturating_sub(want_stdout + stderr_take);
    let stdout_take =
        want_stdout + remaining.min(stdout.len().saturating_sub(want_stdout));

    let mut output = Vec::with_capacity(stdout_take + stderr_take);
    output.extend_from_slice(&stdout[..stdout_take]);
    output.extend_from_slice(&stderr[..stderr_take]);
    output
}

fn build_content_with_timeout(
    timed_out: bool,
    duration: std::time::Duration,
    aggregated_output: &str,
) -> String {
    if timed_out {
        format!(
            "command timed out after {} milliseconds\n{}",
            duration.as_millis(),
            aggregated_output
        )
    } else {
        aggregated_output.to_string()
    }
}

fn format_exec_output_for_model(
    exit_code: i32,
    duration: std::time::Duration,
    timed_out: bool,
    aggregated_output: &str,
    policy: TruncationPolicy,
) -> String {
    let duration_seconds = (duration.as_secs_f32() * 10.0).round() / 10.0;
    let content = build_content_with_timeout(timed_out, duration, aggregated_output);
    let total_lines = content.lines().count();
    let body = truncate_text(&content, policy);

    let mut sections = vec![
        format!("Exit code: {exit_code}"),
        format!("Wall time: {duration_seconds} seconds"),
    ];
    if total_lines != body.lines().count() {
        sections.push(format!("Total output lines: {total_lines}"));
    }
    sections.push("Output:".to_string());
    sections.push(body);
    sections.join("\n")
}

fn truncate_text(content: &str, policy: TruncationPolicy) -> String {
    if content.len() <= policy.byte_budget() {
        return content.to_string();
    }
    match policy {
        TruncationPolicy::Bytes(bytes) => truncate_middle_chars(content, bytes),
        TruncationPolicy::Tokens(tokens) => truncate_middle_with_token_budget(content, tokens).0,
    }
}

fn truncate_middle_chars(text: &str, max_bytes: usize) -> String {
    truncate_with_byte_estimate(text, max_bytes, false)
}

fn truncate_middle_with_token_budget(text: &str, max_tokens: usize) -> (String, Option<u64>) {
    if text.is_empty() {
        return (String::new(), None);
    }
    if max_tokens > 0 && text.len() <= approx_bytes_for_tokens(max_tokens) {
        return (text.to_string(), None);
    }

    let truncated = truncate_with_byte_estimate(
        text,
        approx_bytes_for_tokens(max_tokens),
        true,
    );
    let total_tokens = u64::try_from(approx_token_count(text)).unwrap_or(u64::MAX);
    if truncated == text {
        (truncated, None)
    } else {
        (truncated, Some(total_tokens))
    }
}

fn truncate_with_byte_estimate(text: &str, max_bytes: usize, use_tokens: bool) -> String {
    if text.is_empty() {
        return String::new();
    }

    let total_chars = text.chars().count();
    if max_bytes == 0 {
        return format_truncation_marker(
            use_tokens,
            removed_units(use_tokens, text.len(), total_chars),
        );
    }
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let (left_budget, right_budget) = split_budget(max_bytes);
    let (removed_chars, prefix, suffix) = split_string(text, left_budget, right_budget);
    let marker = format_truncation_marker(
        use_tokens,
        removed_units(
            use_tokens,
            text.len().saturating_sub(max_bytes),
            removed_chars,
        ),
    );
    format!("{prefix}{marker}{suffix}")
}

fn approx_token_count(text: &str) -> usize {
    text.len()
        .saturating_add(APPROX_BYTES_PER_TOKEN.saturating_sub(1))
        / APPROX_BYTES_PER_TOKEN
}

fn approx_bytes_for_tokens(tokens: usize) -> usize {
    tokens.saturating_mul(APPROX_BYTES_PER_TOKEN)
}

fn approx_tokens_from_byte_count(bytes: usize) -> u64 {
    let bytes = bytes as u64;
    bytes.saturating_add((APPROX_BYTES_PER_TOKEN as u64).saturating_sub(1))
        / APPROX_BYTES_PER_TOKEN as u64
}

fn split_string(text: &str, beginning_bytes: usize, end_bytes: usize) -> (usize, &str, &str) {
    if text.is_empty() {
        return (0, "", "");
    }

    let tail_start_target = text.len().saturating_sub(end_bytes);
    let mut prefix_end = 0usize;
    let mut suffix_start = text.len();
    let mut removed_chars = 0usize;
    let mut suffix_started = false;

    for (index, character) in text.char_indices() {
        let character_end = index + character.len_utf8();
        if character_end <= beginning_bytes {
            prefix_end = character_end;
            continue;
        }
        if index >= tail_start_target {
            if !suffix_started {
                suffix_start = index;
                suffix_started = true;
            }
            continue;
        }
        removed_chars = removed_chars.saturating_add(1);
    }

    if suffix_start < prefix_end {
        suffix_start = prefix_end;
    }
    (removed_chars, &text[..prefix_end], &text[suffix_start..])
}

fn split_budget(budget: usize) -> (usize, usize) {
    let left = budget / 2;
    (left, budget - left)
}

fn format_truncation_marker(use_tokens: bool, removed_count: u64) -> String {
    if use_tokens {
        format!("…{removed_count} tokens truncated…")
    } else {
        format!("…{removed_count} chars truncated…")
    }
}

fn removed_units(use_tokens: bool, removed_bytes: usize, removed_chars: usize) -> u64 {
    if use_tokens {
        approx_tokens_from_byte_count(removed_bytes)
    } else {
        u64::try_from(removed_chars).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod exec_output_tests {
    use super::*;

    #[test]
    fn small_output_keeps_stdout_then_stderr_without_truncation() {
        let aggregated = aggregate_output(b"stdout\n", b"stderr\n", Some(DEFAULT_OUTPUT_BYTES_CAP));
        let formatted = format_exec_output_for_model(
            0,
            std::time::Duration::from_millis(1_250),
            false,
            &String::from_utf8_lossy(&aggregated),
            TruncationPolicy::Tokens(10_000),
        );

        assert!(formatted.ends_with("Output:\nstdout\nstderr\n"));
        assert!(!formatted.contains("truncated"));
        assert!(!formatted.contains("Total output lines:"));
    }

    #[test]
    fn aggregate_output_prefers_stderr_when_both_streams_contend() {
        let stdout = vec![b'a'; DEFAULT_OUTPUT_BYTES_CAP];
        let stderr = vec![b'b'; DEFAULT_OUTPUT_BYTES_CAP];
        let aggregated = aggregate_output(&stdout, &stderr, Some(DEFAULT_OUTPUT_BYTES_CAP));
        let stdout_cap = DEFAULT_OUTPUT_BYTES_CAP / 3;

        assert_eq!(aggregated.len(), DEFAULT_OUTPUT_BYTES_CAP);
        assert!(aggregated[..stdout_cap].iter().all(|byte| *byte == b'a'));
        assert!(aggregated[stdout_cap..].iter().all(|byte| *byte == b'b'));
    }

    #[test]
    fn aggregate_output_rebalances_unused_stderr_capacity_to_stdout() {
        let stdout = vec![b'a'; DEFAULT_OUTPUT_BYTES_CAP];
        let stderr = vec![b'b'; 17];
        let aggregated = aggregate_output(&stdout, &stderr, Some(DEFAULT_OUTPUT_BYTES_CAP));
        let stdout_take = DEFAULT_OUTPUT_BYTES_CAP - stderr.len();

        assert_eq!(aggregated.len(), DEFAULT_OUTPUT_BYTES_CAP);
        assert!(aggregated[..stdout_take].iter().all(|byte| *byte == b'a'));
        assert_eq!(&aggregated[stdout_take..], stderr.as_slice());
    }

    #[test]
    fn prefix_cap_keeps_only_the_prefix() {
        assert_eq!(prefix_cap(b"abcdef".to_vec(), Some(4)), b"abcd");
    }

    #[test]
    fn format_exec_output_adds_fixed_metadata_and_timeout_prefix() {
        let formatted = format_exec_output_for_model(
            -1,
            std::time::Duration::from_millis(1_500),
            true,
            "partial output",
            TruncationPolicy::Bytes(1024),
        );

        assert!(formatted.starts_with("Exit code: -1\nWall time: 1.5 seconds\nOutput:\n"));
        assert!(formatted.contains("command timed out after 1500 milliseconds\npartial output"));
    }

    #[test]
    fn format_exec_output_reports_lines_only_when_model_body_is_truncated() {
        let content = (0..100)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let formatted = format_exec_output_for_model(
            0,
            std::time::Duration::from_millis(100),
            false,
            &content,
            TruncationPolicy::Tokens(10),
        );

        assert!(formatted.contains("Total output lines: 100"));
        assert!(formatted.contains("tokens truncated"));
        assert!(formatted.contains("line-0"));
        assert!(formatted.contains("line-99"));
    }

    #[test]
    fn middle_truncation_preserves_utf8_head_and_tail() {
        let content = "😀😀😀😀😀😀😀😀😀😀\nsecond line with text\n";
        let truncated = truncate_text(content, TruncationPolicy::Bytes(20));

        assert_eq!(truncated, "😀😀…21 chars truncated…with text\n");
    }

    #[test]
    fn truncate_text_returns_original_within_budget() {
        assert_eq!(
            truncate_text("small", TruncationPolicy::Tokens(10)),
            "small"
        );
    }

    #[test]
    fn non_shell_policy_should_scale_default_budget_by_one_point_two() {
        assert_eq!(
            default_tool_output_truncation_policy(),
            TruncationPolicy::Tokens(10_000)
        );
        assert_eq!(
            default_non_shell_tool_output_truncation_policy(),
            TruncationPolicy::Tokens(12_000)
        );
        assert_eq!(
            TruncationPolicy::Bytes(10_001) * 1.2,
            TruncationPolicy::Bytes(12_002)
        );
    }
}
