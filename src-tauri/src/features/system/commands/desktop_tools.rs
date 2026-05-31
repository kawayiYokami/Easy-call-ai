#[tauri::command]
async fn desktop_screenshot(input: ScreenshotRequest) -> Result<ScreenshotResponse, String> {
    run_screenshot_tool(input)
        .await
        .map_err(|err| to_tool_err_string(&err))
}

const NATIVE_NOTIFICATION_BODY_MAX_CHARS: usize = 180;
#[cfg(target_os = "windows")]
const NATIVE_NOTIFICATION_SOUND_DEFAULT: &str = "Default";

fn native_notification_text_excerpt(raw: &str, max_chars: usize) -> String {
    let normalized = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn send_native_notification(
    app: &AppHandle,
    title: &str,
    body: &str,
    play_sound: bool,
) -> Result<(), String> {
    use tauri_plugin_notification::{NotificationExt, PermissionState};

    let normalized_title = title.trim();
    let normalized_body = body.trim();
    if normalized_title.is_empty() {
        return Err("通知标题不能为空".to_string());
    }
    if normalized_body.is_empty() {
        return Err("通知正文不能为空".to_string());
    }

    let notifications = app.notification();
    let permission_before = notifications
        .permission_state()
        .map_err(|err| format!("读取通知权限失败：{err}"))?;
    let permission_after = match permission_before {
        PermissionState::Prompt | PermissionState::PromptWithRationale => notifications
            .request_permission()
            .map_err(|err| format!("请求通知权限失败：{err}"))?,
        state => state,
    };

    if permission_after == PermissionState::Denied {
        return Err("系统通知权限已被拒绝，请先在系统设置里允许通知。".to_string());
    }

    let mut builder = notifications
        .builder()
        .title(normalized_title)
        .body(normalized_body);

    #[cfg(target_os = "windows")]
    {
        if play_sound {
            builder = builder.sound(NATIVE_NOTIFICATION_SOUND_DEFAULT);
        }
    }

    builder
        .show()
        .map_err(|err| format!("发送原生通知失败：{err}"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeNotificationDemoResult {
    permission_before: String,
    permission_after: String,
    title: String,
    body: String,
    sent_at: String,
}

#[tauri::command]
fn demo_send_native_notification(app: AppHandle) -> Result<NativeNotificationDemoResult, String> {
    use tauri_plugin_notification::NotificationExt;

    let sent_at = now_local_rfc3339();
    let title = "PAI Demo 原生通知".to_string();
    let body = format!("这是从 Demo 页发出的测试通知。时间：{sent_at}");
    let permission_before = app
        .notification()
        .permission_state()
        .map_err(|err| format!("读取通知权限失败：{err}"))?;
    send_native_notification(&app, &title, &body, true)?;
    let permission_after = app
        .notification()
        .permission_state()
        .map_err(|err| format!("读取通知权限失败：{err}"))?;

    eprintln!(
        "[通知Demo] 完成，permission_before={}，permission_after={}，sent_at={}",
        permission_before,
        permission_after,
        sent_at
    );

    Ok(NativeNotificationDemoResult {
        permission_before: permission_before.to_string(),
        permission_after: permission_after.to_string(),
        title,
        body,
        sent_at,
    })
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XcapToolInput {
    method: String,
    #[serde(default)]
    args: Value,
}

fn xcap_arg_u32(args: &Value, key: &str) -> Result<u32, String> {
    let v = args
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| to_tool_err_string(&DesktopToolError::invalid_params(format!("{key} is required"))))?;
    u32::try_from(v).map_err(|_| {
        to_tool_err_string(&DesktopToolError::invalid_params(format!("{key} is out of range")))
    })
}

fn xcap_arg_i32(args: &Value, key: &str) -> Result<i32, String> {
    let v = args
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| to_tool_err_string(&DesktopToolError::invalid_params(format!("{key} is required"))))?;
    i32::try_from(v).map_err(|_| {
        to_tool_err_string(&DesktopToolError::invalid_params(format!("{key} is out of range")))
    })
}

fn xcap_optional_webp_quality(args: &Value) -> f32 {
    args.get("webpQuality")
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .unwrap_or(default_webp_quality())
}

fn xcap_optional_save_path(args: &Value) -> Option<String> {
    args.get("savePath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

#[tauri::command]
async fn xcap(input: XcapToolInput) -> Result<Value, String> {
    let method = input.method.trim().to_string();
    let args = input.args;
    let out = match method.as_str() {
        "list_windows" => {
            let data = xcap_list_windows_infos().map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        "list_monitors" => {
            let data = xcap_list_monitors_infos().map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        "capture_focused_window" => {
            let req = ScreenshotRequest {
                mode: ScreenshotMode::Desktop,
                monitor_id: None,
                region: None,
                save_path: xcap_optional_save_path(&args),
                webp_quality: xcap_optional_webp_quality(&args),
            };
            let data = run_capture_window_tool(req, None)
                .map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        "capture_window" => {
            let window_id = xcap_arg_u32(&args, "windowId")?;
            let req = ScreenshotRequest {
                mode: ScreenshotMode::Desktop,
                monitor_id: None,
                region: None,
                save_path: xcap_optional_save_path(&args),
                webp_quality: xcap_optional_webp_quality(&args),
            };
            let data = run_capture_window_tool(req, Some(window_id))
                .map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        "capture_monitor" => {
            let monitor_id = xcap_arg_u32(&args, "monitorId")?;
            let req = ScreenshotRequest {
                mode: ScreenshotMode::Monitor,
                monitor_id: Some(monitor_id),
                region: None,
                save_path: xcap_optional_save_path(&args),
                webp_quality: xcap_optional_webp_quality(&args),
            };
            let data = run_screenshot_tool(req)
                .await
                .map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        "capture_region" => {
            let x = xcap_arg_i32(&args, "x")?;
            let y = xcap_arg_i32(&args, "y")?;
            let width = xcap_arg_u32(&args, "width")?;
            let height = xcap_arg_u32(&args, "height")?;
            let monitor_id = args
                .get("monitorId")
                .and_then(Value::as_u64)
                .and_then(|v| u32::try_from(v).ok());
            let req = ScreenshotRequest {
                mode: ScreenshotMode::Region,
                monitor_id,
                region: Some(ScreenBounds {
                    x,
                    y,
                    width,
                    height,
                }),
                save_path: xcap_optional_save_path(&args),
                webp_quality: xcap_optional_webp_quality(&args),
            };
            let data = run_screenshot_tool(req)
                .await
                .map_err(|err| to_tool_err_string(&err))?;
            serde_json::json!({
                "ok": true,
                "method": method,
                "data": data
            })
        }
        _ => {
            return Err(to_tool_err_string(&DesktopToolError::invalid_params(
                "unsupported xcap method",
            )));
        }
    };
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalSelfCheckStepResult {
    name: String,
    ok: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostRuntimePrerequisites {
    git_installed: bool,
    node_installed: bool,
    rg_installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostRuntimePrerequisiteInstallResult {
    kind: String,
    installed: bool,
    message: String,
}

fn command_exists_in_path(name: &str) -> bool {
    let raw = name.trim();
    if raw.is_empty() {
        return false;
    }
    let path_value = match std::env::var_os("PATH") {
        Some(value) => value,
        None => return false,
    };
    let name_path = Path::new(raw);
    let mut candidates = Vec::<String>::new();
    if name_path.extension().is_some() {
        candidates.push(raw.to_string());
    } else {
        candidates.push(raw.to_string());
        #[cfg(target_os = "windows")]
        {
            if let Some(pathext) = std::env::var_os("PATHEXT") {
                for ext in pathext.to_string_lossy().split(';') {
                    let trimmed = ext.trim();
                    if !trimmed.is_empty() {
                        candidates.push(format!("{raw}{trimmed}"));
                    }
                }
            } else {
                candidates.push(format!("{raw}.exe"));
            }
        }
    }

    for dir in std::env::split_paths(&path_value) {
        for candidate in &candidates {
            if dir.join(candidate).is_file() {
                return true;
            }
        }
    }
    false
}

fn host_runtime_prerequisite_installed(kind: &str) -> Result<bool, String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "git" => {
            if command_exists_in_path("git") {
                return Ok(true);
            }
            #[cfg(target_os = "windows")]
            {
                return Ok([
                    r"C:\Program Files\Git\cmd\git.exe",
                    r"C:\Program Files\Git\bin\git.exe",
                    r"C:\Program Files (x86)\Git\cmd\git.exe",
                    r"C:\Program Files (x86)\Git\bin\git.exe",
                ]
                .iter()
                .any(|path| Path::new(path).is_file()));
            }
            #[cfg(not(target_os = "windows"))]
            {
                Ok(false)
            }
        }
        "node" => {
            if command_exists_in_path("node") {
                return Ok(true);
            }
            #[cfg(target_os = "windows")]
            {
                return Ok([
                    r"C:\Program Files\nodejs\node.exe",
                    r"C:\Program Files (x86)\nodejs\node.exe",
                ]
                .iter()
                .any(|path| Path::new(path).is_file()));
            }
            #[cfg(not(target_os = "windows"))]
            {
                Ok(false)
            }
        }
        "rg" | "ripgrep" => Ok(command_exists_in_path("rg")),
        other => Err(format!("不支持的运行时依赖：{other}")),
    }
}

#[tauri::command]
fn get_host_runtime_prerequisites() -> HostRuntimePrerequisites {
    HostRuntimePrerequisites {
        git_installed: host_runtime_prerequisite_installed("git").unwrap_or(false),
        node_installed: host_runtime_prerequisite_installed("node").unwrap_or(false),
        rg_installed: host_runtime_prerequisite_installed("rg").unwrap_or(false),
    }
}

#[cfg(target_os = "windows")]
fn winget_package_id_for_host_runtime(kind: &str) -> Result<&'static str, String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "git" => Ok("Git.Git"),
        "node" => Ok("OpenJS.NodeJS.LTS"),
        other => Err(format!("不支持的运行时依赖：{other}")),
    }
}

#[cfg(target_os = "windows")]
fn run_winget_host_runtime_install(kind: &str, elevated: bool) -> Result<(), String> {
    let package_id = winget_package_id_for_host_runtime(kind)?;
    let winget_args = [
        "install",
        "--id",
        package_id,
        "-e",
        "--source",
        "winget",
        "--silent",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
    ];

    let status = if elevated {
        let quoted_args = winget_args
            .iter()
            .map(|arg| format!("'{}'", arg.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let script = format!(
            "$p = Start-Process -FilePath 'winget' -Verb RunAs -Wait -PassThru -ArgumentList @({quoted_args}); if ($null -eq $p) {{ exit 1 }}; exit $p.ExitCode"
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|err| format!("拉起管理员安装失败：{err}"))?
    } else {
        std::process::Command::new("winget")
            .args(winget_args)
            .status()
            .map_err(|err| format!("启动 winget 失败：{err}"))?
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "winget 安装退出码：{}",
            status.code().map_or_else(|| "未知".to_string(), |code| code.to_string())
        ))
    }
}

#[cfg(target_os = "windows")]
fn install_host_runtime_prerequisite_sync(
    kind: String,
) -> Result<HostRuntimePrerequisiteInstallResult, String> {
    let normalized = kind.trim().to_ascii_lowercase();
    if host_runtime_prerequisite_installed(&normalized)? {
        return Ok(HostRuntimePrerequisiteInstallResult {
            kind: normalized,
            installed: true,
            message: "已经检测到依赖，无需安装。".to_string(),
        });
    }
    if !command_exists_in_path("winget") {
        return Err("当前系统未检测到 winget，已改为打开官方下载页。".to_string());
    }

    if let Err(first_err) = run_winget_host_runtime_install(&normalized, false) {
        eprintln!("[依赖安装] 普通安装失败，准备请求管理员权限，kind={}，error={}", normalized, first_err);
        run_winget_host_runtime_install(&normalized, true)?;
    }

    if host_runtime_prerequisite_installed(&normalized)? {
        Ok(HostRuntimePrerequisiteInstallResult {
            kind: normalized,
            installed: true,
            message: "安装完成。".to_string(),
        })
    } else {
        Err("安装流程结束后仍未检测到依赖，请重启 PAI 或手动安装。".to_string())
    }
}

#[tauri::command]
async fn install_host_runtime_prerequisite(
    kind: String,
) -> Result<HostRuntimePrerequisiteInstallResult, String> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || install_host_runtime_prerequisite_sync(kind))
            .await
            .map_err(|err| format!("安装任务执行失败：{err}"))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let normalized = kind.trim().to_ascii_lowercase();
        Err(format!(
            "{normalized} 暂不支持一键安装，请使用系统包管理器或官方下载页安装。"
        ))
    }
}

#[tauri::command]
async fn terminal_self_check(state: State<'_, AppState>) -> Result<Value, String> {
    let session_id = normalize_terminal_tool_session_id("ui-terminal-self-check");
    let runtime_shell = terminal_shell_for_state(&state);
    #[cfg(target_os = "windows")]
    if runtime_shell.kind == "missing-terminal-shell" {
        return Ok(serde_json::json!({
            "ok": false,
            "blockedReason": "missing_terminal_shell",
            "message": "No supported shell was detected on Windows. Install Git and use Git Bash: https://git-scm.com/downloads",
            "shellKind": runtime_shell.kind,
            "shellPath": runtime_shell.path,
            "steps": []
        }));
    }

    let root_path = terminal_session_root_canonical(&state, &session_id)?;
    let cwd = resolve_terminal_cwd(&state, &session_id, None)?;
    let allowed_project_roots = terminal_allowed_project_roots_canonical(&state)?
        .iter()
        .map(|v| v.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let steps = if runtime_shell.kind.starts_with("powershell") {
        vec![
            "Get-Location",
            "$PSVersionTable.PSVersion.ToString()",
            "Get-Command git -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source",
            "Get-Command pwsh -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source",
        ]
    } else {
        vec![
            "pwd",
            "echo $0",
            "git --version",
            "bash --version",
            "command -v git",
            "command -v bash",
        ]
    };

    let mut results = Vec::<TerminalSelfCheckStepResult>::new();
    for step in steps {
        match sandbox_execute_command(&state, &session_id, step, &cwd, 15_000, false).await {
            Ok(execution) => {
                let (stdout, _) = truncate_terminal_output(&execution.stdout);
                let (stderr, _) = truncate_terminal_output(&execution.stderr);
                results.push(TerminalSelfCheckStepResult {
                    name: step.to_string(),
                    ok: execution.ok,
                    exit_code: execution.exit_code,
                    stdout,
                    stderr,
                    duration_ms: execution.duration_ms,
                });
            }
            Err(err) => {
                results.push(TerminalSelfCheckStepResult {
                    name: step.to_string(),
                    ok: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: err,
                    duration_ms: 0,
                });
                break;
            }
        }
    }

    let ok = results.iter().all(|item| item.ok);
    Ok(serde_json::json!({
        "ok": ok,
        "rootPath": root_path.to_string_lossy().to_string(),
        "cwd": cwd.to_string_lossy().to_string(),
        "shellKind": runtime_shell.kind,
        "shellPath": runtime_shell.path,
        "allowedProjectRoots": allowed_project_roots,
        "steps": results,
    }))
}

#[tauri::command]
fn list_terminal_shell_candidates(state: State<'_, AppState>) -> Result<Value, String> {
    let (preferred_kind, current, options) = terminal_shell_candidates_for_ui(&state);
    Ok(serde_json::json!({
        "preferredKind": preferred_kind,
        "currentKind": current.kind,
        "currentPath": current.path,
        "options": options,
    }))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveTerminalApprovalInput {
    request_id: String,
    approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatShellWorkspaceInput {
    api_config_id: String,
    agent_id: String,
    conversation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveChatShellWorkspacesInput {
    api_config_id: String,
    agent_id: String,
    conversation_id: Option<String>,
    #[serde(default)]
    workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    autonomous_mode: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellWorkspacePathInput {
    #[serde(default)]
    workspace_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrateWorkspaceDirectoryInput {
    old_path: String,
    new_path: String,
    task_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceMigrationProgressPayload {
    task_id: String,
    stage: String,
    processed: usize,
    total: usize,
    current_path: Option<String>,
    message: String,
    done: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatShellWorkspaceOutput {
    session_id: String,
    workspace_name: String,
    root_path: String,
    workspaces: Vec<ShellWorkspaceConfig>,
    autonomous_mode: bool,
}

fn resolve_chat_tool_session_id(
    state: &AppState,
    api_config_id: &str,
    agent_id: &str,
    conversation_id: Option<&str>,
) -> Result<String, String> {
    let api_id = api_config_id.trim();
    let agent = agent_id.trim();
    if api_id.is_empty() {
        return Err("apiConfigId is required.".to_string());
    }
    if agent.is_empty() {
        return Err("agentId is required.".to_string());
    }

    let config = state_read_config_cached(state)?;
    if !config.api_configs.iter().any(|v| v.id == api_id) {
        return Err(format!("Selected API config '{api_id}' not found."));
    }
    let agents = state_read_agents_cached(state)?;
    if !agents.iter().any(|v| v.id == agent && !v.is_built_in_user) {
        return Err(format!("Selected agent '{agent}' not found."));
    }

    let session_id = inflight_chat_key(agent, conversation_id);
    Ok(normalize_terminal_tool_session_id(&session_id))
}

fn resolve_chat_workspace_conversation_id(
    state: &AppState,
    agent_id: &str,
    conversation_id: Option<&str>,
) -> Result<String, String> {
    if let Some(conversation_id) = conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(conversation_id.to_string());
    }
    conversation_service()
        .resolve_latest_foreground_conversation_id(state, agent_id)
        .and_then(|value| {
            value.ok_or_else(|| "当前没有可用的活跃会话，需要提供 conversationId。".to_string())
        })
}

fn apply_conversation_chat_workspace_changes(
    state: &AppState,
    conversation_id: &str,
    shell_workspace_path: Option<Option<String>>,
    shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
    shell_autonomous_mode: Option<bool>,
) -> Result<Conversation, String> {
    if delegate_runtime_thread_conversation_get(state, conversation_id)?.is_some() {
        let next_path = shell_workspace_path.clone();
        let next_workspaces = shell_workspaces.clone();
        delegate_runtime_thread_modify(state, conversation_id, move |thread| {
            let original_path = thread.conversation.shell_workspace_path.clone();
            let original_workspaces = thread.conversation.shell_workspaces.clone();
            let original_autonomous_mode = thread.conversation.shell_autonomous_mode;
            if let Some(value) = next_path.clone() {
                thread.conversation.shell_workspace_path = value;
            }
            if let Some(value) = next_workspaces.clone() {
                thread.conversation.shell_workspaces = value;
            }
            if let Some(value) = shell_autonomous_mode {
                thread.conversation.shell_autonomous_mode = value;
            }
            if thread.conversation.shell_workspace_path.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_some()
                && terminal_workspace_path_from_conversation(state, &thread.conversation).is_none()
            {
                thread.conversation.shell_workspace_path = None;
            }
            if thread.conversation.shell_workspace_path == original_path
                && thread.conversation.shell_workspaces == original_workspaces
                && thread.conversation.shell_autonomous_mode == original_autonomous_mode
            {
                return Ok(());
            }
            mark_prompt_cache_rebuild_for_system_environment_by_conversation(
                state,
                conversation_id,
            );
            Ok(())
        })?;
        return delegate_runtime_thread_conversation_get_any(state, conversation_id)?
            .ok_or_else(|| format!("指定会话不存在：{conversation_id}"));
    }

    let updated = conversation_service().update_persisted_conversation_shell_workspace(
        state,
        conversation_id,
        shell_workspace_path,
        shell_workspaces,
        shell_autonomous_mode,
    )?;
    mark_prompt_cache_rebuild_for_system_environment_by_conversation(state, conversation_id);
    Ok(updated)
}

fn workspace_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|v| v.to_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn resolve_workspace_display_name_for_conversation(
    state: &AppState,
    conversation: Option<&Conversation>,
    root: &Path,
) -> String {
    let root_key = normalize_terminal_path_for_compare(root);
    if let Ok(workspaces) = terminal_allowed_workspaces_for_conversation_canonical(state, conversation) {
        for ws in workspaces {
            if normalize_terminal_path_for_compare(&ws.path) == root_key {
                return ws.name;
            }
        }
    }
    workspace_name_from_path(root)
}

fn build_chat_shell_workspace_list(
    state: &AppState,
    conversation: Option<&Conversation>,
) -> Vec<ShellWorkspaceConfig> {
    terminal_allowed_workspaces_for_conversation_canonical(state, conversation)
        .unwrap_or_default()
        .into_iter()
        .map(|workspace| ShellWorkspaceConfig {
            id: workspace.id,
            name: workspace.name,
            path: workspace.path.to_string_lossy().to_string(),
            level: workspace.level,
            access: workspace.access,
            built_in: workspace.built_in,
        })
        .collect()
}

fn build_chat_shell_workspace_output(
    state: &AppState,
    session_id: String,
    conversation: Option<&Conversation>,
    root: PathBuf,
) -> ChatShellWorkspaceOutput {
    ChatShellWorkspaceOutput {
        session_id,
        workspace_name: resolve_workspace_display_name_for_conversation(state, conversation, &root),
        root_path: root.to_string_lossy().to_string(),
        workspaces: build_chat_shell_workspace_list(state, conversation),
        autonomous_mode: conversation.map(|value| value.shell_autonomous_mode).unwrap_or(false),
    }
}

fn open_shell_path_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let explorer_path = resolved.to_string_lossy().replace('/', "\\");
        std::process::Command::new("explorer")
            .arg(explorer_path.as_str())
            .status()
            .map_err(|err| format!("Open in explorer failed: {err}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .status()
            .map_err(|err| format!("Open in Finder failed: {err}"))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|err| format!("Open in file manager failed: {err}"))?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Open in file manager is not supported on this platform".to_string())
}

fn open_shell_terminal_at_path(state: &AppState, path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|err| format!("解析目录失败 ({}): {err}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("不是目录：{}", canonical.display()));
    }
    let shell = terminal_shell_for_state(state);
    if shell.kind == "missing-terminal-shell" || shell.path.trim().is_empty() {
        return Err("未检测到可用 Shell，请先在设置中配置终端 Shell。".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let cwd = terminal_strip_windows_verbatim_prefix(&canonical.to_string_lossy());
        let title = terminal_powershell_escape_literal("PAI Shell");
        let shell_path = terminal_powershell_escape_literal(&shell.path);
        let cwd_text = terminal_powershell_escape_literal(&cwd);
        let args = if shell.kind == "git-bash" {
            format!("@('--login','-i')")
        } else if matches!(shell.kind.as_str(), "powershell7" | "powershell5") {
            format!("@('-NoLogo','-NoExit','-Command','Set-Location -LiteralPath ''{cwd_text}''')")
        } else {
            "@()".to_string()
        };
        let script = format!(
            "Start-Process -FilePath '{shell_path}' -WorkingDirectory '{cwd_text}' -WindowStyle Normal -ArgumentList {args} -Verb Open -PassThru | Out-Null; $host.UI.RawUI.WindowTitle = '{title}'"
        );
        let mut command = std::process::Command::new("powershell");
        command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script]);
        terminal_apply_windows_utf8_env(&mut command);
        command
            .spawn()
            .map_err(|err| format!("打开 Shell 失败：{err}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let cwd = canonical.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("tell application \"Terminal\" to do script \"cd \\\"{cwd}\\\" && exec \\\"{}\\\"\"", shell.path.replace('"', "\\\""));
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|err| format!("打开 Shell 失败：{err}"))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for terminal in ["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
            let mut command = std::process::Command::new(terminal);
            command.current_dir(&canonical);
            if terminal == "xterm" {
                command.args(["-e", &shell.path]);
            } else {
                command.args(["--", &shell.path]);
            }
            if command.spawn().is_ok() {
                return Ok(());
            }
        }
        return Err("未检测到可用图形终端。".to_string());
    }

    #[allow(unreachable_code)]
    Err("当前平台不支持打开 Shell。".to_string())
}

fn resolve_requested_shell_workspace_root(
    state: &AppState,
    requested: Option<&str>,
    create_if_missing: bool,
) -> Result<PathBuf, String> {
    let root = if let Some(raw) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        let normalized = normalize_terminal_path_input_for_current_platform(raw);
        if normalized.is_empty() {
            return Err("工作区路径不能为空".to_string());
        }
        let candidate = PathBuf::from(&normalized);
        if candidate.is_absolute() {
            candidate
        } else {
            state.llm_workspace_path.join(candidate)
        }
    } else {
        configured_workspace_root_path(state)?
    };

    if create_if_missing {
        return ensure_workspace_root_ready(&root);
    }
    let canonical = root
        .canonicalize()
        .map_err(|err| format!("解析工作区路径失败 ({}): {err}", root.display()))?;
    if !canonical.is_dir() {
        return Err(format!("工作区目录不存在：{}", canonical.display()));
    }
    Ok(canonical)
}

const WORKSPACE_MIGRATION_EVENT: &str = "easy-call:workspace-migration-progress";

fn emit_workspace_migration_progress(
    app: &AppHandle,
    payload: &WorkspaceMigrationProgressPayload,
) {
    let _ = app.emit(WORKSPACE_MIGRATION_EVENT, payload);
}

fn collect_workspace_entries(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(path)
        .map_err(|err| format!("读取目录失败 ({}): {err}", path.display()))?
    {
        let entry = entry.map_err(|err| format!("读取目录项失败 ({}): {err}", path.display()))?;
        let child = entry.path();
        out.push(child.clone());
        if child.is_dir() {
            collect_workspace_entries(&child, out)?;
        }
    }
    Ok(())
}

fn copy_workspace_entry_recursive(
    from: &Path,
    to: &Path,
    app: &AppHandle,
    task_id: &str,
    processed: &mut usize,
    total: usize,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(from)
        .map_err(|err| format!("读取路径信息失败 ({}): {err}", from.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("暂不支持迁移符号链接：{}", from.display()));
    }
    if metadata.is_dir() {
        if to.exists() && !to.is_dir() {
            return Err(format!("迁移失败，目标路径已存在同名文件：{}", to.display()));
        }
        fs::create_dir_all(to)
            .map_err(|err| format!("创建目录失败 ({}): {err}", to.display()))?;
        *processed += 1;
        emit_workspace_migration_progress(
            app,
            &WorkspaceMigrationProgressPayload {
                task_id: task_id.to_string(),
                stage: "copying".to_string(),
                processed: *processed,
                total,
                current_path: Some(to.to_string_lossy().to_string()),
                message: "正在复制目录".to_string(),
                done: false,
                error: None,
            },
        );
        for entry in fs::read_dir(from)
            .map_err(|err| format!("读取目录失败 ({}): {err}", from.display()))?
        {
            let entry = entry.map_err(|err| format!("读取目录项失败 ({}): {err}", from.display()))?;
            let child_from = entry.path();
            let child_to = to.join(entry.file_name());
            copy_workspace_entry_recursive(&child_from, &child_to, app, task_id, processed, total)?;
        }
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建目标目录失败 ({}): {err}", parent.display()))?;
    }
    if to.exists() {
        return Err(format!("迁移失败，目标路径已存在同名文件：{}", to.display()));
    }
    fs::copy(from, to).map_err(|err| {
        format!(
            "复制文件失败 ({} -> {}): {err}",
            from.display(),
            to.display()
        )
    })?;
    let target_meta = fs::metadata(to)
        .map_err(|err| format!("读取复制后文件失败 ({}): {err}", to.display()))?;
    if target_meta.len() != metadata.len() {
        return Err(format!("复制校验失败，文件大小不一致：{}", to.display()));
    }
    *processed += 1;
    emit_workspace_migration_progress(
        app,
        &WorkspaceMigrationProgressPayload {
            task_id: task_id.to_string(),
            stage: "copying".to_string(),
            processed: *processed,
            total,
            current_path: Some(to.to_string_lossy().to_string()),
            message: "正在复制文件".to_string(),
            done: false,
            error: None,
        },
    );
    Ok(())
}

fn remove_workspace_entry_recursive(
    path: &Path,
    app: &AppHandle,
    task_id: &str,
    processed: &mut usize,
    total: usize,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("读取路径信息失败 ({}): {err}", path.display()))?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(|err| format!("读取目录失败 ({}): {err}", path.display()))?
        {
            let entry = entry.map_err(|err| format!("读取目录项失败 ({}): {err}", path.display()))?;
            remove_workspace_entry_recursive(&entry.path(), app, task_id, processed, total)?;
        }
        fs::remove_dir(path)
            .map_err(|err| format!("删除目录失败 ({}): {err}", path.display()))?;
    } else {
        fs::remove_file(path)
            .map_err(|err| format!("删除文件失败 ({}): {err}", path.display()))?;
    }
    *processed += 1;
    emit_workspace_migration_progress(
        app,
        &WorkspaceMigrationProgressPayload {
            task_id: task_id.to_string(),
            stage: "deleting".to_string(),
            processed: *processed,
            total,
            current_path: Some(path.to_string_lossy().to_string()),
            message: "正在清理旧目录".to_string(),
            done: false,
            error: None,
        },
    );
    Ok(())
}

#[tauri::command]
fn open_chat_shell_workspace_dir(
    input: Option<ShellWorkspacePathInput>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = resolve_requested_shell_workspace_root(
        &state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    open_shell_path_in_file_manager(&root)?;
    Ok(shell_workspace_display_path(&root))
}

fn shell_workspace_display_path(path: &Path) -> String {
    #[cfg(target_os = "windows")]
    {
        let raw = path.to_string_lossy();
        if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = raw.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
        raw.to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string_lossy().to_string()
    }
}

#[tauri::command]
fn reset_chat_shell_workspace(
    input: Option<ShellWorkspacePathInput>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = resolve_requested_shell_workspace_root(
        &state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    ensure_workspace_mcp_layout_at_root(&root)?;
    ensure_workspace_skills_layout_at_root(&root)?;
    ensure_workspace_private_organization_layout_at_root(&root)?;
    Ok(shell_workspace_display_path(&root))
}

#[tauri::command]
fn get_default_chat_shell_workspace_path(state: State<'_, AppState>) -> Result<String, String> {
    let root = terminal_default_session_root_canonical(&state)?;
    Ok(shell_workspace_display_path(&root))
}

#[tauri::command]
async fn migrate_shell_workspace_directory(
    input: MigrateWorkspaceDirectoryInput,
    app: AppHandle,
) -> Result<String, String> {
    let old_root = PathBuf::from(normalize_terminal_path_input_for_current_platform(&input.old_path));
    let new_root = PathBuf::from(normalize_terminal_path_input_for_current_platform(&input.new_path));
    if input.old_path.trim().is_empty() || input.new_path.trim().is_empty() {
        return Err("工作区迁移路径不能为空".to_string());
    }
    let task_id = input.task_id.trim().to_string();
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let old_root = old_root
            .canonicalize()
            .map_err(|err| format!("解析旧工作区失败 ({}): {err}", old_root.display()))?;
        let new_root = ensure_workspace_root_ready(&new_root)?;
        emit_workspace_migration_progress(
            &app_handle,
            &WorkspaceMigrationProgressPayload {
                task_id: task_id.clone(),
                stage: "scanning".to_string(),
                processed: 0,
                total: 0,
                current_path: Some(old_root.to_string_lossy().to_string()),
                message: "正在扫描旧工作区".to_string(),
                done: false,
                error: None,
            },
        );
        let mut entries = Vec::<PathBuf>::new();
        collect_workspace_entries(&old_root, &mut entries)?;
        let total = entries.len();
        emit_workspace_migration_progress(
            &app_handle,
            &WorkspaceMigrationProgressPayload {
                task_id: task_id.clone(),
                stage: "copying".to_string(),
                processed: 0,
                total,
                current_path: Some(old_root.to_string_lossy().to_string()),
                message: "开始复制工作区内容".to_string(),
                done: false,
                error: None,
            },
        );
        let mut copy_processed = 0usize;
        for entry in fs::read_dir(&old_root)
            .map_err(|err| format!("读取旧工作区失败 ({}): {err}", old_root.display()))?
        {
            let entry = entry.map_err(|err| format!("读取旧工作区目录项失败 ({}): {err}", old_root.display()))?;
            let from = entry.path();
            let to = new_root.join(entry.file_name());
            copy_workspace_entry_recursive(&from, &to, &app_handle, &task_id, &mut copy_processed, total)?;
        }
        emit_workspace_migration_progress(
            &app_handle,
            &WorkspaceMigrationProgressPayload {
                task_id: task_id.clone(),
                stage: "deleting".to_string(),
                processed: 0,
                total,
                current_path: Some(old_root.to_string_lossy().to_string()),
                message: "开始清理旧工作区".to_string(),
                done: false,
                error: None,
            },
        );
        let mut delete_processed = 0usize;
        for entry in fs::read_dir(&old_root)
            .map_err(|err| format!("读取旧工作区失败 ({}): {err}", old_root.display()))?
        {
            let entry = entry.map_err(|err| format!("读取旧工作区目录项失败 ({}): {err}", old_root.display()))?;
            remove_workspace_entry_recursive(&entry.path(), &app_handle, &task_id, &mut delete_processed, total)?;
        }
        runtime_log_info(format!(
            "[工作区迁移] 完成: old='{}', new='{}', moved_entries={}",
            old_root.display(),
            new_root.display(),
            total
        ));
        emit_workspace_migration_progress(
            &app_handle,
            &WorkspaceMigrationProgressPayload {
                task_id: task_id.clone(),
                stage: "completed".to_string(),
                processed: total,
                total,
                current_path: Some(new_root.to_string_lossy().to_string()),
                message: "工作区迁移完成".to_string(),
                done: true,
                error: None,
            },
        );
        Ok(shell_workspace_display_path(&new_root))
    })
    .await
    .map_err(|err| format!("工作区迁移任务执行失败: {err}"))?
    .map_err(|err: String| {
        emit_workspace_migration_progress(
            &app,
            &WorkspaceMigrationProgressPayload {
                task_id: input.task_id.trim().to_string(),
                stage: "failed".to_string(),
                processed: 0,
                total: 0,
                current_path: None,
                message: "工作区迁移失败".to_string(),
                done: true,
                error: Some(err.clone()),
            },
        );
        err
    })
}

#[tauri::command]
fn get_chat_shell_workspace(
    input: ChatShellWorkspaceInput,
    state: State<'_, AppState>,
) -> Result<ChatShellWorkspaceOutput, String> {
    let session_id =
        resolve_chat_tool_session_id(
            &state,
            &input.api_config_id,
            &input.agent_id,
            input.conversation_id.as_deref(),
        )?;
    let conversation = terminal_session_conversation(&state, &session_id)?;
    let root = terminal_session_root_canonical(&state, &session_id)?;
    Ok(build_chat_shell_workspace_output(
        &state,
        session_id,
        conversation.as_ref(),
        root,
    ))
}

#[tauri::command]
fn update_chat_shell_workspace_layout(
    input: SaveChatShellWorkspacesInput,
    state: State<'_, AppState>,
) -> Result<ChatShellWorkspaceOutput, String> {
    let session_id =
        resolve_chat_tool_session_id(
            &state,
            &input.api_config_id,
            &input.agent_id,
            input.conversation_id.as_deref(),
        )?;
    let conversation_id = resolve_chat_workspace_conversation_id(
        &state,
        &input.agent_id,
        input.conversation_id.as_deref(),
    )?;
    let normalized_workspaces = normalize_conversation_shell_workspaces(&state, &input.workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        &state,
        &conversation_id,
        Some(None),
        Some(normalized_workspaces),
        input.autonomous_mode,
    )?;
    {
        let mut roots = state
            .terminal_session_roots
            .lock()
            .map_err(|_| "Failed to lock terminal session roots".to_string())?;
        roots.remove(&session_id);
    }
    let root = terminal_session_root_canonical(&state, &session_id)?;
    Ok(build_chat_shell_workspace_output(
        &state,
        session_id,
        Some(&updated),
        root,
    ))
}

#[tauri::command]
fn resolve_terminal_approval(
    input: ResolveTerminalApprovalInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = resolve_terminal_approval_request(&state, &input.request_id, input.approved)?;
    Ok(())
}

#[tauri::command]
fn open_file_reader_window_command(app: AppHandle, path: String) -> Result<String, String> {
    open_file_reader_window(&app, path)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderFilePayload {
    path: String,
    name: String,
    extension: String,
    kind: String,
    content: String,
    force_plain: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderDirectoryEntry {
    path: String,
    name: String,
    is_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderDirectoryPayload {
    path: String,
    name: String,
    entries: Vec<FileReaderDirectoryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchTargetInput {
    path: String,
    kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchTargetsInput {
    session_id: String,
    targets: Vec<FileReaderWatchTargetInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchEventPayload {
    session_id: String,
    path: String,
    kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileReaderWatchFingerprint {
    exists: bool,
    is_dir: bool,
    len: u64,
    modified_ms: u128,
    directory_signature: String,
}

#[derive(Debug, Clone)]
struct FileReaderWatchTrackedTarget {
    path: String,
    kind: String,
    fingerprint: FileReaderWatchFingerprint,
}

static FILE_READER_WATCH_TARGETS: OnceLock<
    Mutex<std::collections::HashMap<String, Vec<FileReaderWatchTrackedTarget>>>,
> = OnceLock::new();
static FILE_READER_WATCH_THREAD_STARTED: OnceLock<()> = OnceLock::new();

const FILE_READER_PLAIN_TEXT_THRESHOLD: u64 = 2 * 1024 * 1024;
const FILE_READER_LINE_TRUNCATE_CHARS: usize = 10000;
const FILE_READER_READ_BURST_WINDOW_MS: u64 = 100;
const FILE_READER_WATCH_POLL_MS: u64 = 700;
const FILE_READER_WATCH_MAX_TARGETS: usize = 160;
const FILE_READER_WATCH_DIRECTORY_SIGNATURE_LIMIT: usize = 100;

#[derive(Debug, Clone)]
struct FileReaderReadTrace {
    at: std::time::Instant,
    window_label: String,
    path: String,
}

static FILE_READER_READ_TRACES: OnceLock<Mutex<Vec<FileReaderReadTrace>>> = OnceLock::new();

fn file_reader_file_kind(extension: &str) -> &'static str {
    match extension {
        "md" | "markdown" | "mdx" => "markdown",
        _ => "code",
    }
}

fn file_reader_watch_targets_store(
) -> &'static Mutex<std::collections::HashMap<String, Vec<FileReaderWatchTrackedTarget>>> {
    FILE_READER_WATCH_TARGETS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn file_reader_watch_modified_ms(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn file_reader_watch_fingerprint(path: &str) -> FileReaderWatchFingerprint {
    let path_buf = PathBuf::from(path);
    let metadata = match fs::metadata(&path_buf) {
        Ok(value) => value,
        Err(_) => {
            return FileReaderWatchFingerprint {
                exists: false,
                is_dir: false,
                len: 0,
                modified_ms: 0,
                directory_signature: String::new(),
            };
        }
    };
    let is_dir = metadata.is_dir();
    let directory_signature = if is_dir {
        file_reader_watch_directory_signature(&path_buf)
    } else {
        String::new()
    };
    FileReaderWatchFingerprint {
        exists: true,
        is_dir,
        len: metadata.len(),
        modified_ms: file_reader_watch_modified_ms(&metadata),
        directory_signature,
    }
}

fn file_reader_watch_directory_signature(path: &PathBuf) -> String {
    let read_dir = match fs::read_dir(path) {
        Ok(value) => value,
        Err(_) => return String::new(),
    };
    let mut entries = Vec::new();
    for entry in read_dir.take(FILE_READER_WATCH_DIRECTORY_SIGNATURE_LIMIT) {
        let Ok(entry) = entry else { continue };
        let entry_path = entry.path();
        let metadata = match entry.metadata() {
            Ok(value) => value,
            Err(_) => continue,
        };
        entries.push(format!(
            "{}|{}|{}|{}",
            entry.file_name().to_string_lossy(),
            if metadata.is_dir() { "d" } else { "f" },
            metadata.len(),
            file_reader_watch_modified_ms(&metadata)
        ));
        if entry_path.as_os_str().is_empty() {
            continue;
        }
    }
    entries.sort();
    entries.join("\n")
}

fn start_file_reader_watch_polling(app: AppHandle) {
    let _ = FILE_READER_WATCH_THREAD_STARTED.get_or_init(|| {
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(FILE_READER_WATCH_POLL_MS));
            let events = {
                let store = file_reader_watch_targets_store();
                let mut targets_by_session =
                    store.lock().unwrap_or_else(|poison| poison.into_inner());
                let mut events = Vec::new();
                for (session_id, targets) in targets_by_session.iter_mut() {
                    for target in targets.iter_mut() {
                        let next = file_reader_watch_fingerprint(&target.path);
                        if next == target.fingerprint {
                            continue;
                        }
                        target.fingerprint = next;
                        events.push(FileReaderWatchEventPayload {
                            session_id: session_id.clone(),
                            path: target.path.clone(),
                            kind: target.kind.clone(),
                        });
                    }
                }
                events
            };
            for payload in events {
                let _ = app.emit("easy-call:file-reader-watch-changed", payload);
            }
        });
    });
}

#[tauri::command]
fn update_file_reader_watch_targets(
    app: AppHandle,
    input: FileReaderWatchTargetsInput,
) -> Result<(), String> {
    let session_id = input.session_id.trim().to_string();
    if session_id.is_empty() {
        return Err("sessionId is required".to_string());
    }
    start_file_reader_watch_polling(app);
    let mut seen = std::collections::HashSet::new();
    let mut targets = Vec::new();
    for target in input.targets.into_iter().take(FILE_READER_WATCH_MAX_TARGETS) {
        let raw_path = target.path.trim();
        if raw_path.is_empty() {
            continue;
        }
        let normalized_path = PathBuf::from(raw_path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(raw_path))
            .to_string_lossy()
            .replace('\\', "/");
        if !seen.insert(normalized_path.clone()) {
            continue;
        }
        let kind = match target.kind.trim() {
            "directory" => "directory",
            _ => "file",
        }
        .to_string();
        targets.push(FileReaderWatchTrackedTarget {
            fingerprint: file_reader_watch_fingerprint(&normalized_path),
            path: normalized_path,
            kind,
        });
    }
    let store = file_reader_watch_targets_store();
    let mut targets_by_session = store.lock().map_err(|_| "文件阅读器监听状态锁定失败".to_string())?;
    if targets.is_empty() {
        targets_by_session.remove(&session_id);
    } else {
        targets_by_session.insert(session_id, targets);
    }
    Ok(())
}

fn log_file_reader_read_burst(window_label: &str, path: &str) {
    let now = std::time::Instant::now();
    let traces = FILE_READER_READ_TRACES.get_or_init(|| Mutex::new(Vec::new()));
    let mut traces = traces.lock().unwrap_or_else(|poison| poison.into_inner());
    traces.retain(|item| {
        now.saturating_duration_since(item.at).as_millis()
            <= u128::from(FILE_READER_READ_BURST_WINDOW_MS)
    });
    traces.push(FileReaderReadTrace {
        at: now,
        window_label: window_label.to_string(),
        path: path.to_string(),
    });
    if traces.len() <= 2 {
        return;
    }

    let recent = traces
        .iter()
        .map(|item| {
            format!(
                "{{窗口={}, 距今={}ms, 路径={}}}",
                item.window_label,
                now.saturating_duration_since(item.at).as_millis(),
                item.path
            )
        })
        .collect::<Vec<_>>()
        .join("；");
    eprintln!(
        "[文件阅读窗口] 高频读取，任务=read_file_reader_file，窗口={}，100ms内次数={}，当前路径={}，最近读取=[{}]",
        window_label,
        traces.len(),
        path,
        recent
    );
}

fn format_hex_dump(bytes: &[u8]) -> String {
    let display_bytes = &bytes[..bytes.len().min(65536)];
    let mut lines = Vec::with_capacity(display_bytes.len() / 16 + 1);
    for (i, chunk) in display_bytes.chunks(16).enumerate() {
        let offset = i * 16;
        let hex: String = chunk.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
        let ascii: String = chunk.iter().map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' }).collect();
        lines.push(format!("{offset:08X}  {hex:<48}  {ascii}"));
    }
    if bytes.len() > 65536 {
        lines.push(format!("... (已截断，总大小 {} bytes)", bytes.len()));
    }
    lines.join("\n")
}

fn truncate_long_lines(text: &str, max_chars: usize) -> String {
    text.lines()
        .map(|line| {
            if line.chars().count() > max_chars {
                format!("{}...", line.chars().take(max_chars).collect::<String>())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tauri::command]
fn read_file_reader_file(window: tauri::Window, path: String) -> Result<FileReaderFilePayload, String> {
    let raw_path = path.trim();
    if raw_path.is_empty() {
        return Err("path is required".to_string());
    }
    log_file_reader_read_burst(window.label(), raw_path);
    let file_path = PathBuf::from(raw_path);
    if !file_path.exists() {
        return Err(format!("文件不存在：{raw_path}"));
    }
    if !file_path.is_file() {
        return Err(format!("目标不是文件：{raw_path}"));
    }
    let metadata = fs::metadata(&file_path).map_err(|err| format!("读取文件信息失败：{err}"))?;
    let file_size = metadata.len();
    let force_plain = file_size > FILE_READER_PLAIN_TEXT_THRESHOLD;
    let content = match decode_text_file_from_path(&file_path) {
        Ok(decoded) => {
            if force_plain {
                truncate_long_lines(&decoded.text, FILE_READER_LINE_TRUNCATE_CHARS)
            } else {
                decoded.text
            }
        }
        Err(_) => {
            let bytes = fs::read(&file_path).map_err(|err| format!("读取文件失败：{err}"))?;
            format_hex_dump(&bytes)
        }
    };
    let resolved_path = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
    let extension = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(raw_path)
        .to_string();
    let file_key = if extension.is_empty() {
        name.trim().to_ascii_lowercase()
    } else {
        extension.clone()
    };
    Ok(FileReaderFilePayload {
        path: resolved_path.to_string_lossy().replace('\\', "/"),
        name,
        extension: file_key.clone(),
        kind: file_reader_file_kind(&file_key).to_string(),
        content,
        force_plain,
    })
}

#[tauri::command]
fn list_file_reader_directory(path: String) -> Result<FileReaderDirectoryPayload, String> {
    let raw_path = path.trim();
    if raw_path.is_empty() {
        return Err("path is required".to_string());
    }
    let directory_path = PathBuf::from(raw_path);
    if !directory_path.exists() {
        return Err(format!("目录不存在：{raw_path}"));
    }
    if !directory_path.is_dir() {
        return Err(format!("目标不是目录：{raw_path}"));
    }

    let resolved_path = directory_path
        .canonicalize()
        .unwrap_or_else(|_| directory_path.clone());
    let mut entries = Vec::new();
    let read_dir = fs::read_dir(&directory_path).map_err(|err| format!("读取目录失败：{err}"))?;
    for entry in read_dir {
        let entry = entry.map_err(|err| format!("读取目录项失败：{err}"))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("读取目录项类型失败：{err}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.trim().is_empty() {
            continue;
        }
        entries.push(FileReaderDirectoryEntry {
            path: entry_path
                .canonicalize()
                .unwrap_or(entry_path)
                .to_string_lossy()
                .replace('\\', "/"),
            name,
            is_directory: file_type.is_dir(),
        });
    }
    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });

    let name = resolved_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_else(|| raw_path.trim_end_matches(['/', '\\']))
        .to_string();
    Ok(FileReaderDirectoryPayload {
        path: resolved_path.to_string_lossy().replace('\\', "/"),
        name,
        entries,
    })
}

#[tauri::command]
fn open_file_reader_directory_shell(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let raw_path = path.trim();
    if raw_path.is_empty() {
        return Err("path is required".to_string());
    }
    open_shell_terminal_at_path(&state, &PathBuf::from(raw_path))
}

#[tauri::command]
fn open_local_file_directory(path: String) -> Result<(), String> {
    let raw_path = path.trim();
    if raw_path.is_empty() {
        return Err("path is required".to_string());
    }
    let file_path = PathBuf::from(raw_path);

    if file_path.is_file() {
        #[cfg(target_os = "windows")]
        {
            let resolved_path = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
            let explorer_target = resolved_path.to_string_lossy().replace('/', "\\");
            std::process::Command::new("explorer")
                .args(["/select,", explorer_target.as_str()])
                .status()
                .map_err(|err| format!("Failed to open explorer: {err}"))?;
            return Ok(());
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", raw_path])
                .status()
                .map_err(|err| format!("Failed to open Finder: {err}"))?;
            return Ok(());
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Some(parent) = file_path.parent() {
                std::process::Command::new("xdg-open")
                    .arg(parent)
                    .status()
                    .map_err(|err| format!("Failed to open file manager: {err}"))?;
            } else {
                std::process::Command::new("xdg-open")
                    .arg(&file_path)
                    .status()
                    .map_err(|err| format!("Failed to open file manager: {err}"))?;
            }
            return Ok(());
        }
    }

    open_shell_path_in_file_manager(&file_path)?;
    Ok(())
}

#[tauri::command]
fn open_file_with_default_program(path: String) -> Result<(), String> {
    let raw_path = path.trim();
    if raw_path.is_empty() {
        return Err("path is required".to_string());
    }
    let file_path = PathBuf::from(raw_path);
    if !file_path.exists() {
        return Err(format!("File not found: {raw_path}"));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", raw_path])
            .status()
            .map_err(|err| format!("Failed to open file: {err}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(raw_path)
            .status()
            .map_err(|err| format!("Failed to open file: {err}"))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(raw_path)
            .status()
            .map_err(|err| format!("Failed to open file: {err}"))?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Open file is not supported on this platform".to_string())
}
