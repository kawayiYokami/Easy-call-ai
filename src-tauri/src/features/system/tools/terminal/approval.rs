#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalApprovalRequestPayload {
    request_id: String,
    title: String,
    message: String,
    approval_kind: String,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    call_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    existing_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    target_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_opinion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_model_name: Option<String>,
}

async fn terminal_request_user_approval(
    state: &AppState,
    title: &str,
    message: &str,
    session_id: &str,
    approval_kind: &str,
    tool_name: Option<&str>,
    summary: Option<&str>,
    call_preview: Option<&str>,
    cwd: Option<&Path>,
    command: Option<&str>,
    requested_path: Option<&Path>,
    reason: Option<&str>,
    existing_paths: &[PathBuf],
    target_paths: &[PathBuf],
    review_opinion: Option<&str>,
    review_model_name: Option<&str>,
) -> Result<bool, String> {
    let request_id = Uuid::new_v4().to_string();
    let app_handle = {
        let guard = state
            .app_handle
            .lock()
            .map_err(|_| "Failed to lock app handle".to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "App handle is not ready".to_string())?
    };

    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    {
        let mut pending = state
            .terminal_pending_approvals
            .lock()
            .map_err(|_| "Failed to lock terminal pending approvals".to_string())?;
        pending.insert(request_id.clone(), tx);
    }

    let payload = TerminalApprovalRequestPayload {
        request_id: request_id.clone(),
        title: title.to_string(),
        message: message.to_string(),
        approval_kind: approval_kind.to_string(),
        session_id: session_id.to_string(),
        tool_name: tool_name
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        summary: summary
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        call_preview: call_preview
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        cwd: cwd.map(terminal_path_for_user),
        command: command
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        requested_path: requested_path.map(terminal_path_for_user),
        reason: reason
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        existing_paths: existing_paths
            .iter()
            .take(32)
            .map(|path| terminal_path_for_user(path))
            .collect(),
        target_paths: target_paths
            .iter()
            .take(32)
            .map(|path| terminal_path_for_user(path))
            .collect(),
        review_opinion: review_opinion
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
        review_model_name: review_model_name
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string),
    };

    if let Err(err) = app_handle.emit("easy-call:terminal-approval-request", &payload) {
        if let Ok(mut pending) = state.terminal_pending_approvals.lock() {
            pending.remove(&request_id);
        }
        return Err(format!("Emit terminal approval request failed: {err}"));
    }
    if let Ok(value) = serde_json::to_value(&payload) {
        ide_chat_broadcast_notification("terminalApproval.requested", value);
    }

    let wait_result = rx.await;

    if let Ok(mut pending) = state.terminal_pending_approvals.lock() {
        pending.remove(&request_id);
    }

    match wait_result {
        Ok(approved) => Ok(approved),
        Err(_) => Err("Terminal approval channel closed unexpectedly.".to_string()),
    }
}

fn resolve_terminal_approval_request(
    state: &AppState,
    request_id: &str,
    approved: bool,
) -> Result<bool, String> {
    let trimmed = request_id.trim();
    if trimmed.is_empty() {
        return Err("requestId is empty.".to_string());
    }

    let sender = {
        let mut pending = state
            .terminal_pending_approvals
            .lock()
            .map_err(|_| "Failed to lock terminal pending approvals".to_string())?;
        pending.remove(trimmed)
    };

    let Some(sender) = sender else {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] terminal approval request not found: {}",
            trimmed
        ));
        return Ok(false);
    };

    if sender.send(approved).is_err() {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] terminal approval receiver dropped: {}",
            trimmed
        ));
        return Ok(false);
    }
    Ok(true)
}
