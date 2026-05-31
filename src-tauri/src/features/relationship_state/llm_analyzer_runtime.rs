pub async fn run_relationship_interaction_analyzer(
    resolved_api: &ResolvedApiConfig,
    selected_api: &ApiConfig,
    model_name: &str,
    user_text: &str,
    recent_context: &[String],
    current_state: &relationship_state::AgentRelationshipState,
    agent: &AgentProfile,
    app_state: &AppState,
) -> Result<relationship_state::InteractionEvent, String> {
    let mut analyzer_api = selected_api.clone();
    analyzer_api.enable_tools = false;
    analyzer_api.enable_image = false;
    analyzer_api.enable_audio = false;
    let prepared = PreparedPrompt {
        preamble: relationship_state::RELATIONSHIP_ANALYZER_SYSTEM_PROMPT.to_string(),
        history_messages: Vec::new(),
        latest_user_text: relationship_state::build_relationship_analyzer_user_prompt(
            user_text,
            recent_context,
            current_state,
            &agent.name,
        ),
        latest_user_meta_text: String::new(),
        latest_user_extra_text: String::new(),
        latest_user_extra_blocks: Vec::new(),
        latest_images: Vec::new(),
        latest_audios: Vec::new(),
    };
    let reply = if analyzer_api.request_format.is_gemini() {
        call_model_gemini(resolved_api, model_name, prepared, Some(app_state)).await?
    } else if analyzer_api.request_format.is_anthropic() {
        call_model_anthropic(resolved_api, model_name, prepared, Some(app_state)).await?
    } else if analyzer_api.request_format.is_openai_responses_family() {
        call_model_openai_responses(resolved_api, model_name, prepared, None, Some(app_state)).await?
    } else {
        call_model_openai_non_stream(resolved_api, model_name, prepared, Some(app_state)).await?
    };
    relationship_state::parse_interaction_event_json(&reply.final_response_text)
}

pub fn relationship_recent_context(conversation: &Conversation, agent_id: &str) -> Vec<String> {
    conversation
        .messages
        .iter()
        .rev()
        .filter_map(|message| {
            let role = prompt_role_for_message(message, agent_id)?;
            let text = render_message_content_for_model(message).trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(format!("{}: {}", role, text))
            }
        })
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
