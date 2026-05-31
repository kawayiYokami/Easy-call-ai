async fn builtin_reload(app_state: &AppState) -> Result<Value, String> {
    let result = reload_workspace(app_state).await?;
    serde_json::to_value(result).map_err(|err| format!("序列化刷新结果失败：{err}"))
}

async fn builtin_organize_context(
    app_state: &AppState,
    session_id: &str,
    api_config_id: &str,
    agent_id: &str,
) -> Result<Value, String> {
    let (selected_api, source) = {
        let app_config = state_read_config_cached(app_state)?;
        let selected_api = resolve_selected_api_config(&app_config, Some(api_config_id))
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let (_, session_agent_id, session_conversation_id) = delegate_parse_session_parts(session_id);
        let requested_agent_id = agent_id.trim();
        let effective_agent_id = if requested_agent_id.is_empty() {
            session_agent_id.trim().to_string()
        } else {
            requested_agent_id.to_string()
        };
        if effective_agent_id.trim().is_empty() {
            return Err("缺少人格 ID，无法整理上下文。".to_string());
        }
        let conversation_id = session_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "缺少当前工具调用会话 ID，无法整理上下文。".to_string())?;
        let source = state_read_conversation_cached(app_state, conversation_id)
            .map_err(|err| format!("当前工具调用会话不可整理：{}", err))?;
        if source.messages.len() < 10 {
            return Ok(serde_json::json!({
                "ok": false,
                "shouldArchive": false,
                "message": "此时不应该整理：当前对话少于 10 句。"
            }));
        }
        let usage_ratio = conversation_prompt_service()
            .latest_real_prompt_usage(&source, &selected_api)
            .map(|usage| usage.usage_ratio.max(0.0))
            .unwrap_or(0.0);
        if usage_ratio < 0.10 {
            return Ok(serde_json::json!({
                "ok": false,
                "shouldArchive": false,
                "usageRatio": usage_ratio,
                "message": "此时不应该整理：当前上下文占用不足 10%。"
            }));
        }
        (selected_api, source)
    };

    Ok(serde_json::json!({
        "ok": true,
        "applied": true,
        "terminal": true,
        "scheduled": false,
        "apiConfigId": selected_api.id,
        "conversationId": source.id,
        "message": "上下文整理已接管当前轮，系统会先保存已完成工作，再整理并续跑。"
    }))
}
