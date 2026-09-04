const TOOL_OUTPUT_RETENTION_SECS: u64 = 7 * 24 * 60 * 60;
static TOOL_OUTPUT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn tool_output_line_count(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count().saturating_add(1)
}

fn tool_output_directory_from_workspace(llm_workspace_path: &std::path::Path) -> std::path::PathBuf {
    llm_workspace_path.join("tool-output")
}

fn tool_output_directory(state: &AppState) -> std::path::PathBuf {
    tool_output_directory_from_workspace(&state.llm_workspace_path)
}

fn cleanup_expired_tool_outputs(directory: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(directory) else { return; };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        let expired = entry.metadata().ok().and_then(|meta| meta.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age.as_secs() > TOOL_OUTPUT_RETENTION_SECS);
        if expired { let _ = std::fs::remove_file(path); }
    }
}

fn store_full_tool_output_at(directory: &std::path::Path, text: &str) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(&directory).map_err(|err| format!("创建工具输出目录失败: {err}"))?;
    cleanup_expired_tool_outputs(directory);
    let sequence = TOOL_OUTPUT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let file = directory.join(format!("tool_{}_{}.txt", chrono::Utc::now().timestamp_millis(), sequence));
    std::fs::write(&file, text).map_err(|err| format!("保存完整工具输出失败: {err}"))?;
    Ok(file)
}

fn store_full_tool_output(state: &AppState, text: &str) -> Result<std::path::PathBuf, String> {
    store_full_tool_output_at(&tool_output_directory(state), text)
}

fn append_full_tool_output_notice(text: &str, path: &std::path::Path) -> String {
    format!(
        "{text}\n\nFull output saved to: {}\nUse search or ranged reads; do not read the whole file.",
        terminal_path_for_user(path)
    )
}

fn append_full_tool_output_failure_notice(text: &str) -> String {
    format!("{text}\n\nFull output could not be saved.")
}

#[derive(Debug, Clone)]
struct ProviderToolProjection {
    text: String,
    metadata: ProviderToolMetadata,
}

fn project_provider_tool_result(
    state: Option<&AppState>,
    tool_name: &str,
    result: &ProviderToolResult,
) -> ProviderToolProjection {
    if tool_name == "exec" {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&result.output) {
            if value.get("background").and_then(|flag| flag.as_bool()) == Some(true) {
                return ProviderToolProjection {
                    text: result.output.clone(),
                    metadata: result.metadata.clone(),
                };
            }
        }
        let exit_code = result
            .metadata
            .exit_code
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(-1);
        let duration = std::time::Duration::from_millis(
            result.metadata.wall_time_ms.unwrap_or_default(),
        );
        let text = format_exec_output_for_model(
            exit_code,
            duration,
            result.metadata.timed_out,
            &result.output,
            default_tool_output_truncation_policy(),
        );
        return ProviderToolProjection {
            text,
            metadata: result.metadata.clone(),
        };
    }

    let text = result.output.as_str();
    let policy = default_non_shell_tool_output_truncation_policy();
    let mut metadata = result.metadata.clone();
    let projected_output = if text.len() > policy.byte_budget() {
        let (full_path, content) = match state {
            Some(state) => match store_full_tool_output(state, text) {
                Ok(path) => {
                    let content = append_full_tool_output_notice(text, &path);
                    (Some(path), content)
                }
                Err(_) => (None, append_full_tool_output_failure_notice(text)),
            },
            None => (None, text.to_string()),
        };
        metadata.truncated = true;
        metadata.total_output_lines = Some(tool_output_line_count(text));
        if let Some(path) = full_path.as_ref() {
            metadata.output_paths.push(terminal_path_for_user(path));
        }
        truncate_text(&content, policy)
    } else {
        text.to_string()
    };
    ProviderToolProjection {
        text: projected_output,
        metadata,
    }
}

#[cfg(test)]
mod tool_output_store_tests {
    use super::*;

    #[test]
    fn small_output_should_not_be_bounded() {
        let result = ProviderToolResult::text("small");
        assert_eq!(project_provider_tool_result(None, "read", &result).text, "small");
    }

    #[test]
    fn non_shell_projection_should_use_twelve_k_token_middle_truncation_and_keep_media() {
        let output = format!("head\n{}\ntail", "中".repeat(30_000));
        let result = ProviderToolResult {
            output,
            metadata: ProviderToolMetadata::default(),
            parts: vec![
                ProviderToolResultPart::Resource { mime: Some("text/plain".to_string()), uri: Some("mcp://large".to_string()), text: "中".repeat(30_000) },
                ProviderToolResultPart::Image { mime: "image/png".to_string(), data_base64: "abc".to_string(), width: 1, height: 1 },
            ],
            is_error: false,
        };
        let projected = project_provider_tool_result(None, "mcp", &result);
        assert!(projected.text.starts_with("head\n"));
        assert!(projected.text.ends_with("\ntail"));
        assert!(projected.text.contains("tokens truncated"));
        assert!(projected.metadata.truncated);
        assert!(result.parts.iter().any(|part| matches!(part, ProviderToolResultPart::Image { .. })));
    }

    #[test]
    fn non_shell_projection_should_not_truncate_by_line_count_alone() {
        let output = (0..3_000).map(|_| "x").collect::<Vec<_>>().join("\n");
        let result = ProviderToolResult::text(output.clone());

        let projected = project_provider_tool_result(None, "mcp", &result);

        assert_eq!(projected.text, output);
        assert!(!projected.metadata.truncated);
    }

    #[test]
    fn full_output_notice_should_survive_at_truncated_tail() {
        let path = std::path::PathBuf::from("tool-output").join("complete.txt");
        let content = append_full_tool_output_notice(&"x".repeat(100_000), &path);
        let projected = truncate_text(
            &content,
            default_non_shell_tool_output_truncation_policy(),
        );

        assert!(projected.contains("tokens truncated"));
        assert!(projected.ends_with("Use search or ranged reads; do not read the whole file."));
    }

    #[test]
    fn exec_projection_should_passthrough_background_result_json() {
        let output = serde_json::json!({
            "ok": true,
            "background": true,
            "backgroundId": "bg-1",
            "id": "bg-1",
            "status": "running",
            "command": "sleep 10",
            "description": "sleep",
            "cwd": "C:/workspace",
            "mode": "background",
            "logPath": "C:/workspace/logs/bg-1.log"
        })
        .to_string();
        let result = ProviderToolResult::text(output);

        let projected = project_provider_tool_result(None, "exec", &result);

        assert_eq!(projected.text, result.output);
        assert!(projected.text.contains("backgroundId"));
        assert!(projected.text.contains("logPath"));
    }

    #[test]
    fn exec_projection_should_use_codex_wrapper_and_middle_truncation() {
        let result = ProviderToolResult {
            output: (0..10_000)
                .map(|index| format!("line-{index}"))
                .collect::<Vec<_>>()
                .join("\n"),
            metadata: ProviderToolMetadata {
                exit_code: Some(0),
                wall_time_ms: Some(420),
                ..ProviderToolMetadata::default()
            },
            parts: Vec::new(),
            is_error: false,
        };
        let projection = project_provider_tool_result(None, "exec", &result);
        assert!(projection.text.starts_with("Exit code: 0\nWall time: 0.4 seconds"));
        assert!(projection.text.contains("Total output lines: 10000"));
        assert!(projection.text.contains("tokens truncated"));
        assert!(projection.text.contains("line-0"));
        assert!(projection.text.contains("line-9999"));
        assert!(!projection.text.contains("... output truncated ..."));
    }

    #[test]
    fn exec_projection_should_prefix_timeout_content() {
        let result = ProviderToolResult {
            output: "partial".to_string(),
            metadata: ProviderToolMetadata {
                exit_code: Some(-1),
                wall_time_ms: Some(1_500),
                timed_out: true,
                ..ProviderToolMetadata::default()
            },
            parts: Vec::new(),
            is_error: true,
        };
        let projection = project_provider_tool_result(None, "exec", &result);

        assert!(projection.text.starts_with("Exit code: -1\nWall time: 1.5 seconds\nOutput:\n"));
        assert!(projection.text.contains("command timed out after 1500 milliseconds\npartial"));
    }

    #[test]
    fn full_output_should_be_written_to_managed_directory() {
        let root = std::env::temp_dir().join(format!("pai_tool_output_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()));
        let file = store_full_tool_output_at(&root, "complete output").expect("store output");
        assert_eq!(std::fs::read_to_string(&file).expect("read output"), "complete output");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn managed_directory_should_be_inside_llm_workspace() {
        let workspace = std::path::PathBuf::from("root").join("llm-workspace");
        assert_eq!(tool_output_directory_from_workspace(&workspace), workspace.join("tool-output"));
    }
}
