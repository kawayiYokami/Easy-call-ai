fn format_message_time_rfc3339_local(raw: &str) -> String {
    format_utc_storage_time_to_local_rfc3339(raw)
}

fn format_message_time_text(raw: &str) -> String {
    format_utc_storage_time_to_local_text(raw)
}

fn default_agent() -> AgentProfile {
    let now = now_iso();
    AgentProfile {
        id: DEFAULT_AGENT_ID.to_string(),
        name: "助理".to_string(),
        system_prompt: "你是谁：你是助理，是用户默认会先对话的助手。\n台词技巧：表达自然、直接、有人味；先给结论，再补必要说明；少空话，少套话。\n性格画像：耐心、友善、靠谱、利落。".to_string(),
        tools: default_agent_tools(),
        created_at: now.clone(),
        updated_at: now,
        avatar_path: None,
        avatar_updated_at: None,
        is_built_in_user: false,
        is_built_in_system: false,
        private_memory_enabled: false,
        memory_recall_mode: default_agent_memory_recall_mode(),
        source: default_main_source(),
        scope: default_global_scope(),
    }
}

#[allow(dead_code)]
fn default_deputy_agent() -> AgentProfile {
    let now = now_iso();
    AgentProfile {
        id: DEPUTY_AGENT_ID.to_string(),
        name: "副手".to_string(),
        system_prompt: "你是谁：你是副手，是一个偏执行、偏推进的助手分身。\n台词技巧：短句作答，直给重点，少铺垫，少客套。\n性格画像：简洁、干脆、克制、利落。".to_string(),
        tools: default_agent_tools(),
        created_at: now.clone(),
        updated_at: now,
        avatar_path: None,
        avatar_updated_at: None,
        is_built_in_user: false,
        is_built_in_system: true,
        private_memory_enabled: false,
        memory_recall_mode: default_agent_memory_recall_mode(),
        source: default_main_source(),
        scope: default_global_scope(),
    }
}

fn default_user_persona() -> AgentProfile {
    let now = now_iso();
    AgentProfile {
        id: USER_PERSONA_ID.to_string(),
        name: "用户".to_string(),
        system_prompt: "我是...".to_string(),
        tools: default_agent_tools(),
        created_at: now.clone(),
        updated_at: now,
        avatar_path: None,
        avatar_updated_at: None,
        is_built_in_user: true,
        is_built_in_system: false,
        private_memory_enabled: false,
        memory_recall_mode: default_agent_memory_recall_mode(),
        source: default_main_source(),
        scope: default_global_scope(),
    }
}

fn default_system_persona() -> AgentProfile {
    let now = now_iso();
    AgentProfile {
        id: SYSTEM_PERSONA_ID.to_string(),
        name: "pai system".to_string(),
        system_prompt: "你是谁：你是 pai system，是系统消息与状态播报使用的人格。\n台词技巧：用词明确、稳定、客观，像系统通知，不抒情，不延展。\n性格画像：冷静、克制、严谨。".to_string(),
        tools: default_agent_tools(),
        created_at: now.clone(),
        updated_at: now,
        avatar_path: None,
        avatar_updated_at: None,
        is_built_in_user: false,
        is_built_in_system: true,
        private_memory_enabled: false,
        memory_recall_mode: default_agent_memory_recall_mode(),
        source: default_main_source(),
        scope: default_global_scope(),
    }
}

fn normalize_agent_tools(agent: &mut AgentProfile) -> bool {
    let defaults = default_agent_tools();
    let mut next = Vec::<ApiToolConfig>::new();
    for default in defaults {
        let found = agent.tools.iter().find(|tool| {
            tool.id == default.id || (default.id == "read" && tool.id == "read_file")
        });
        if let Some(found) = found {
            next.push(ApiToolConfig {
                id: default.id.clone(),
                command: if found.command.trim().is_empty() {
                    default.command.clone()
                } else {
                    found.command.clone()
                },
                args: if found.args.is_empty() {
                    default.args.clone()
                } else if default.id == "read" && found.id == "read_file" {
                    default.args.clone()
                } else {
                    found.args.clone()
                },
                enabled: found.enabled,
                values: found.values.clone(),
            });
        } else {
            next.push(default);
        }
    }
    let changed = agent.tools.len() != next.len()
        || agent.tools.iter().zip(next.iter()).any(|(left, right)| {
            left.id != right.id
                || left.enabled != right.enabled
                || left.command != right.command
                || left.args != right.args
                || left.values != right.values
        });
    if changed {
        agent.tools = next;
    }
    changed
}

fn ensure_required_builtin_agents(data: &mut AppData) -> bool {
    let mut changed = false;
    if !data.agents.iter().any(|agent| agent.id == DEFAULT_AGENT_ID) {
        data.agents.push(default_agent());
        changed = true;
    }
    if !data.agents.iter().any(|agent| agent.id == DEPUTY_AGENT_ID) {
        data.agents.push(default_deputy_agent());
        changed = true;
    }
    if !data.agents.iter().any(|agent| agent.id == USER_PERSONA_ID) {
        data.agents.push(default_user_persona());
        changed = true;
    }
    if !data.agents.iter().any(|agent| agent.id == SYSTEM_PERSONA_ID) {
        data.agents.push(default_system_persona());
        changed = true;
    }
    changed
}

fn fill_missing_message_speaker_agent_ids(data: &mut AppData) -> bool {
    fn provider_meta_speaker_agent_id(message: &ChatMessage) -> Option<String> {
        let meta = message.provider_meta.as_ref()?;
        let object = meta.as_object()?;
        for key in [
            "speakerAgentId",
            "speaker_agent_id",
            "targetAgentId",
            "target_agent_id",
            "agentId",
            "agent_id",
            "sourceAgentId",
            "source_agent_id",
        ] {
            let value = object
                .get(key)
                .and_then(|item| item.as_str())
                .unwrap_or("")
                .trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
        None
    }

    let mut changed = false;
    for conversation in &mut data.conversations {
        let host_agent_id = conversation.agent_id.trim().to_string();
        if host_agent_id.is_empty() {
            continue;
        }
        for message in &mut conversation.messages {
            let current = message
                .speaker_agent_id
                .as_deref()
                .map(str::trim)
                .unwrap_or("");
            if current.is_empty() {
                message.speaker_agent_id =
                    Some(provider_meta_speaker_agent_id(message).unwrap_or_else(|| {
                        if message.role == "user" {
                            USER_PERSONA_ID.to_string()
                        } else {
                            host_agent_id.clone()
                        }
                    }));
                changed = true;
            }
        }
    }
    changed
}

fn fill_missing_conversation_metadata(data: &mut AppData) -> bool {
    let mut changed = false;
    for conversation in &mut data.conversations {
        if conversation.conversation_kind.trim().is_empty() {
            conversation.conversation_kind = CONVERSATION_KIND_CHAT.to_string();
            changed = true;
        }
    }
    for archive in &mut data.archived_conversations {
        if archive
            .source_conversation
            .conversation_kind
            .trim()
            .is_empty()
        {
            archive.source_conversation.conversation_kind = CONVERSATION_KIND_CHAT.to_string();
            changed = true;
        }
    }
    changed
}
