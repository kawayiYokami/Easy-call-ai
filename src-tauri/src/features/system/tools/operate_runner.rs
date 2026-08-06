async fn run_operate_tool(
    input: OperateRequest,
    screenshots_root: &std::path::Path,
    include_base64: bool,
) -> DesktopToolResult<OperateResponse> {
    let started = std::time::Instant::now();
    ensure_dpi_awareness_once();
    let actions = parse_script(&input)?;
    let total_actions = actions.len();
    runtime_log_info(format!(
        "[桌面脚本] 开始，任务=run_operate_tool，total_actions={}，timestamp={}",
        total_actions,
        now_iso()
    ));
    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|err| DesktopToolError::internal_error(format!("创建 Enigo 失败：{err}")))?;
    let mut steps = Vec::<DesktopScriptStepResult>::new();
    let mut latest_screenshot: Option<LatestScreenshotInfo> = None;
    let mut image_mime = None;
    let mut image_base64 = None;
    let mut width = None;
    let mut height = None;

    for action in actions {
        match action {
            DesktopScriptAction::MouseClick { line, button, target, repeat, delay, pre_delay, press } => {
                execute_mouse_click(&mut enigo, button, &target, repeat, delay, pre_delay, press).await?;
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Mouse,
                    summary: format!("mouse click completed, repeat={repeat}"),
                    ok: true,
                    saved_path: None,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=MouseClick，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            DesktopScriptAction::MouseScroll { line, direction, repeat, delay, pre_delay } => {
                execute_mouse_scroll(&mut enigo, direction, repeat, delay, pre_delay).await?;
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Mouse,
                    summary: format!("mouse scroll completed, repeat={repeat}"),
                    ok: true,
                    saved_path: None,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=MouseScroll，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            DesktopScriptAction::Key { line, keys, repeat, delay, pre_delay, press } => {
                execute_key_action(&mut enigo, &keys, line, repeat, delay, pre_delay, press).await?;
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Key,
                    summary: format!("key action completed, combo={}, repeat={repeat}", keys.join("+")),
                    ok: true,
                    saved_path: None,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=Key，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            DesktopScriptAction::Text { line, text, repeat, delay, pre_delay } => {
                execute_text_action(&mut enigo, &text, repeat, delay, pre_delay).await?;
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Text,
                    summary: format!("text input completed, chars={}, repeat={repeat}", text.chars().count()),
                    ok: true,
                    saved_path: None,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=Text，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            DesktopScriptAction::Wait { line, duration } => {
                sleep_duration(duration).await;
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Wait,
                    summary: format!("wait completed, seconds={:.3}", duration.as_secs_f64()),
                    ok: true,
                    saved_path: None,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=Wait，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            DesktopScriptAction::Screenshot { line, mode, save_path, quality } => {
                let (result, mode_name) =
                    execute_screenshot_action(&mode, save_path, quality, screenshots_root, include_base64).await?;
                latest_screenshot = Some(LatestScreenshotInfo {
                    mode: mode_name.clone(),
                    width: result.width,
                    height: result.height,
                    saved_path: result.path.clone(),
                });
                image_mime = Some(result.image_mime.clone());
                image_base64 = result.image_base64.clone();
                width = Some(result.width);
                height = Some(result.height);
                let step = DesktopScriptStepResult {
                    line,
                    kind: DesktopScriptStepKind::Screenshot,
                    summary: format!("screenshot completed, mode={mode_name}"),
                    ok: true,
                    saved_path: result.path,
                };
                runtime_log_info(format!(
                    "[桌面脚本] 步骤完成，任务=run_operate_tool，line={}，kind=Screenshot，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
        }
    }

    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    runtime_log_info(format!(
        "[桌面脚本] 完成，任务=run_operate_tool，executed_count={}，elapsed_ms={}，latest_screenshot={}，image_mime={}，has_image_base64={}，width={}，height={}",
        steps.len(),
        elapsed_ms,
        latest_screenshot
            .as_ref()
            .map(|shot| format!(
                "mode={},width={},height={},saved_path={}",
                shot.mode,
                shot.width,
                shot.height,
                shot.saved_path.as_deref().unwrap_or("-")
            ))
            .unwrap_or_else(|| "none".to_string()),
        image_mime.as_deref().unwrap_or("-"),
        image_base64.as_ref().map(|_| true).unwrap_or(false),
        width
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        height
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    ));

    Ok(OperateResponse {
        ok: true,
        executed_count: steps.len(),
        elapsed_ms,
        steps,
        latest_screenshot,
        image_mime,
        image_base64,
        width,
        height,
    })
}

/// 清空指定会话的 operate 截图临时目录（temp/screenshots/{conversation_id}/），
/// 会话压缩/归档/删除/撤回时调用。返回 (删除文件数, 删除子目录数)；目录不存在时视为已清空。
fn clear_operate_screenshots_temp(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<(usize, usize), String> {
    let dir = app_root_from_data_path(data_path)
        .join("temp")
        .join("screenshots")
        .join(conversation_id);
    let mut removed_files = 0usize;
    let mut removed_dirs = 0usize;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path).map_err(|err| {
                    format!("清理 operate 截图失败（{}）：{err}", path.to_string_lossy())
                })?;
                removed_files = removed_files.saturating_add(1);
            } else if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(|err| {
                    format!("清理 operate 截图子目录失败（{}）：{err}", path.to_string_lossy())
                })?;
                removed_dirs = removed_dirs.saturating_add(1);
            }
        }
    }
    Ok((removed_files, removed_dirs))
}

#[cfg(test)]
mod operate_tool_tests {
    use super::*;

    fn parse_single(script: &str) -> DesktopScriptAction {
        parse_script(&OperateRequest { script: script.to_string(), timeout_ms: None })
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn parse_mouse_click_script() {
        match parse_single("mouse left click @0.50,0.10 repeat=2 delay=0.1") {
            DesktopScriptAction::MouseClick { repeat, .. } => assert_eq!(repeat, 2),
            _ => panic!("expected mouse click"),
        }
    }

    #[test]
    fn parse_key_script() {
        match parse_single("key Control+L") {
            DesktopScriptAction::Key { keys, .. } => assert_eq!(keys, vec!["Control".to_string(), "L".to_string()]),
            _ => panic!("expected key action"),
        }
    }

    #[test]
    fn parse_text_requires_quotes() {
        let err = parse_script(&OperateRequest { script: "text hello".to_string(), timeout_ms: None }).unwrap_err();
        assert!(err.message.contains("第 1 行 text"));
        assert!(err.message.contains("双引号"));
    }

    #[test]
    fn screenshot_save_requires_absolute_path() {
        let err = parse_script(&OperateRequest { script: r#"screenshot save="tmp/shot.webp""#.to_string(), timeout_ms: None }).unwrap_err();
        assert!(err.message.contains("第 1 行 screenshot"));
        assert!(err.message.contains("绝对路径"));
    }

    #[test]
    fn mouse_coordinates_must_be_normalized() {
        let err = parse_script(&OperateRequest { script: "mouse left click @1.2,0.5".to_string(), timeout_ms: None }).unwrap_err();
        assert!(err.message.contains("第 1 行 mouse"));
        assert!(err.message.contains("0.0~1.0"));
    }

    #[test]
    fn screenshot_region_should_parse() {
        match parse_single("screenshot region=@0.10,0.10,0.80,0.60") {
            DesktopScriptAction::Screenshot { mode: ScreenshotModeSpec::Region(_), .. } => {}
            _ => panic!("expected screenshot region"),
        }
    }

    #[test]
    fn clear_operate_screenshots_temp_should_only_remove_screenshots() {
        let root = std::env::temp_dir().join("easy-call-ai-clear-operate-test");
        let _ = std::fs::remove_dir_all(&root);
        let data_path = root.join("config");
        let conversation_id = "convo-test-1";
        let screenshots = root
            .join("temp")
            .join("screenshots")
            .join(conversation_id);
        let sub = screenshots.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(screenshots.join("a.webp"), b"a").unwrap();
        std::fs::write(sub.join("b.webp"), b"b").unwrap();
        // 其他会话的截图不应被清理
        let other = root.join("temp").join("screenshots").join("convo-other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("keep.webp"), b"keep").unwrap();
        let records = root.join("temp").join("apply_patch").join("records");
        let blobs = root.join("temp").join("apply_patch").join("blobs");
        std::fs::create_dir_all(&records).unwrap();
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(records.join("r.json"), b"{}").unwrap();
        std::fs::write(blobs.join("b.json"), b"{}").unwrap();

        let (files, dirs) = clear_operate_screenshots_temp(&data_path, conversation_id).unwrap();
        assert_eq!(files, 1, "top-level screenshot file should be removed");
        assert_eq!(dirs, 1, "nested screenshot dir should be removed recursively");
        assert!(!screenshots.join("a.webp").exists());
        assert!(!sub.exists(), "nested dir with inner file should be gone");
        assert!(
            !screenshots.join("sub").join("b.webp").exists(),
            "inner screenshot file should be gone with its dir"
        );
        assert!(
            other.join("keep.webp").exists(),
            "other conversation screenshots must survive"
        );
        assert!(
            records.join("r.json").exists(),
            "apply_patch records must survive cleanup"
        );
        assert!(
            blobs.join("b.json").exists(),
            "apply_patch blobs must survive cleanup"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
