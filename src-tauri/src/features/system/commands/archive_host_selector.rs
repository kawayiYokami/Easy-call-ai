fn resolve_archive_owner_agent_id(
    config: &AppConfig,
    agents: &[AgentProfile],
    source: &Conversation,
) -> Result<String, String> {
    let department_id = source.department_id.trim();
    if department_id.is_empty() {
        return Err(format!(
            "会话缺少归属部门，无法确定归档记忆归属人格: conversation_id={}",
            source.id
        ));
    }

    let department = department_by_id(config, department_id).ok_or_else(|| {
        format!(
            "会话归属部门不存在，无法确定归档记忆归属人格: conversation_id={}, department_id={}",
            source.id, department_id
        )
    })?;

    let owner_agent_id = source.agent_id.trim();
    let owner_agent_id = if owner_agent_id.is_empty() {
        first_available_department_agent(department, agents)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| {
                format!(
                    "会话归属部门没有可用人格，无法确定归档记忆归属人格: conversation_id={}, department_id={}",
                    source.id, department_id
                )
            })?
    } else {
        if available_non_user_agent(agents, owner_agent_id).is_none() {
            return Err(format!(
                "归档记忆归属人格不存在: conversation_id={}, department_id={}, agent_id={}",
                source.id, department_id, owner_agent_id
            ));
        }
        owner_agent_id.to_string()
    };

    Ok(owner_agent_id)
}

#[cfg(test)]
mod archive_host_selection_tests {
    use super::*;

    fn mk_agent(id: &str) -> AgentProfile {
        AgentProfile {
            id: id.to_string(),
            name: id.to_string(),
            system_prompt: String::new(),
            tools: default_agent_tools(),
            created_at: now_iso(),
            updated_at: now_iso(),
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

    fn mk_department(id: &str, agent_ids: Vec<&str>) -> DepartmentConfig {
        DepartmentConfig {
            id: id.to_string(),
            name: id.to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: Vec::new(),
            api_config_id: String::new(),
            model_failure_fallback_enabled: false,
            agent_ids: agent_ids.into_iter().map(ToOwned::to_owned).collect(),
            child_department_ids: Vec::new(),
            created_at: now_iso(),
            updated_at: now_iso(),
            order_index: 0,
            is_built_in_assistant: false,
            is_deputy: false,
            source: default_main_source(),
            scope: default_global_scope(),
            permission_control: DepartmentPermissionControl::default(),
        }
    }

    fn mk_msg_with_agent_hint(agent_id: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            created_at: now_iso(),
            speaker_agent_id: Some(agent_id.to_string()),
            parts: vec![MessagePart::Text {
                text: "x".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "agentId": agent_id,
            })),
            tool_call: None,
            mcp_call: None,
        meme_annotations: None,
        }
    }

    fn mk_source(department_id: &str, agent_id: &str, messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            id: "c1".to_string(),
            title: "t".to_string(),
            agent_id: agent_id.to_string(),
            department_id: department_id.to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now_iso(),
            updated_at: now_iso(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            shell_work_mode: default_shell_work_mode(),
            archived_at: None,
            messages,
            fast_request_turns: Vec::new(),
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
            is_draft: false,
        }
    }

    #[test]
    fn archive_owner_should_come_from_conversation_agent() {
        let config = AppConfig {
            departments: vec![mk_department("dept-main", vec!["owner-agent", "message-agent"])],
            ..AppConfig::default()
        };
        let agents = vec![mk_agent("owner-agent"), mk_agent("message-agent")];
        let source = mk_source(
            "dept-main",
            "message-agent",
            vec![
                mk_msg_with_agent_hint("message-agent"),
                mk_msg_with_agent_hint("message-agent"),
            ],
        );

        let owner = resolve_archive_owner_agent_id(&config, &agents, &source).unwrap();

        assert_eq!(owner, "message-agent");
    }

    #[test]
    fn archive_owner_should_reject_missing_department() {
        let config = AppConfig::default();
        let agents = vec![mk_agent("owner-agent")];
        let source = mk_source("missing-dept", "owner-agent", Vec::new());

        let err = resolve_archive_owner_agent_id(&config, &agents, &source).unwrap_err();

        assert!(err.contains("会话归属部门不存在"));
    }

    #[test]
    fn archive_owner_should_use_department_first_agent_when_conversation_agent_missing() {
        let agents = vec![mk_agent("a1"), mk_agent("a2")];
        let config = AppConfig {
            departments: vec![mk_department("dept-main", vec!["a1", "a2"])],
            ..AppConfig::default()
        };
        let source = mk_source("dept-main", "", Vec::new());

        let owner = resolve_archive_owner_agent_id(&config, &agents, &source).unwrap();

        assert_eq!(owner, "a1");
    }

    #[test]
    fn archive_owner_should_reject_missing_agent() {
        let config = AppConfig {
            departments: vec![mk_department("dept-main", vec!["owner-agent"])],
            ..AppConfig::default()
        };
        let source = mk_source("dept-main", "owner-agent", Vec::new());

        let err = resolve_archive_owner_agent_id(&config, &[], &source).unwrap_err();

        assert!(err.contains("归档记忆归属人格不存在"));
    }

    #[test]
    fn archive_owner_should_accept_private_runtime_department() {
        let root = std::env::temp_dir().join(format!(
            "eca-archive-owner-private-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("data").join("app_data.json");
        let departments_dir = app_root_from_data_path(&data_path)
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&departments_dir).expect("create private departments dir");
        std::fs::write(
            departments_dir.join("dept-private.json"),
            r#"{
  "id": "dept-private",
  "name": "私域归档部门",
  "agentIds": ["private-owner"]
}"#,
        )
        .expect("write private department");

        let snapshot = build_runtime_organization_snapshot_from_parts(
            &data_path,
            &AppConfig::default(),
            &[mk_agent("private-owner"), default_user_persona()],
        )
        .expect("build runtime snapshot");
        let source = mk_source("dept-private", "private-owner", Vec::new());

        let owner =
            resolve_archive_owner_agent_id(&snapshot.config, &snapshot.agents, &source).unwrap();

        assert_eq!(owner, "private-owner");
        let _ = std::fs::remove_dir_all(root);
    }
}
