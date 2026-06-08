    #[test]
    fn build_prompt_should_include_structured_tool_history_messages() {
        let now = now_iso();
        let mut assistant_with_tool = test_text_message("assistant", "我去查一下", &now);
        assistant_with_tool.tool_call = Some(vec![
            serde_json::json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "bing_search",
                        "arguments": "{\"query\":\"rust\"}"
                    }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"results\":[{\"title\":\"Rust\"}]}"
            }),
        ]);
        let agent = default_agent();
        assistant_with_tool.speaker_agent_id = Some(agent.id.clone());

        let messages = vec![
            test_text_message("user", "帮我查 Rust", &now),
            assistant_with_tool,
            test_text_message("user", "继续", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert!(
            prepared
                .history_messages
                .iter()
                .any(|m| m.role == "assistant" && m.tool_calls.is_some())
        );
        assert!(
            prepared.history_messages.iter().any(|m| {
                m.role == "tool"
                    && m.tool_call_id.as_deref() == Some("call_1")
                    && m.text.contains("\"results\"")
            })
        );
    }

    #[test]
    fn build_prompt_should_replay_final_parts_reasoning_after_tool_history() {
        let now = now_iso();
        let agent = default_agent();
        let mut assistant = test_text_message("assistant", "最终回答", &now);
        assistant.speaker_agent_id = Some(agent.id.clone());
        assistant.tool_call = Some(vec![serde_json::json!({
            "role": "assistant",
            "content": null,
            "reasoning_content": "工具思考",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "read",
                    "arguments": "{}"
                }
            }]
        }), serde_json::json!({
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "工具结果"
        })]);
        if let Some(MessagePart::Text {
            reasoning_content,
            ..
        }) = assistant.parts.first_mut()
        {
            *reasoning_content = Some("最终思考".to_string());
        }

        let messages = vec![
            test_text_message("user", "上一轮问题", &now),
            assistant,
            test_text_message("user", "继续", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        let final_assistant_messages = prepared
            .history_messages
            .iter()
            .filter(|message| message.role == "assistant" && message.text == "最终回答")
            .collect::<Vec<_>>();

        assert_eq!(final_assistant_messages.len(), 1);
        assert_eq!(
            final_assistant_messages[0].reasoning_content.as_deref(),
            Some("最终思考")
        );
        assert!(prepared.history_messages.iter().any(|message| {
            message.role == "assistant"
                && message.tool_calls.as_ref().is_some_and(|calls| !calls.is_empty())
                && message.reasoning_content.as_deref() == Some("工具思考")
        }));
    }

    #[test]
    fn conversation_prompt_service_snapshot_should_keep_cache_hits_stable() {
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            test_text_message("user", "帮我看一下会话缓存", &now),
            test_text_message("assistant", "我先整理一下", &now),
            test_text_message("user", "继续", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now.clone()));
        let fixed_system_prompt = build_core_system_prompt_text(
            &conv,
            &agent,
            &[],
            Some(("用户", "我是测试用户")),
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
        );

        let first = conversation_prompt_service().build_prompt_snapshot(
            None,
            "chat",
            &conv,
            &agent,
            &[],
            "zh-CN",
            None,
            &fixed_system_prompt,
            None,
            None,
            &[],
            None,
        );
        let second = conversation_prompt_service().build_prompt_snapshot(
            None,
            "chat",
            &conv,
            &agent,
            &[],
            "zh-CN",
            None,
            &fixed_system_prompt,
            None,
            None,
            &[],
            None,
        );

        assert_eq!(first.revisions, second.revisions);
        assert_eq!(first.department_prompt, second.department_prompt);
        assert_eq!(first.environment_prompt, second.environment_prompt);
        assert_eq!(first.abstract_messages, second.abstract_messages);
    }

    #[test]
    fn conversation_prompt_service_prompt_revision_should_ignore_todos_and_memory_recall() {
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            test_text_message("user", "检查 prompt revision", &now),
            test_text_message("assistant", "收到", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now.clone()));
        let fixed_system_prompt = build_core_system_prompt_text(
            &conv,
            &agent,
            &[],
            Some(("用户", "我是测试用户")),
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
        );
        let baseline = conversation_prompt_service().build_prompt_snapshot(
            None,
            "chat",
            &conv,
            &agent,
            &[],
            "zh-CN",
            None,
            &fixed_system_prompt,
            None,
            None,
            &[],
            None,
        );

        let mut with_conversation_side_blocks = conv.clone();
        with_conversation_side_blocks.current_todos.push(ConversationTodoItem {
            content: "第一步".to_string(),
            status: "in_progress".to_string(),
        });
        with_conversation_side_blocks
            .memory_recall_table
            .push("memory-1".to_string());
        let fixed_after = build_core_system_prompt_text(
            &with_conversation_side_blocks,
            &agent,
            &[],
            Some(("用户", "我是测试用户")),
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
        );
        let mutated = conversation_prompt_service().build_prompt_snapshot(
            None,
            "chat",
            &with_conversation_side_blocks,
            &agent,
            &[],
            "zh-CN",
            None,
            &fixed_after,
            None,
            None,
            &[],
            None,
        );

        assert_eq!(baseline.revisions.prompt_revision, mutated.revisions.prompt_revision);
    }

    #[test]
    fn build_prompt_should_map_non_self_personas_to_user_with_speaker_block() {
        let now = now_iso();
        let agent = default_agent();
        let system_persona = default_system_persona();
        let messages = vec![
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(system_persona.id.clone()),
                parts: vec![MessagePart::Text {
                    text: "请检查今天的任务触发情况".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(agent.id.clone()),
                parts: vec![MessagePart::Text {
                    text: "我马上处理".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(system_persona.id.clone()),
                parts: vec![MessagePart::Text {
                    text: "现在补发第二次提醒".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona(), system_persona.clone()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 2);
        assert_eq!(prepared.history_messages[0].role, "user");
        assert!(
            prepared.history_messages[0]
                .user_time_text
                .as_deref()
                .unwrap_or_default()
                .contains("pai system")
        );
        assert_eq!(prepared.history_messages[1].role, "assistant");
        assert!(prepared.latest_user_meta_text.contains("pai system"));
        assert!(prepared.latest_user_text.contains("现在补发第二次提醒"));
    }

    #[test]
    fn build_prompt_user_meta_text_should_not_append_memory_injected_tag() {
        let now = now_iso();
        let mut message = test_text_message("user", "继续", &now);
        message.extra_text_blocks.push(
            "<memory_context>\n<id:m1>\n用户询问 codex 是什么\n> 无\n</id:m1>\n</memory_context>"
                .to_string(),
        );

        let meta = build_prompt_user_meta_text(
            &message,
            &[default_agent(), default_user_persona()],
            "用户",
            "zh-CN",
            false,
        )
        .expect("meta text");

        assert!(!meta.contains("memory=已注入"));
        assert!(meta.contains("T"));
    }

    #[test]
    fn build_prompt_user_meta_text_should_include_local_user_id() {
        let now = now_iso();
        let mut message = test_text_message("user", "继续", &now);
        message.speaker_agent_id = Some(USER_PERSONA_ID.to_string());

        let meta = build_prompt_user_meta_text(
            &message,
            &[default_agent(), default_user_persona()],
            "用户",
            "zh-CN",
            false,
        )
        .expect("meta text");

        assert!(meta.contains("user_id=user-persona"));
    }

    #[test]
    fn build_prompt_user_meta_text_should_use_snake_case_remote_identity_tags() {
        let now = now_iso();
        let mut message = test_text_message("user", "你好", &now);
        message.provider_meta = Some(serde_json::json!({
            "origin": {
                "kind": "remote_im",
                "channel_id": "remote-im-1",
                "contact_type": "group",
                "contact_id": "group-42",
                "contact_name": "测试群",
                "sender_name": "张三"
            }
        }));

        let meta = build_prompt_user_meta_text(
            &message,
            &[default_agent(), default_user_persona()],
            "用户",
            "zh-CN",
            true,
        )
        .expect("meta text");

        assert!(meta.contains("张三 (测试群)"));
        assert!(meta.contains("channel_id=remote-im-1"));
        assert!(meta.contains("contact_id=group-42"));
        assert!(!meta.contains("channelId="));
        assert!(!meta.contains("contactId="));
    }

    #[test]
    fn build_prompt_user_meta_text_should_ignore_legacy_remote_identity_keys() {
        let now = now_iso();
        let mut message = test_text_message("user", "你好", &now);
        message.provider_meta = Some(serde_json::json!({
            "origin": {
                "kind": "remote_im",
                "channelId": "legacy-channel",
                "remoteContactType": "private",
                "remoteContactId": "legacy-contact",
                "remoteContactName": "旧联系人",
                "senderName": "旧联系人"
            }
        }));

        let meta = build_prompt_user_meta_text(
            &message,
            &[default_agent(), default_user_persona()],
            "用户",
            "zh-CN",
            true,
        )
        .expect("meta text");

        assert!(!meta.contains("旧联系人"));
        assert!(!meta.contains("channel_id=legacy-channel"));
        assert!(!meta.contains("contact_id=legacy-contact"));
    }

    #[test]
    fn build_prompt_should_delay_inject_retrieved_memories_with_request_local_dedupe() {
        let state = test_chat_runtime_state();
        let drafts = vec![
            MemoryDraftInput {
                memory_type: "knowledge".to_string(),
                judgment: "用户很喜欢猫咪".to_string(),
                reasoning: "因为用户妈妈从小养猫".to_string(),
                tags: vec!["猫".to_string()],
                owner_agent_id: None,
            },
            MemoryDraftInput {
                memory_type: "knowledge".to_string(),
                judgment: "用户对花生过敏".to_string(),
                reasoning: "因为用户小时候吃花生酱休克过".to_string(),
                tags: vec!["花生".to_string(), "过敏".to_string()],
                owner_agent_id: None,
            },
        ];
        let (saved, _) = memory_store_upsert_drafts(&state.data_path, &drafts).expect("seed memories");
        let cat_memory_id = saved[0].id.clone().expect("cat memory id");
        let peanut_memory_id = saved[1].id.clone().expect("peanut memory id");
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(USER_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "我家猫吐毛球怎么办？".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "retrieved_memory_ids": [cat_memory_id]
                })),
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(agent.id.clone()),
                parts: vec![MessagePart::Text {
                    text: "吐毛球可以先观察饮食和梳毛频率。".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(USER_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "我想吃花生酱面包，可以吗？".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "retrieved_memory_ids": [saved[0].id.clone().expect("dup cat id"), peanut_memory_id.clone(), peanut_memory_id]
                })),
                tool_call: None,
                mcp_call: None,
            },
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            Some(&state.data_path),
            None,
            None,
            false,
        );

        let history_extra = prepared.history_messages[0].extra_text_blocks.join("\n");
        assert_eq!(history_extra.matches("用户很喜欢猫咪").count(), 1);
        assert!(history_extra.contains("因为用户妈妈从小养猫"));
        assert!(!history_extra.contains("用户对花生过敏"));
        assert!(prepared.latest_user_extra_text.contains("用户对花生过敏"));
        assert_eq!(prepared.latest_user_extra_text.matches("用户很喜欢猫咪").count(), 0);
        assert_eq!(prepared.latest_user_extra_text.matches("用户对花生过敏").count(), 1);
    }

    #[test]
    fn manual_memory_recall_mode_should_skip_rag_but_keep_builtin_recall() {
        let state = test_chat_runtime_state();
        let mut agent = default_agent();
        agent.memory_recall_mode = MEMORY_RECALL_MODE_MANUAL.to_string();
        state_write_agents_cached(&state, &[agent.clone(), default_user_persona()])
            .expect("seed agents");
        memory_store_upsert_drafts(
            &state.data_path,
            &[MemoryDraftInput {
                memory_type: "knowledge".to_string(),
                judgment: "用户喜欢猫咪".to_string(),
                reasoning: "回归测试".to_string(),
                tags: vec!["猫咪".to_string()],
                owner_agent_id: None,
            }],
        )
        .expect("seed memory");
        let user_message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "user".to_string(),
            created_at: now_iso(),
            speaker_agent_id: Some(USER_PERSONA_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "猫咪".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        };

        let recall_payload = collect_recall_payload_for_user_message(
            &state.data_path,
            &[agent.clone()],
            &agent.id,
            &user_message,
        )
        .expect("collect recall payload");
        assert!(recall_payload.stored_ids.is_empty());
        assert!(recall_payload.raw_ids.is_empty());

        let memory_context = memory_agent_context_from_agent(&agent)
            .expect("manual memory context");
        let recall_result = builtin_recall(&state, &memory_context, "猫咪", None, None, None)
            .expect("manual recall");
        let memory_board = recall_result
            .get("memoryBoard")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(recall_result.get("count").and_then(Value::as_u64), Some(1));
        assert!(memory_board.contains("用户喜欢猫咪"));
    }

    #[test]
    fn off_memory_recall_mode_should_skip_rag_and_builtin_recall_results() {
        let state = test_chat_runtime_state();
        let mut agent = default_agent();
        agent.memory_recall_mode = MEMORY_RECALL_MODE_OFF.to_string();
        state_write_agents_cached(&state, &[agent.clone(), default_user_persona()])
            .expect("seed agents");
        memory_store_upsert_drafts(
            &state.data_path,
            &[MemoryDraftInput {
                memory_type: "knowledge".to_string(),
                judgment: "用户喜欢猫咪".to_string(),
                reasoning: "回归测试".to_string(),
                tags: vec!["猫咪".to_string()],
                owner_agent_id: None,
            }],
        )
        .expect("seed memory");
        let user_message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "user".to_string(),
            created_at: now_iso(),
            speaker_agent_id: Some(USER_PERSONA_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "猫咪".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        };

        let recall_payload = collect_recall_payload_for_user_message(
            &state.data_path,
            &[agent.clone()],
            &agent.id,
            &user_message,
        )
        .expect("collect recall payload");
        assert!(recall_payload.stored_ids.is_empty());
        assert!(recall_payload.raw_ids.is_empty());

        let memory_context = memory_agent_context_from_agent(&agent)
            .expect("off memory context");
        let recall_result = builtin_recall(&state, &memory_context, "猫咪", None, None, None)
            .expect("off recall");
        assert_eq!(recall_result.get("count").and_then(Value::as_u64), Some(0));
        assert_eq!(recall_result.get("total").and_then(Value::as_u64), Some(0));
        assert_eq!(
            recall_result.get("memoryBoard").and_then(Value::as_str),
            Some("")
        );
    }

    #[test]
    fn builtin_memory_save_should_use_current_tool_agent_as_owner() {
        let state = test_chat_runtime_state();
        let mut assistant = default_agent();
        assistant.private_memory_enabled = true;
        let mut worker = default_agent();
        worker.id = "worker-agent".to_string();
        worker.name = "执行者".to_string();
        worker.private_memory_enabled = true;
        let worker_memory_context =
            memory_agent_context_from_agent(&worker).expect("worker memory context");
        state_write_agents_cached(
            &state,
            &[assistant.clone(), worker.clone(), default_user_persona()],
        )
        .expect("seed agents");

        let result = builtin_memory_save(
            &state,
            &worker_memory_context,
            serde_json::json!({
                "action": "create",
                "memory": {
                    "memoryType": "knowledge",
                    "judgment": "当前任务由执行者人格负责",
                    "reasoning": "回归测试",
                    "tags": ["执行者", "回归"]
                }
            }),
        )
        .expect("save memory");

        assert_eq!(result.get("saved").and_then(Value::as_bool), Some(true));
        let memories = memory_store_list_memories(&state.data_path).expect("list memories");
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].owner_agent_id.as_deref(), Some(worker.id.as_str()));
        assert_eq!(memories[0].judgment, "当前任务由执行者人格负责");
    }

    #[test]
    fn builtin_recall_should_read_memories_for_current_tool_agent_only() {
        let state = test_chat_runtime_state();
        let mut assistant = default_agent();
        assistant.private_memory_enabled = true;
        let mut worker = default_agent();
        worker.id = "worker-agent".to_string();
        worker.name = "执行者".to_string();
        worker.private_memory_enabled = true;
        let worker_memory_context =
            memory_agent_context_from_agent(&worker).expect("worker memory context");
        state_write_agents_cached(
            &state,
            &[assistant.clone(), worker.clone(), default_user_persona()],
        )
        .expect("seed agents");
        memory_store_upsert_drafts(
            &state.data_path,
            &[
                MemoryDraftInput {
                    memory_type: "knowledge".to_string(),
                    judgment: "这是主助理的私有记忆".to_string(),
                    reasoning: "回归测试".to_string(),
                    tags: vec!["共享线索".to_string()],
                    owner_agent_id: Some(assistant.id.clone()),
                },
                MemoryDraftInput {
                    memory_type: "knowledge".to_string(),
                    judgment: "这是执行者的私有记忆".to_string(),
                    reasoning: "回归测试".to_string(),
                    tags: vec!["共享线索".to_string()],
                    owner_agent_id: Some(worker.id.clone()),
                },
            ],
        )
        .expect("seed memories");

        let result = builtin_recall(&state, &worker_memory_context, "共享线索", None, None, None)
            .expect("recall current agent memories");
        let memory_board = result
            .get("memoryBoard")
            .and_then(Value::as_str)
            .unwrap_or_default();

        assert_eq!(result.get("count").and_then(Value::as_u64), Some(1));
        assert!(memory_board.contains("这是执行者的私有记忆"));
        assert!(!memory_board.contains("这是主助理的私有记忆"));
    }

    #[test]
    fn builtin_memory_tools_should_accept_deputy_agent_as_current_persona() {
        let state = test_chat_runtime_state();
        let assistant = default_agent();
        let deputy = default_deputy_agent();
        let deputy_memory_context =
            memory_agent_context_from_agent(&deputy).expect("deputy memory context");
        state_write_agents_cached(
            &state,
            &[assistant, deputy.clone(), default_user_persona(), default_system_persona()],
        )
        .expect("seed agents");

        let save_result = builtin_memory_save(
            &state,
            &deputy_memory_context,
            serde_json::json!({
                "action": "create",
                "memory": {
                    "memoryType": "knowledge",
                    "judgment": "这是副手人格记录的共享记忆",
                    "reasoning": "回归测试",
                    "tags": ["副手回归", "共享"]
                }
            }),
        )
        .expect("save deputy memory");

        assert_eq!(save_result.get("saved").and_then(Value::as_bool), Some(true));

        let recall_result = builtin_recall(&state, &deputy_memory_context, "副手回归", None, None, None)
            .expect("recall deputy memory");
        let memory_board = recall_result
            .get("memoryBoard")
            .and_then(Value::as_str)
            .unwrap_or_default();

        assert_eq!(recall_result.get("count").and_then(Value::as_u64), Some(1));
        assert!(memory_board.contains("这是副手人格记录的共享记忆"));
    }

    #[test]
    fn builtin_memory_tools_should_accept_private_workspace_runtime_agent() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let private_worker = AgentProfile {
            id: "private-worker".to_string(),
            name: "私人执行者".to_string(),
            system_prompt: "你是谁：你是私人执行者。\n台词技巧：直接。\n性格画像：可靠。".to_string(),
            tools: default_agent_tools(),
            created_at: now.clone(),
            updated_at: now,
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: default_private_workspace_source(),
            scope: default_assistant_private_scope(),
        };
        let private_worker_memory_context =
            memory_agent_context_from_agent(&private_worker).expect("private worker memory context");

        let save_result = builtin_memory_save(
            &state,
            &private_worker_memory_context,
            serde_json::json!({
                "action": "create",
                "memory": {
                    "memoryType": "knowledge",
                    "judgment": "这是私有工作区人格记录的共享记忆",
                    "reasoning": "回归测试",
                    "tags": ["私有回归", "共享"]
                }
            }),
        )
        .expect("save private workspace memory");

        assert_eq!(save_result.get("saved").and_then(Value::as_bool), Some(true));

        let recall_result = builtin_recall(&state, &private_worker_memory_context, "私有回归", None, None, None)
            .expect("recall private workspace memory");
        let memory_board = recall_result
            .get("memoryBoard")
            .and_then(Value::as_str)
            .unwrap_or_default();

        assert_eq!(recall_result.get("count").and_then(Value::as_u64), Some(1));
        assert!(memory_board.contains("这是私有工作区人格记录的共享记忆"));
    }

    #[test]
    fn build_prompt_user_meta_text_should_skip_compaction_message_metadata() {
        let now = now_iso();
        let mut message = test_text_message(
            "user",
            "[上下文整理]\n触发原因：manual\n整理摘要：\n用户刚刚确认继续推进。",
            &now,
        );
        message.provider_meta = Some(serde_json::json!({
            "message_meta": {
                "kind": "context_compaction"
            },
            "origin": {
                "kind": "remote_im",
                "channel_id": "remote-im-1",
                "contact_type": "private",
                "contact_id": "contact-42",
                "contact_name": "测试联系人",
                "sender_name": "张三"
            }
        }));

        let meta = build_prompt_user_meta_text(
            &message,
            &[default_agent(), default_user_persona()],
            "用户",
            "zh-CN",
            true,
        );

        assert!(meta.is_none());
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_keep_structured_tool_history_messages() {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![
                PreparedHistoryMessage {
                    role: "user".to_string(),
                    text: "你好".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: Some("[测试用户] 2026-03-18T12:18".to_string()),
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: String::new(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "bing_search", "arguments": "{\"query\":\"rust\"}" }
                    })]),
                    tool_call_id: None,
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"results\":[{\"title\":\"Rust\"}]}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_1".to_string()),
                    reasoning_content: None,
                },
            ],
            latest_user_text: "继续".to_string(),
            latest_user_meta_text: "2026-02-11 17:30:45".to_string(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };
        let messages = prepared_prompt_to_messages_json(&prepared);
        assert!(messages.iter().any(|m| {
            m.get("role").and_then(Value::as_str) == Some("assistant")
                && m.get("tool_calls").and_then(Value::as_array).is_some()
        }));
        assert!(messages.iter().any(|m| {
            m.get("role").and_then(Value::as_str) == Some("tool")
                && m.get("tool_call_id").and_then(Value::as_str) == Some("call_1")
        }));
        assert!(messages.iter().any(|m| {
            m.get("role").and_then(Value::as_str) == Some("user")
                && m.get("content")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.len() == 2
                            && arr[0].get("type").and_then(Value::as_str) == Some("text")
                            && arr[0].get("text").and_then(Value::as_str) == Some("你好")
                            && arr[1].get("type").and_then(Value::as_str) == Some("text")
                            && arr[1].get("text").and_then(Value::as_str)
                                == Some("[测试用户] 2026-03-18T12:18")
                    })
                    .unwrap_or(false)
        }));
    }

    #[test]
    fn build_prompt_should_not_extract_latest_user_when_tail_is_assistant() {
        let now = now_iso();
        let agent = default_agent();
        let mut user_message = test_text_message("user", "现在时间是多少？", &now);
        user_message.speaker_agent_id = None;
        let mut assistant_message = test_text_message("assistant", "2026-03-30 00:26（+08:00）", &now);
        assistant_message.speaker_agent_id = Some(agent.id.clone());
        let messages = vec![user_message, assistant_message];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert!(prepared.latest_user_text.trim().is_empty());
        assert_eq!(prepared.history_messages.len(), 2);
        assert_eq!(prepared.history_messages[0].role, "user");
        assert_eq!(prepared.history_messages[1].role, "assistant");
    }

    #[test]
    fn build_prompt_should_merge_adjacent_plain_assistant_history_messages() {
        let now = now_iso();
        let agent = default_agent();
        let mut user_message = test_text_message("user", "先听我说", &now);
        user_message.speaker_agent_id = None;
        let mut assistant_message_1 = test_text_message("assistant", "第一段回复", &now);
        assistant_message_1.speaker_agent_id = Some(agent.id.clone());
        let mut assistant_message_2 = test_text_message("assistant", "第二段补充", &now);
        assistant_message_2.speaker_agent_id = Some(agent.id.clone());
        let messages = vec![user_message, assistant_message_1, assistant_message_2];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 2);
        assert_eq!(prepared.history_messages[0].role, "user");
        assert_eq!(prepared.history_messages[1].role, "assistant");
        assert_eq!(prepared.history_messages[1].text, "第一段回复\n\n第二段补充");
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_keep_tool_call_reasoning_per_assistant_message(
    ) {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![
                PreparedHistoryMessage {
                    role: "user".to_string(),
                    text: "查一下结果".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: "我先调用工具".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "search_docs",
                            "arguments": "{\"q\":\"结果\"}"
                        }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("先查资料".to_string()),
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: "工具结果我看完了".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_2",
                        "type": "function",
                        "function": {
                            "name": "resolve_link",
                            "arguments": "{\"query\":\"结果详情\"}"
                        }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("再补一轮定位".to_string()),
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"ok\":true}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_2".to_string()),
                    reasoning_content: None,
                },
            ],
            latest_user_text: String::new(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let messages = prepared_prompt_to_messages_json(&prepared);

        assert_eq!(messages.len(), 5);
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(
            messages[2]["content"].as_str(),
            Some("我先调用工具")
        );
        assert_eq!(
            messages[2]["reasoning_content"].as_str(),
            Some("先查资料")
        );
        assert_eq!(
            messages[2]["tool_calls"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(messages[3]["role"], "assistant");
        assert_eq!(
            messages[3]["content"].as_str(),
            Some("工具结果我看完了")
        );
        assert_eq!(
            messages[3]["reasoning_content"].as_str(),
            Some("再补一轮定位")
        );
        assert_eq!(
            messages[3]["tool_calls"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(messages[4]["role"], "tool");
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_keep_reasoning_for_four_tool_rounds_in_order() {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![
                PreparedHistoryMessage {
                    role: "user".to_string(),
                    text: "继续完成任务".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: String::new(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "read_file", "arguments": "{\"path\":\"a.md\"}" }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("第1轮思考".to_string()),
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"ok\":true,\"step\":1}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_1".to_string()),
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: String::new(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_2",
                        "type": "function",
                        "function": { "name": "grep", "arguments": "{\"pattern\":\"quest\"}" }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("第2轮思考".to_string()),
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"ok\":true,\"step\":2}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_2".to_string()),
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: String::new(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_3",
                        "type": "function",
                        "function": { "name": "http", "arguments": "{\"url\":\"/quests\"}" }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("第3轮思考".to_string()),
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"ok\":true,\"step\":3}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_3".to_string()),
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: String::new(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: Some(vec![serde_json::json!({
                        "id": "call_4",
                        "type": "function",
                        "function": { "name": "write_file", "arguments": "{\"path\":\"out.md\"}" }
                    })]),
                    tool_call_id: None,
                    reasoning_content: Some("第4轮思考".to_string()),
                },
                PreparedHistoryMessage {
                    role: "tool".to_string(),
                    text: "{\"ok\":true,\"step\":4}".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: Some("call_4".to_string()),
                    reasoning_content: None,
                },
            ],
            latest_user_text: "继续下一步".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let messages = prepared_prompt_to_messages_json(&prepared);
        let assistant_tool_reasonings = messages
            .iter()
            .filter(|message| {
                message.get("role").and_then(Value::as_str) == Some("assistant")
                    && message.get("tool_calls").and_then(Value::as_array).is_some()
            })
            .map(|message| {
                message
                    .get("reasoning_content")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            assistant_tool_reasonings,
            vec![
                "第1轮思考".to_string(),
                "第2轮思考".to_string(),
                "第3轮思考".to_string(),
                "第4轮思考".to_string(),
            ]
        );
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_keep_reasoning_for_plain_assistant_messages() {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![PreparedHistoryMessage {
                role: "assistant".to_string(),
                text: "这是结论".to_string(),
                extra_text_blocks: Vec::new(),
                user_time_text: None,
                images: Vec::new(),
                audios: Vec::new(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: Some("这是思考过程".to_string()),
            }],
            latest_user_text: String::new(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let messages = prepared_prompt_to_messages_json(&prepared);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"].as_str(), Some("这是结论"));
        assert_eq!(
            messages[1]["reasoning_content"].as_str(),
            Some("这是思考过程")
        );
    }

    #[test]
    fn build_prompt_should_replay_clean_tool_reasoning_chain_on_new_dispatch() {
        let now = now_iso();
        let agent = default_agent();
        let mut assistant = test_text_message("assistant", "我已经完成了工具阶段", &now);
        assistant.speaker_agent_id = Some(agent.id.clone());
        assistant.tool_call = Some(vec![
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "第1次请求返回的思考",
                "tool_calls": [{
                    "id": "fc_1",
                    "type": "function",
                    "function": { "name": "read_file", "arguments": "{\"path\":\"a.md\"}" }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "fc_1",
                "content": "{\"ok\":true,\"step\":1}"
            }),
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "第2次请求返回的思考",
                "tool_calls": [{
                    "id": "fc_2",
                    "type": "function",
                    "function": { "name": "grep", "arguments": "{\"pattern\":\"quest\"}" }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "fc_2",
                "content": "{\"ok\":true,\"step\":2}"
            }),
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "第3次请求返回的思考",
                "tool_calls": [{
                    "id": "fc_3",
                    "type": "function",
                    "function": { "name": "http", "arguments": "{\"url\":\"/quests\"}" }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "fc_3",
                "content": "{\"ok\":true,\"step\":3}"
            }),
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "第4次请求返回的思考",
                "tool_calls": [{
                    "id": "fc_4",
                    "type": "function",
                    "function": { "name": "write_file", "arguments": "{\"path\":\"out.md\"}" }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "fc_4",
                "content": "{\"ok\":true,\"step\":4}"
            }),
        ]);
        let messages = vec![
            test_text_message("user", "先帮我查 quest API", &now),
            assistant,
            test_text_message("user", "现在继续下一次调度", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        let tool_reasonings = prepared
            .history_messages
            .iter()
            .filter(|message| {
                message.role == "assistant"
                    && message
                        .tool_calls
                        .as_ref()
                        .map(|calls| !calls.is_empty())
                        .unwrap_or(false)
            })
            .map(|message| message.reasoning_content.clone().unwrap_or_default())
            .collect::<Vec<_>>();

        assert_eq!(
            tool_reasonings,
            vec![
                "第1次请求返回的思考".to_string(),
                "第2次请求返回的思考".to_string(),
                "第3次请求返回的思考".to_string(),
                "第4次请求返回的思考".to_string(),
            ]
        );
        assert!(tool_reasonings.iter().all(|value| value != "整轮累计思考A。整轮累计思考A。整轮累计思考A。"));
        assert!(
            prepared.history_messages.iter().any(|message| {
                message.role == "assistant"
                    && message.text == "我已经完成了工具阶段"
                    && message.reasoning_content.is_none()
            })
        );
    }

    #[test]
    fn build_prompt_should_replay_persisted_real_tool_message_into_request_preview_shape() {
        let agent = default_agent();
        let raw: Value = serde_json::from_str(
            r#"{
                "kind":"message",
                "message":{
                    "id":"926b767d-c730-4e20-bfba-e57a84ee40b9",
                    "role":"assistant",
                    "createdAt":"2026-05-08T11:44:08Z",
                    "speakerAgentId":"persona-1776445616370",
                    "parts":[
                        {
                            "type":"text",
                            "text":"这是一张你发来的截图，显示的是我们的聊天界面和刚才查询终端版本的对话内容。看起来你想让我看什么？\n\n让我再仔细看看截图内容——它展示的就是我们刚刚这段对话：你问我终端版本，我执行了 `pwsh --version`，返回了 PowerShell 7.5.4 的信息。\n\n你想确认什么？还是截错了？"
                        }
                    ],
                    "extraTextBlocks":[],
                    "toolCall":[
                        {
                            "content":null,
                            "reasoning_content":"用户想让我调用一次终端命令，查看版本信息。这应该是指操作系统的终端信息，比如 PowerShell 版本之类的。",
                            "role":"assistant",
                            "tool_calls":[
                                {
                                    "call_id":"call_915742b663134f368df81c7d",
                                    "function":{
                                        "arguments":"{\"command\":\"pwsh --version; Write-Host \\\"---\\\"; $PSVersionTable; Write-Host \\\"---\\\"; [System.Environment]::OSVersion.VersionString\"}",
                                        "name":"exec"
                                    },
                                    "id":"call_915742b663134f368df81c7d",
                                    "type":"function"
                                }
                            ]
                        },
                        {
                            "content":"{\"durationMs\":764,\"exitCode\":0,\"ok\":true,\"stderr\":\"\",\"stderrTruncated\":false,\"stdout\":\"PowerShell 7.5.4\\r\\n---\\r\\nPSVersion 7.5.4\\r\\n---\\r\\nMicrosoft Windows NT 10.0.26200.0\\r\\n\",\"stdoutTruncated\":false,\"timedOut\":false,\"truncated\":false}",
                            "role":"tool",
                            "tool_call_id":"call_915742b663134f368df81c7d"
                        }
                    ],
                    "mcpCall":null
                }
            }"#,
        )
        .expect("real stored message json should parse");
        let mut assistant: ChatMessage = serde_json::from_value(raw["message"].clone())
            .expect("chat message should deserialize from stored json");
        assistant.speaker_agent_id = Some(agent.id.clone());

        let messages = vec![
            test_text_message("user", "你先调用一次工具，看看终端版本，然后告诉我是什么", "2026-05-08T11:43:52Z"),
            assistant,
            test_text_message("user", "继续", "2026-05-08T11:45:00Z"),
        ];
        let conv = test_active_conversation_with_messages(messages, Some("2026-05-08T11:45:00Z".to_string()));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert!(prepared.history_messages.iter().any(|message| {
            message.role == "assistant"
                && message
                    .tool_calls
                    .as_ref()
                    .map(|calls| calls.len() == 1)
                    .unwrap_or(false)
                && message.reasoning_content.as_deref() == Some("用户想让我调用一次终端命令，查看版本信息。这应该是指操作系统的终端信息，比如 PowerShell 版本之类的。")
        }));
        assert!(prepared.history_messages.iter().any(|message| {
            message.role == "tool"
                && message.tool_call_id.as_deref() == Some("call_915742b663134f368df81c7d")
                && message.text.contains("PowerShell 7.5.4")
        }));
        assert!(prepared.history_messages.iter().any(|message| {
            message.role == "assistant"
                && message.text.contains("这是一张你发来的截图")
                && message.reasoning_content.as_deref() == Some("用户想让我调用一次终端命令，查看版本信息。这应该是指操作系统的终端信息，比如 PowerShell 版本之类的。")
        }));

        let request_messages = prepared_prompt_to_messages_json(&prepared);
        assert!(request_messages.iter().any(|message| {
            message.get("role").and_then(Value::as_str) == Some("assistant")
                && message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|calls| calls.len() == 1)
                    .unwrap_or(false)
                && message
                    .get("reasoning_content")
                    .and_then(Value::as_str)
                    == Some("用户想让我调用一次终端命令，查看版本信息。这应该是指操作系统的终端信息，比如 PowerShell 版本之类的。")
        }));
        assert!(request_messages.iter().any(|message| {
            message.get("role").and_then(Value::as_str) == Some("tool")
                && message.get("tool_call_id").and_then(Value::as_str)
                    == Some("call_915742b663134f368df81c7d")
        }));
        assert!(request_messages.iter().any(|message| {
            message.get("role").and_then(Value::as_str) == Some("assistant")
                && message.get("content").and_then(Value::as_str).is_some_and(|text| text.contains("这是一张你发来的截图"))
        }));
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_omit_empty_latest_user_turn() {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![
                PreparedHistoryMessage {
                    role: "user".to_string(),
                    text: "现在时间是多少？".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
                PreparedHistoryMessage {
                    role: "assistant".to_string(),
                    text: "2026-03-30 00:26（+08:00）".to_string(),
                    extra_text_blocks: Vec::new(),
                    user_time_text: None,
                    images: Vec::new(),
                    audios: Vec::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
            ],
            latest_user_text: String::new(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let messages = prepared_prompt_to_messages_json(&prepared);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].get("role").and_then(Value::as_str), Some("user"));
        assert_eq!(messages[2].get("role").and_then(Value::as_str), Some("assistant"));
    }

    #[test]
    fn build_prompt_should_prefer_conversation_bound_agent_over_passed_agent() {
        let now = now_iso();
        let agent = default_agent();
        let mut wrong_agent = default_agent();
        wrong_agent.id = "another-agent".to_string();
        wrong_agent.name = "另一个人格".to_string();
        let mut assistant = test_text_message("assistant", "我本来是一条助手消息", &now);
        assistant.speaker_agent_id = Some(agent.id.clone());
        assistant.tool_call = Some(vec![
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "先调用工具",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "exec",
                        "arguments": "{\"command\":\"pwsh --version\"}"
                    }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "PowerShell 7.5.4"
            }),
        ]);
        let messages = vec![
            test_text_message("user", "上一轮", &now),
            assistant,
            test_text_message("user", "继续", &now),
        ];
        let mut conv = test_active_conversation_with_messages(messages, Some(now));
        conv.agent_id = agent.id.clone();
        conv.department_id = "dept-a".to_string();

        let prepared = build_prompt(
            &conv,
            &wrong_agent,
            &[agent.clone(), wrong_agent.clone(), default_user_persona()],
            &[DepartmentConfig {
                id: "dept-a".to_string(),
                name: "部门 A".to_string(),
                summary: String::new(),
                guide: String::new(),
                api_config_ids: vec!["provider-a".to_string()],
                api_config_id: "provider-a".to_string(),
                model_failure_fallback_enabled: false,
                agent_ids: vec![agent.id.clone()],
                child_department_ids: Vec::new(),
                created_at: now_utc_rfc3339(),
                updated_at: now_utc_rfc3339(),
                order_index: 1,
                is_built_in_assistant: false,
                is_deputy: false,
                source: "main_config".to_string(),
                scope: "global".to_string(),
                permission_control: DepartmentPermissionControl::default(),
            }],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert!(prepared.history_messages.iter().any(|message| {
            message.role == "assistant"
                && message
                    .tool_calls
                    .as_ref()
                    .map(|calls| !calls.is_empty())
                    .unwrap_or(false)
                && message.reasoning_content.as_deref() == Some("先调用工具")
        }));
        assert!(prepared.history_messages.iter().all(|message| {
            !(message.role == "user" && message.text == "我本来是一条助手消息")
        }));
    }

    #[test]
    fn resolve_conversation_bound_agent_should_fallback_to_department_when_agent_missing() {
        let now = now_iso();
        let agent = default_agent();
        let mut conv = test_active_conversation_with_messages(Vec::new(), Some(now));
        conv.agent_id = "missing-agent".to_string();
        conv.department_id = "dept-a".to_string();
        let agents = vec![agent.clone(), default_user_persona()];
        let departments = vec![DepartmentConfig {
            id: "dept-a".to_string(),
            name: "部门 A".to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: vec!["provider-a".to_string()],
            api_config_id: "provider-a".to_string(),
            model_failure_fallback_enabled: false,
            agent_ids: vec![agent.id.clone()],
            child_department_ids: Vec::new(),
            created_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
            order_index: 1,
            is_built_in_assistant: false,
            is_deputy: false,
            source: "main_config".to_string(),
            scope: "global".to_string(),
            permission_control: DepartmentPermissionControl::default(),
        }];

        let resolved = resolve_conversation_bound_agent(&conv, &agents, &departments)
            .expect("department fallback should resolve bound agent");

        assert_eq!(resolved.id, agent.id);
    }

    #[test]
    fn resolve_conversation_bound_agent_should_error_when_agent_and_department_missing() {
        let now = now_iso();
        let agent = default_agent();
        let mut conv = test_active_conversation_with_messages(Vec::new(), Some(now));
        conv.agent_id = "missing-agent".to_string();
        conv.department_id = String::new();

        let err = resolve_conversation_bound_agent(
            &conv,
            &[agent, default_user_persona()],
            &[],
        )
        .expect_err("missing agent and department should fail");

        assert!(err.contains("会话缺少有效人格绑定"));
    }

    #[test]
    fn build_stop_chat_partial_assistant_message_should_keep_final_reasoning_separate_from_tool_round_reasoning(
    ) {
        let message = build_stop_chat_partial_assistant_message(
            "agent-a",
            "终端版本是 PowerShell 7.5.4。",
            "先调用终端工具查看 PowerShell 版本。我已经拿到工具结果，现在直接回答用户终端版本。",
            "",
            &[
                serde_json::json!({
                    "role": "assistant",
                    "content": Value::Null,
                    "reasoning_content": "先调用终端工具查看 PowerShell 版本。",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "exec",
                            "arguments": "{\"command\":\"pwsh --version\"}"
                        }
                    }]
                }),
                serde_json::json!({
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "PowerShell 7.5.4"
                }),
            ],
        );

        assert!(message.provider_meta.is_none());
        assert_eq!(
            message
                .tool_call
                .as_ref()
                .and_then(|events| events.first())
                .and_then(|event| event.get("reasoning_content"))
                .and_then(Value::as_str),
            Some("先调用终端工具查看 PowerShell 版本。")
        );
    }

    #[test]
    fn build_stop_chat_partial_assistant_message_should_not_promote_tool_reasoning_when_final_text_missing(
    ) {
        let message = build_stop_chat_partial_assistant_message(
            "agent-a",
            "",
            "第1轮先调用工具",
            "",
            &[
                serde_json::json!({
                    "role": "assistant",
                    "content": Value::Null,
                    "reasoning_content": "第1轮先调用工具",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"README.md\"}"
                        }
                    }]
                }),
                serde_json::json!({
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "{\"ok\":true}"
                }),
            ],
        );

        assert!(message.provider_meta.is_none());
        assert_eq!(
            message
                .tool_call
                .as_ref()
                .and_then(|events| events.first())
                .and_then(|event| event.get("reasoning_content"))
                .and_then(Value::as_str),
            Some("第1轮先调用工具")
        );
    }

    #[test]
    fn build_stop_chat_partial_assistant_message_should_keep_reasoning_only_final_turn_when_text_not_arrived_yet(
    ) {
        let message = build_stop_chat_partial_assistant_message(
            "agent-a",
            "",
            "第1轮先调用工具\n\n我已经拿到结果，准备组织最终答复。",
            "",
            &[
                serde_json::json!({
                    "role": "assistant",
                    "content": Value::Null,
                    "reasoning_content": "第1轮先调用工具",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"README.md\"}"
                        }
                    }]
                }),
                serde_json::json!({
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "{\"ok\":true}"
                }),
            ],
        );

        assert!(message.provider_meta.is_none());
        match message.parts.first() {
            Some(MessagePart::Text { text, .. }) => assert!(text.is_empty()),
            other => panic!("unexpected message part: {:?}", other),
        }
        assert_eq!(
            message
                .tool_call
                .as_ref()
                .and_then(|events| events.first())
                .and_then(|event| event.get("reasoning_content"))
                .and_then(Value::as_str),
            Some("第1轮先调用工具")
        );
    }

    #[test]
    fn build_prompt_should_prefix_latest_user_text_with_mentions() {
        let now = now_iso();
        let agent = default_agent();
        let mut user_message = test_text_message("user", "请你看看这个方案", &now);
        user_message.provider_meta = Some(serde_json::json!({
            "message_meta": {
                "kind": "user_message",
                "mentions": [
                    {
                        "agentId": "agent-fairy",
                        "agentName": "fairy",
                        "departmentId": "department-fairy",
                        "departmentName": "测试部门"
                    },
                    {
                        "agentId": "agent-zhongli",
                        "agentName": "钟离",
                        "departmentId": "department-zhongli",
                        "departmentName": "璃月顾问组"
                    }
                ]
            }
        }));
        let conv = test_active_conversation_with_messages(vec![user_message], Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.latest_user_text, "@fairy,@钟离\n请你看看这个方案");
    }

    #[test]
    fn prepared_prompt_to_messages_json_should_keep_mention_prefix_for_latest_user() {
        let prepared = PreparedPrompt {
            preamble: "sys".to_string(),
            history_messages: vec![PreparedHistoryMessage {
                role: "assistant".to_string(),
                text: "收到".to_string(),
                extra_text_blocks: Vec::new(),
                user_time_text: None,
                images: Vec::new(),
                audios: Vec::new(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            latest_user_text: "@fairy,@钟离\n请你看看这个方案".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let messages = prepared_prompt_to_messages_json(&prepared);
        let latest_user = messages
            .last()
            .and_then(Value::as_object)
            .cloned()
            .expect("latest user message should exist");
        assert_eq!(latest_user.get("role").and_then(Value::as_str), Some("user"));
        let content = latest_user
            .get("content")
            .and_then(Value::as_str)
            .or_else(|| {
                latest_user
                    .get("content")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                    .and_then(|item| item.get("text"))
                    .and_then(Value::as_str)
            });
        assert_eq!(content, Some("@fairy,@钟离\n请你看看这个方案"));
    }

    #[test]
    fn build_prompt_should_not_duplicate_compaction_message_into_latest_user_text() {
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            test_text_message("user", "第一轮用户原始消息", &now),
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "[上下文整理]\n触发原因：force_context_usage_82_after_reply\n整理摘要：\n保留关键上下文。".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "message_meta": {
                        "kind": "context_compaction",
                        "scene": "compaction",
                        "reason": "force_context_usage_82_after_reply"
                    }
                })),
                tool_call: None,
                mcp_call: None,
            },
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 1);
        assert!(prepared.history_messages[0].text.contains("[上下文整理]"));
        assert!(prepared.latest_user_text.trim().is_empty());
        assert!(prepared.latest_user_meta_text.trim().is_empty());
        assert_eq!(prepared.history_messages[0].role, "user");
    }

    #[test]
    fn build_prompt_should_only_keep_last_compaction_message_as_boundary() {
        let now = now_iso();
        let agent = default_agent();
        let mut trailing_assistant = test_text_message("assistant", "摘要后的助手消息", &now);
        trailing_assistant.speaker_agent_id = Some(agent.id.clone());
        let messages = vec![
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "[上下文整理]\n旧摘要".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "message_meta": {
                        "kind": "summary_context_seed",
                        "scene": "seed",
                    }
                })),
                tool_call: None,
                mcp_call: None,
            },
            test_text_message("user", "中间用户消息", &now),
            test_text_message("assistant", "中间助手消息", &now),
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "[上下文整理]\n新摘要".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "message_meta": {
                        "kind": "context_compaction",
                        "scene": "compaction",
                        "reason": "manual"
                    }
                })),
                tool_call: None,
                mcp_call: None,
            },
            trailing_assistant,
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 2);
        assert!(prepared.history_messages[0].text.contains("新摘要"));
        assert!(!prepared.history_messages[0].text.contains("旧摘要"));
        assert_eq!(prepared.history_messages[1].text, "摘要后的助手消息");
    }

    #[test]
    fn build_prompt_after_compaction_should_not_replay_pre_compaction_checkpoint_tool_history() {
        let now = now_iso();
        let agent = default_agent();
        let checkpoint = build_stop_chat_partial_assistant_message(
            &agent.id,
            "我已经读取完文件，准备继续处理。",
            "先读取文件，再继续总结。",
            "",
            &[
                serde_json::json!({
                    "role": "assistant",
                    "content": Value::Null,
                    "reasoning_content": "先读取文件",
                    "tool_calls": [{
                        "id": "call_read",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"README.md\"}"
                        }
                    }]
                }),
                serde_json::json!({
                    "role": "tool",
                    "tool_call_id": "call_read",
                    "content": "README 内容"
                }),
            ],
        );
        let compaction = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "user".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "[上下文整理]\n整理摘要：已读取 README，接下来继续处理。".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "message_meta": {
                    "kind": "context_compaction",
                    "scene": "compaction",
                    "reason": "organize_context"
                }
            })),
            tool_call: None,
            mcp_call: None,
        };
        let messages = vec![
            test_text_message("user", "请读取 README 并继续处理", &now),
            checkpoint,
            compaction,
            test_text_message("user", "继续", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.latest_user_text, "继续");
        assert_eq!(prepared.history_messages.len(), 1);
        assert!(prepared.history_messages[0].text.contains("已读取 README"));
        assert!(
            prepared
                .history_messages
                .iter()
                .all(|message| message.tool_calls.is_none())
        );
        assert!(
            prepared
                .history_messages
                .iter()
                .all(|message| !message.text.contains("README 内容"))
        );
    }

    #[test]
    fn build_prompt_should_resolve_latest_user_from_trimmed_context_window() {
        let now = now_iso();
        let agent = default_agent();
        let mut trailing_assistant = test_text_message("assistant", "收到，我继续处理", &now);
        trailing_assistant.speaker_agent_id = Some(agent.id.clone());
        let messages = vec![
            test_text_message("user", "这是很久之前的超长历史消息，不应再参与本轮提示词", &now),
            test_text_message("assistant", "这是旧助手回复", &now),
            ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "[上下文整理]\n只保留最近有效上下文".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "message_meta": {
                        "kind": "context_compaction",
                        "scene": "compaction",
                        "reason": "manual"
                    }
                })),
                tool_call: None,
                mcp_call: None,
            },
            trailing_assistant,
            test_text_message("user", "这是压缩后的最新用户消息", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.latest_user_text, "这是压缩后的最新用户消息");
        assert_eq!(prepared.history_messages.len(), 2);
        assert!(
            prepared.history_messages[0]
                .text
                .contains("只保留最近有效上下文")
        );
        assert_eq!(prepared.history_messages[1].text, "收到，我继续处理");
        assert!(
            prepared
                .history_messages
                .iter()
                .all(|message| !message.text.contains("很久之前的超长历史消息"))
        );
    }

    #[test]
    fn build_prompt_should_not_treat_normal_message_with_compaction_phrase_as_compaction_boundary() {
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            test_text_message("user", "第一轮用户原始消息", &now),
            test_text_message("assistant", "第一轮助手回复", &now),
            test_text_message(
                "user",
                "plan 写入 markdown，是为了防止上下文压缩之后，计划被压缩掉了的设计。",
                &now,
            ),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 2);
        assert_eq!(prepared.history_messages[0].text, "第一轮用户原始消息");
        assert_eq!(prepared.history_messages[1].text, "第一轮助手回复");
        assert!(prepared
            .latest_user_text
            .contains("防止上下文压缩之后"));
    }

    #[test]
    fn build_prompt_should_not_treat_prefix_only_message_without_meta_as_compaction_boundary() {
        let now = now_iso();
        let agent = default_agent();
        let messages = vec![
            test_text_message("user", "第一轮用户原始消息", &now),
            test_text_message("assistant", "第一轮助手回复", &now),
            test_text_message("user", "[上下文整理]\n这只是普通文本，不是系统压缩消息。", &now),
        ];
        let conv = test_active_conversation_with_messages(messages, Some(now));

        let prepared = build_prompt(
            &conv,
            &agent,
            &[agent.clone(), default_user_persona()],
            &[],
            "用户",
            "我是...",
            DEFAULT_RESPONSE_STYLE_ID,
            "zh-CN",
            None,
            None,
            None,
            false,
        );

        assert_eq!(prepared.history_messages.len(), 2);
        assert!(prepared.latest_user_text.contains("这只是普通文本"));
    }

    #[test]
    fn build_remote_im_activation_runtime_block_should_warn_multiple_sources_no_auto_send() {
        let sources = vec![
            RemoteImActivationSource {
                channel_id: "remote-im-a".to_string(),
                platform: RemoteImPlatform::OnebotV11,
                remote_contact_type: "private".to_string(),
                remote_contact_id: "contact-a".to_string(),
                remote_contact_name: "张三".to_string(),
            },
            RemoteImActivationSource {
                channel_id: "remote-im-b".to_string(),
                platform: RemoteImPlatform::Dingtalk,
                remote_contact_type: "private".to_string(),
                remote_contact_id: "contact-b".to_string(),
                remote_contact_name: "李四".to_string(),
            },
        ];

        let block =
            build_remote_im_activation_runtime_block(&sources, "zh-CN").expect("runtime block");

        assert!(block.contains("多个远程 IM 来源共同激活"));
        assert!(block.contains("系统不会自动外发本轮最终回复"));
        assert!(block.contains("channel_id=remote-im-a"));
        assert!(block.contains("channel_id=remote-im-b"));
    }

    #[test]
    fn resolve_remote_im_auto_send_target_should_only_auto_send_single_source_without_no_reply() {
        let single_source = RemoteImActivationSource {
            channel_id: "remote-im-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-a".to_string(),
            remote_contact_name: "张三".to_string(),
        };
        let single_target = resolve_remote_im_auto_send_target(
            "你好，这里是最终回复。",
            &[single_source.clone()],
            None,
        )
        .expect("single target");
        assert_eq!(single_target, Some(single_source.clone()));

        let multiple_sources = resolve_remote_im_auto_send_target(
            "你好，这里是最终回复。",
            &[
                single_source,
                RemoteImActivationSource {
                    channel_id: "remote-im-b".to_string(),
                    platform: RemoteImPlatform::Dingtalk,
                    remote_contact_type: "private".to_string(),
                    remote_contact_id: "contact-b".to_string(),
                    remote_contact_name: "李四".to_string(),
                },
            ],
            None,
        )
        .expect("multiple sources should skip auto send");
        assert!(multiple_sources.is_none());

        let no_reply_decision = RemoteImReplyDecisionSummary {
            action: "no_reply".to_string(),
            target: None,
        };
        let no_reply_target = resolve_remote_im_auto_send_target(
            "你好，这里是最终回复。",
            &[RemoteImActivationSource {
                channel_id: "remote-im-a".to_string(),
                platform: RemoteImPlatform::OnebotV11,
                remote_contact_type: "private".to_string(),
                remote_contact_id: "contact-a".to_string(),
                remote_contact_name: "张三".to_string(),
            }],
            Some(&no_reply_decision),
        )
        .expect("no_reply should suppress auto send");
        assert!(no_reply_target.is_none());
    }

    #[test]
    fn resolve_bound_remote_im_activation_source_should_only_bind_single_source() {
        let single_source = RemoteImActivationSource {
            channel_id: "remote-im-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-a".to_string(),
            remote_contact_name: "张三".to_string(),
        };

        assert_eq!(
            resolve_bound_remote_im_activation_source(std::slice::from_ref(&single_source)),
            Some(single_source.clone())
        );
        assert!(resolve_bound_remote_im_activation_source(&[]).is_none());
        assert!(resolve_bound_remote_im_activation_source(&[
            single_source,
            RemoteImActivationSource {
                channel_id: "remote-im-b".to_string(),
                platform: RemoteImPlatform::Dingtalk,
                remote_contact_type: "private".to_string(),
                remote_contact_id: "contact-b".to_string(),
                remote_contact_name: "李四".to_string(),
            }
        ])
        .is_none());
    }

    #[test]
    fn collect_activated_remote_im_sources_should_dedup_same_contact_and_ignore_inactive_events() {
        let created_at = now_iso();
        let remote_sender_a = RemoteImMessageSource {
            channel_id: "remote-im-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "QQ".to_string(),
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-a".to_string(),
            remote_contact_name: "张三".to_string(),
            sender_id: "contact-a".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: None,
            platform_message_id: None,
        };
        let remote_sender_b = RemoteImMessageSource {
            channel_id: "remote-im-b".to_string(),
            platform: RemoteImPlatform::Dingtalk,
            im_name: "钉钉".to_string(),
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-b".to_string(),
            remote_contact_name: "李四".to_string(),
            sender_id: "contact-b".to_string(),
            sender_name: "李四".to_string(),
            sender_avatar_url: None,
            platform_message_id: None,
        };
        let events = vec![
            ChatPendingEvent {
                id: Uuid::new_v4().to_string(),
                conversation_id: "conversation-a".to_string(),
                created_at: created_at.clone(),
                source: ChatEventSource::RemoteIm,
                queue_mode: ChatQueueMode::Normal,
                messages: vec![test_text_message("user", "来自张三的第一条消息", &created_at)],
                activate_assistant: true,
                session_info: ChatSessionInfo {
                    department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                    agent_id: DEFAULT_AGENT_ID.to_string(),
                },
                runtime_context: None,
                sender_info: Some(remote_sender_a.clone()),
            },
            ChatPendingEvent {
                id: Uuid::new_v4().to_string(),
                conversation_id: "conversation-a".to_string(),
                created_at: created_at.clone(),
                source: ChatEventSource::RemoteIm,
                queue_mode: ChatQueueMode::Normal,
                messages: vec![test_text_message("user", "来自张三的第二条消息", &created_at)],
                activate_assistant: true,
                session_info: ChatSessionInfo {
                    department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                    agent_id: DEFAULT_AGENT_ID.to_string(),
                },
                runtime_context: None,
                sender_info: Some(remote_sender_a),
            },
            ChatPendingEvent {
                id: Uuid::new_v4().to_string(),
                conversation_id: "conversation-a".to_string(),
                created_at: created_at.clone(),
                source: ChatEventSource::RemoteIm,
                queue_mode: ChatQueueMode::Normal,
                messages: vec![test_text_message("user", "来自李四的消息", &created_at)],
                activate_assistant: true,
                session_info: ChatSessionInfo {
                    department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                    agent_id: DEFAULT_AGENT_ID.to_string(),
                },
                runtime_context: None,
                sender_info: Some(remote_sender_b),
            },
            ChatPendingEvent {
                id: Uuid::new_v4().to_string(),
                conversation_id: "conversation-a".to_string(),
                created_at,
                source: ChatEventSource::User,
                queue_mode: ChatQueueMode::Normal,
                messages: vec![test_text_message("user", "普通用户消息", &now_iso())],
                activate_assistant: true,
                session_info: ChatSessionInfo {
                    department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                    agent_id: DEFAULT_AGENT_ID.to_string(),
                },
                runtime_context: None,
                sender_info: None,
            },
        ];

        let sources =
            collect_activated_remote_im_sources(&events, &[true, true, false, true]);

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].channel_id, "remote-im-a");
        assert_eq!(sources[0].remote_contact_id, "contact-a");

        let all_sources =
            collect_activated_remote_im_sources(&events, &[true, true, true, true]);
        assert_eq!(all_sources.len(), 2);
        assert_eq!(all_sources[0].channel_id, "remote-im-a");
        assert_eq!(all_sources[1].channel_id, "remote-im-b");
    }

    #[test]
    fn filter_remote_im_follow_up_sources_should_wait_for_pending_queue_message() {
        let state = test_chat_runtime_state();
        let created_at = now_iso();
        let remote_sender = RemoteImMessageSource {
            channel_id: "remote-im-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "QQ".to_string(),
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-a".to_string(),
            remote_contact_name: "张三".to_string(),
            sender_id: "contact-a".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: None,
            platform_message_id: None,
        };
        let source = remote_im_activation_source_from_sender(&remote_sender);
        let event = ChatPendingEvent {
            id: Uuid::new_v4().to_string(),
            conversation_id: "conversation-a".to_string(),
            created_at: created_at.clone(),
            source: ChatEventSource::RemoteIm,
            queue_mode: ChatQueueMode::Normal,
            messages: vec![test_text_message("user", "忙碌期间来的新消息", &created_at)],
            activate_assistant: true,
            session_info: ChatSessionInfo {
                department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                agent_id: DEFAULT_AGENT_ID.to_string(),
            },
            runtime_context: None,
            sender_info: Some(remote_sender),
        };
        {
            let mut slots = state
                .conversation_runtime_slots
                .lock()
                .expect("lock runtime slots");
            let slot = conversation_slot_mut(&mut slots, "conversation-a");
            slot.pending_queue.push_back(event);
        }

        assert!(remote_im_source_has_pending_queue_event(
            &state,
            "conversation-a",
            &source,
        ));
        let filtered = filter_remote_im_follow_up_sources_for_pending_queue(
            &state,
            "conversation-a",
            vec![source],
        );
        assert!(filtered.is_empty());
    }

    fn seed_remote_im_auto_send_test_state(
        channel_credentials: Value,
    ) -> (AppState, RemoteImActivationSource, String, String, String) {
        let state = test_chat_runtime_state();
        let mut config = AppConfig::default();
        config.remote_im_channels.push(RemoteImChannelConfig {
            id: "remote-im-a".to_string(),
            name: "测试渠道".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            enabled: true,
            credentials: channel_credentials,
            activate_assistant: true,
            receive_files: true,
            streaming_send: false,
            show_tool_calls: false,
            allow_send_files: false,
        });
        write_config(&state.config_path, &config).expect("write config");

        let conversation_id = "conversation-a".to_string();
        let assistant_message_id = Uuid::new_v4().to_string();
        let assistant_text = "这里是自动发送回复".to_string();
        let created_at = now_iso();

        let mut conversation = test_chat_conversation(&conversation_id, "active", &created_at);
        conversation.messages.push(ChatMessage {
            id: assistant_message_id.clone(),
            role: "assistant".to_string(),
            created_at: created_at.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: assistant_text.clone(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "remoteImDecision": {
                    "action": "send_async",
                    "processingMode": "continuous",
                    "conversationKind": "standard_conversation",
                    "activationSourceCount": 1,
                    "error": ""
                }
            })),
            tool_call: None,
            mcp_call: None,
        });

        let mut data = AppData::default();
        data.conversations.push(conversation);
        data.remote_im_contacts.push(RemoteImContact {
            id: "contact-record-a".to_string(),
            channel_id: "remote-im-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "private".to_string(),
            remote_contact_id: "contact-a".to_string(),
            remote_contact_name: "张三".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "always".to_string(),
            activation_keywords: Vec::new(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: None,
            bound_conversation_id: Some(conversation_id.clone()),
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        });
        state_write_app_data_cached(&state, &data).expect("write app data");

        (
            state,
            RemoteImActivationSource {
                channel_id: "remote-im-a".to_string(),
                platform: RemoteImPlatform::OnebotV11,
                remote_contact_type: "private".to_string(),
                remote_contact_id: "contact-a".to_string(),
                remote_contact_name: "张三".to_string(),
            },
            conversation_id,
            assistant_message_id,
            assistant_text,
        )
    }

    fn read_remote_im_decision_for_message(
        state: &AppState,
        conversation_id: &str,
        assistant_message_id: &str,
    ) -> Value {
        let data = state_read_app_data_cached(state).expect("read app data");
        data.conversations
            .iter()
            .find(|conversation| conversation.id == conversation_id)
            .and_then(|conversation| {
                conversation
                    .messages
                    .iter()
                    .find(|message| message.id == assistant_message_id)
            })
            .and_then(|message| message.provider_meta.as_ref())
            .and_then(|meta| meta.get("remoteImDecision"))
            .cloned()
            .expect("remoteImDecision")
    }

    #[test]
    fn remote_im_auto_send_and_record_decision_should_update_message_after_mock_send() {
        let (state, activation_source, conversation_id, assistant_message_id, assistant_text) =
            seed_remote_im_auto_send_test_state(serde_json::json!({
                "mockSend": true
            }));
        let assistant_message = state_read_app_data_cached(&state)
            .expect("read app data")
            .conversations
            .iter()
            .find(|conversation| conversation.id == conversation_id)
            .and_then(|conversation| {
                conversation
                    .messages
                    .iter()
                    .find(|message| message.id == assistant_message_id)
            })
            .cloned()
            .expect("assistant message");

        let outcome = test_runtime()
            .block_on(remote_im_auto_send_and_record_decision(
                &state,
                &activation_source,
                &conversation_id,
                &assistant_text,
                Some(&assistant_message),
                Some(&assistant_message_id),
            ))
            .expect("auto send should succeed");

        assert_eq!(
            outcome,
            RemoteImAutoSendExecutionOutcome::Sent {
                action: "reply_async".to_string()
            }
        );

        let decision =
            read_remote_im_decision_for_message(&state, &conversation_id, &assistant_message_id);
        assert_eq!(
            decision.get("action").and_then(Value::as_str),
            Some("reply_async")
        );
        assert_eq!(
            decision.get("processingMode").and_then(Value::as_str),
            Some("continuous")
        );
        assert_eq!(
            decision.get("conversationKind").and_then(Value::as_str),
            Some("standard_conversation")
        );
        assert_eq!(
            decision.get("activationSourceCount").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(decision.get("error").and_then(Value::as_str), Some(""));
    }

    #[test]
    fn remote_im_auto_send_and_record_decision_should_mark_send_failed_after_mock_error() {
        let (state, activation_source, conversation_id, assistant_message_id, assistant_text) =
            seed_remote_im_auto_send_test_state(serde_json::json!({
                "mockSendError": "mock remote send failed"
            }));
        let assistant_message = state_read_app_data_cached(&state)
            .expect("read app data")
            .conversations
            .iter()
            .find(|conversation| conversation.id == conversation_id)
            .and_then(|conversation| {
                conversation
                    .messages
                    .iter()
                    .find(|message| message.id == assistant_message_id)
            })
            .cloned()
            .expect("assistant message");

        let err = test_runtime()
            .block_on(remote_im_auto_send_and_record_decision(
                &state,
                &activation_source,
                &conversation_id,
                &assistant_text,
                Some(&assistant_message),
                Some(&assistant_message_id),
            ))
            .expect_err("auto send should fail");

        assert!(err.contains("mock remote send failed"));

        let decision =
            read_remote_im_decision_for_message(&state, &conversation_id, &assistant_message_id);
        assert_eq!(
            decision.get("action").and_then(Value::as_str),
            Some("send_failed")
        );
        assert_eq!(
            decision.get("processingMode").and_then(Value::as_str),
            Some("continuous")
        );
        assert_eq!(
            decision.get("conversationKind").and_then(Value::as_str),
            Some("standard_conversation")
        );
        assert_eq!(
            decision.get("activationSourceCount").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            decision.get("error").and_then(Value::as_str),
            Some("mock remote send failed")
        );
    }

    #[test]
    fn archive_decision_should_force_when_usage_reaches_82pct() {
        let now = now_iso();
        let d = decide_archive_before_model_request(820, 1000, Some(&now), true);
        assert!(d.should_archive);
        assert!(d.forced);
        assert!(d.usage_ratio >= 0.82);
    }

    #[test]
    fn archive_decision_should_archive_after_30m_and_30pct() {
        let now = now_utc();
        let old = (now - time::Duration::minutes(31))
            .format(&Rfc3339)
            .expect("format old time");
        let d = decide_archive_before_model_request(300, 1000, Some(&old), true);
        assert!(d.should_archive);
        assert!(!d.forced);
        assert!(d.usage_ratio >= 0.30);
    }

    #[test]
    fn archive_decision_should_not_archive_when_usage_below_30pct() {
        let now = now_utc();
        let old = (now - time::Duration::minutes(31))
            .format(&Rfc3339)
            .expect("format old time");
        let d = decide_archive_before_model_request(299, 1000, Some(&old), true);
        assert!(!d.should_archive);
        assert!(!d.forced);
        assert!(d.usage_ratio < 0.30);
    }

    #[test]
    fn archive_decision_should_use_prepared_prompt_usage_before_model_request() {
        let now = now_iso();
        let d = decide_archive_before_model_request(166_000, 200_000, Some(&now), true);
        assert!(d.should_archive);
        assert!(d.forced);
        assert!(d.usage_ratio >= 0.82);
    }

    #[test]
    fn archive_decision_should_prefer_cached_effective_prompt_tokens() {
        let now = now_iso();
        let (decision, source) =
            decide_archive_before_send_with_fallback(820, 0.10, Some(100), 1000, Some(&now), true);
        assert_eq!(source, "cached_effective_prompt_tokens");
        assert!(decision.should_archive);
        assert!(decision.forced);
        assert!(decision.usage_ratio >= 0.82);
    }

    #[test]
    fn archive_decision_should_fallback_to_estimate_only_when_cache_missing() {
        let now = now_iso();
        let (decision, source) =
            decide_archive_before_send_with_fallback(0, 0.0, Some(820), 1000, Some(&now), true);
        assert_eq!(source, "estimated_prompt_tokens");
        assert!(!decision.should_archive);
        assert!(!decision.forced);
        assert!(decision.usage_ratio >= 0.82);
    }

    #[test]
    fn archive_decision_should_force_only_at_95pct_when_estimate_is_used() {
        let now = now_iso();
        let (decision, source) =
            decide_archive_before_send_with_fallback(0, 0.0, Some(950), 1000, Some(&now), true);
        assert_eq!(source, "estimated_prompt_tokens");
        assert!(decision.should_archive);
        assert!(decision.forced);
        assert_eq!(decision.reason, "force_estimated_context_usage_95");
    }

    #[test]
    fn latest_real_prompt_usage_should_prefer_latest_assistant_message_provider_meta() {
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-main", "active", &now);
        conversation.messages.push(ChatMessage {
            id: "assistant-1".to_string(),
            role: "assistant".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "这是最近一条助手消息".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "effectivePromptTokens": 640,
                "contextUsageRatio": 0.64,
                "contextUsagePercent": 64
            })),
            tool_call: None,
            mcp_call: None,
        });
        let agent = AgentProfile {
            id: DEFAULT_AGENT_ID.to_string(),
            name: "默认助手".to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "global".to_string(),
            scope: "global".to_string(),
        };
        let usage = conversation_prompt_service()
            .latest_real_prompt_usage(&conversation, &ApiConfig::default())
            .expect("latest real prompt usage");

        assert_eq!(usage.source, "assistant_message_effective_prompt_tokens");
        assert_eq!(usage.effective_prompt_tokens, 640);
        assert!((usage.usage_ratio - 0.64).abs() < f64::EPSILON);
        assert!(usage.estimated_prompt_tokens.is_none());

        let prepared = PreparedPrompt {
            preamble: String::new(),
            history_messages: Vec::new(),
            latest_user_text: String::new(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };
        let resolved = conversation_prompt_service().resolve_prompt_usage(
            &prepared,
            &ApiConfig::default(),
            &agent,
            &conversation,
        );
        assert_eq!(resolved, usage);
    }

    #[test]
    fn latest_real_prompt_usage_should_read_compatible_prompt_usage_fields() {
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-main", "active", &now);
        conversation.messages.push(ChatMessage {
            id: "assistant-1".to_string(),
            role: "assistant".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "兼容字段".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "providerPromptTokens": 250,
                "contextUsagePercent": 30
            })),
            tool_call: None,
            mcp_call: None,
        });
        let mut api = ApiConfig::default();
        api.context_window_tokens = 1000;

        let usage = conversation_prompt_service()
            .latest_real_prompt_usage(&conversation, &api)
            .expect("latest real prompt usage");

        assert_eq!(usage.source, "assistant_message_provider_prompt_tokens");
        assert_eq!(usage.effective_prompt_tokens, 250);
        assert!((usage.usage_ratio - 0.3).abs() < f64::EPSILON);
        assert!(usage.estimated_prompt_tokens.is_none());
    }

    #[test]
    fn latest_real_prompt_usage_should_not_cross_context_compaction_boundary() {
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-main", "active", &now);
        conversation.messages.push(ChatMessage {
            id: "assistant-before-compaction".to_string(),
            role: "assistant".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "压缩前的一条助手消息".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "effectivePromptTokens": 640,
                "contextUsageRatio": 0.64,
                "contextUsagePercent": 64
            })),
            tool_call: None,
            mcp_call: None,
        });
        conversation.messages.push(ChatMessage {
            id: "compaction-boundary".to_string(),
            role: "user".to_string(),
            created_at: now.clone(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Text {
                text: "上下文整理".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "message_meta": {
                    "kind": "context_compaction"
                }
            })),
            tool_call: None,
            mcp_call: None,
        });
        conversation.messages.push(ChatMessage {
            id: "user-after-compaction".to_string(),
            role: "user".to_string(),
            created_at: now.clone(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Text {
                text: "压缩后的用户消息".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        });

        let usage =
            conversation_prompt_service().latest_real_prompt_usage(&conversation, &ApiConfig::default());
        assert!(usage.is_none());
    }

    #[test]
    fn resolve_prompt_usage_should_ignore_conversation_last_fields_without_assistant_meta() {
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-main", "active", &now);
        conversation.messages.push(ChatMessage {
            id: "assistant-1".to_string(),
            role: "assistant".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "这是最近一条没有 provider meta 的助手消息".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        });
        let agent = AgentProfile {
            id: DEFAULT_AGENT_ID.to_string(),
            name: "默认助手".to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "global".to_string(),
            scope: "global".to_string(),
        };
        let prepared = PreparedPrompt {
            preamble: "系统提示词".to_string(),
            history_messages: Vec::new(),
            latest_user_text: "用户消息".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let usage = conversation_prompt_service().resolve_prompt_usage(
            &prepared,
            &ApiConfig::default(),
            &agent,
            &conversation,
        );

        assert_eq!(usage.source, "estimated_prompt_tokens");
        assert!(usage.estimated_prompt_tokens.is_some());
        assert!(usage.effective_prompt_tokens > 0);
        assert!(usage.usage_ratio > 0.0);
    }

    #[test]
    fn runtime_trusted_prompt_usage_should_be_reused_during_dispatch() {
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-main", "active", &now);
        conversation.messages.push(ChatMessage {
            id: "assistant-1".to_string(),
            role: "assistant".to_string(),
            created_at: now.clone(),
            speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
            parts: vec![MessagePart::Text {
                text: "最近一次真实返回".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "effectivePromptTokens": 640,
                "contextUsageRatio": 0.64,
                "contextUsagePercent": 64
            })),
            tool_call: None,
            mcp_call: None,
        });
        let agent = AgentProfile {
            id: DEFAULT_AGENT_ID.to_string(),
            name: "默认助手".to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "global".to_string(),
            scope: "global".to_string(),
        };
        let prepared = PreparedPrompt {
            preamble: "系统提示词".to_string(),
            history_messages: Vec::new(),
            latest_user_text: "用户消息".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };
        let mut runtime_context = RuntimeContext::default();
        let primed = conversation_prompt_service().prime_runtime_trusted_prompt_usage(
            &mut runtime_context,
            &conversation,
            &prepared,
            &ApiConfig::default(),
            &agent,
        );
        assert_eq!(primed.source, "assistant_message_effective_prompt_tokens");

        assert!(runtime_context.trusted_prompt_usage.is_some());

        let reused = conversation_prompt_service().prime_runtime_trusted_prompt_usage(
            &mut runtime_context,
            &conversation,
            &prepared,
            &ApiConfig::default(),
            &agent,
        );
        assert_eq!(reused.source, "trusted_prompt_usage");
        assert_eq!(reused.effective_prompt_tokens, 640);
        assert!(reused.estimated_prompt_tokens.is_none());
    }

    #[test]
    fn shared_trusted_prompt_usage_should_refresh_after_provider_response() {
        let now = now_iso();
        let agent = AgentProfile {
            id: DEFAULT_AGENT_ID.to_string(),
            name: "默认助手".to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "global".to_string(),
            scope: "global".to_string(),
        };
        let prepared = PreparedPrompt {
            preamble: "系统提示词".to_string(),
            history_messages: Vec::new(),
            latest_user_text: "用户消息".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };
        let trusted = std::sync::Mutex::new(None::<TrustedPromptUsage>);

        conversation_prompt_service().refresh_shared_trusted_prompt_usage(
            &trusted,
            Some(640),
            &ApiConfig::default(),
        );
        let first = conversation_prompt_service().resolve_shared_trusted_prompt_usage_or_estimate(
            &trusted,
            &prepared,
            &ApiConfig::default(),
            &agent,
        );
        assert_eq!(first.source, "trusted_prompt_usage");
        assert_eq!(first.effective_prompt_tokens, 640);
        assert!(first.estimated_prompt_tokens.is_none());

        let second = conversation_prompt_service().resolve_shared_trusted_prompt_usage_or_estimate(
            &trusted,
            &prepared,
            &ApiConfig::default(),
            &agent,
        );
        assert_eq!(second.source, "trusted_prompt_usage");
        assert_eq!(second.effective_prompt_tokens, 640);
        assert!(second.estimated_prompt_tokens.is_none());

        conversation_prompt_service().refresh_shared_trusted_prompt_usage(
            &trusted,
            None,
            &ApiConfig::default(),
        );
        let after_missing_provider_usage = conversation_prompt_service()
            .resolve_shared_trusted_prompt_usage_or_estimate(
                &trusted,
                &prepared,
                &ApiConfig::default(),
                &agent,
            );
        assert_eq!(after_missing_provider_usage.source, "trusted_prompt_usage");
        assert_eq!(after_missing_provider_usage.effective_prompt_tokens, 640);
    }

    #[test]
    fn decide_archive_before_send_from_trusted_usage_should_use_real_threshold_branch() {
        let usage = PromptUsageResolution {
            effective_prompt_tokens: 240_845,
            usage_ratio: 0.8854595588235294,
            estimated_prompt_tokens: None,
            source: "trusted_prompt_usage",
        };

        let (decision, source) = decide_archive_before_send_from_usage(
            &usage,
            Some(&now_iso()),
            true,
        );

        assert_eq!(source, "trusted_prompt_usage");
        assert!(decision.should_archive);
        assert!(decision.forced);
        assert_eq!(decision.reason, "force_context_usage_82");
    }

    fn test_chat_runtime_state() -> AppState {
        let root = std::env::temp_dir().join(format!("eca-chat-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp test root");
        std::fs::create_dir_all(root.join("llm-workspace")).expect("create temp llm workspace");
        AppState {
            app_handle: Arc::new(Mutex::new(None)),
            config_path: root.join("app_config.toml"),
            data_path: root.join("app_data.json"),
            llm_workspace_path: root.join("llm-workspace"),
            shared_http_client: reqwest::Client::new(),
            terminal_shell: detect_default_terminal_shell(),
            terminal_shell_candidates: detect_terminal_shell_candidates(),
            conversation_lock: Arc::new(ConversationDomainLock::new()),
            memory_lock: Arc::new(Mutex::new(())),
            cached_config: Arc::new(Mutex::new(None)),
            cached_config_mtime: Arc::new(Mutex::new(None)),
            cached_agents: Arc::new(Mutex::new(None)),
            cached_agents_mtime: Arc::new(Mutex::new(None)),
            cached_runtime_state: Arc::new(Mutex::new(None)),
            cached_runtime_state_mtime: Arc::new(Mutex::new(None)),
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_conversations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_metadata: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_pending: Arc::new(Mutex::new(None)),
            app_data_persist_notify: Arc::new(tokio::sync::Notify::new()),
            app_data_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            conversation_persist_pending: Arc::new(Mutex::new(None)),
            conversation_persist_notify: Arc::new(tokio::sync::Notify::new()),
            conversation_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conversation_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_conversation_dirty_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_deleted_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            app_data_persist_write_lock: Arc::new(Mutex::new(())),
            last_panic_snapshot: Arc::new(Mutex::new(None)),
            inflight_chat_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_tool_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_completed_tool_history: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_session_roots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new("github".to_string())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn test_pending_event(conversation_id: &str) -> ChatPendingEvent {
        let created_at = now_iso();
        ChatPendingEvent {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            created_at: created_at.clone(),
            source: ChatEventSource::User,
            queue_mode: ChatQueueMode::Normal,
            messages: vec![test_text_message("user", "hello", &created_at)],
            activate_assistant: true,
            session_info: ChatSessionInfo {
                department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
                agent_id: DEFAULT_AGENT_ID.to_string(),
            },
            runtime_context: None,
            sender_info: None,
        }
    }

    fn test_chat_conversation(conversation_id: &str, status: &str, updated_at: &str) -> Conversation {
        Conversation {
            id: conversation_id.to_string(),
            title: conversation_id.to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
            last_user_at: None,
            last_assistant_at: None,
            status: status.to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: Vec::new(),
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }

    fn test_user_switched_to_sub_conversation_data() -> AppData {
        let now = now_iso();
        let later = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format later");
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![
            test_chat_conversation("conversation-main", "inactive", &now),
            test_chat_conversation("conversation-sub", "active", &later),
        ];
        data
    }

    fn total_queue_len(state: &AppState) -> Result<usize, String> {
        let slots = state
            .conversation_runtime_slots
            .lock()
            .map_err(|err| format!("lock conversation_runtime_slots failed: {err}"))?;
        Ok(slots.values().map(|slot| slot.pending_queue.len()).sum())
    }

    #[test]
    fn state_read_chat_index_cached_should_rebuild_from_storage_without_disk_index() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-heal", "active", &now);

        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        let item = chat_index
            .conversations
            .iter()
            .find(|item| item.id == conversation.id)
            .expect("healed chat index item");
        assert_eq!(item.updated_at, conversation.updated_at);
        assert_eq!(item.status, conversation.status);
        assert_eq!(item.summary, conversation.summary);
        assert_eq!(item.archived_at, conversation.archived_at);
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn state_read_chat_index_cached_should_rebuild_archived_fields_from_storage() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-hidden-heal", "active", &now);
        conversation.summary = "测试摘要".to_string();
        conversation.archived_at = Some(now.clone());
        conversation.status = "archived".to_string();

        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        let item = chat_index
            .conversations
            .iter()
            .find(|item| item.id == conversation.id)
            .expect("archived chat index item");
        assert_eq!(item.summary, conversation.summary);
        assert_eq!(item.archived_at, conversation.archived_at);
        assert_eq!(item.status, conversation.status);
    }

    #[test]
    fn state_read_chat_index_cached_should_recover_archived_items_from_storage_snapshot() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-archived-recover", "active", &now);
        conversation.summary = "恢复用归档摘要".to_string();
        conversation.archived_at = Some(now.clone());
        conversation.status = "archived".to_string();

        write_conversation_shard(&state.data_path, &conversation).expect("write archived conversation");

        let chat_index = state_read_chat_index_cached(&state).expect("read rebuilt chat index");
        let item = chat_index
            .conversations
            .iter()
            .find(|item| item.id == conversation.id)
            .expect("recovered archived item");
        assert_eq!(item.summary, conversation.summary);
        assert_eq!(item.archived_at, conversation.archived_at);
        assert_eq!(item.status, conversation.status);
    }

    #[test]
    fn read_app_bootstrap_snapshot_should_build_memory_chat_index_from_storage_snapshot() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-bootstrap-recover", "active", &now);
        conversation.summary = "启动恢复归档摘要".to_string();
        conversation.archived_at = Some(now.clone());
        conversation.status = "archived".to_string();

        write_conversation_shard(&state.data_path, &conversation).expect("write archived conversation");

        let _snapshot = read_app_bootstrap_snapshot(&state).expect("read bootstrap snapshot");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert_eq!(chat_index.conversations.len(), 1);
        assert_eq!(chat_index.conversations[0].id, conversation.id);
        assert_eq!(chat_index.conversations[0].status, conversation.status);
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn state_read_chat_index_cached_should_rebuild_from_legacy_app_data_file() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-legacy-index", "active", &now);
        let mut data = AppData::default();
        data.conversations = vec![conversation.clone()];
        std::fs::write(
            &state.data_path,
            serde_json::to_vec_pretty(&data).expect("serialize legacy app data"),
        )
        .expect("write legacy app data");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");

        assert_eq!(chat_index.conversations.len(), 1);
        assert_eq!(chat_index.conversations[0].id, conversation.id);
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn state_schedule_conversation_persist_should_update_memory_chat_index_only() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-memory-upsert", "active", &now);
        state_schedule_conversation_persist(&state, &conversation).expect("schedule persist");
        conversation.summary = "updated summary".to_string();
        conversation.status = "archived".to_string();
        conversation.archived_at = Some(now.clone());
        state_schedule_conversation_persist(&state, &conversation).expect("schedule updated persist");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        let item = chat_index
            .conversations
            .iter()
            .find(|item| item.id == conversation.id)
            .expect("chat index item");
        assert_eq!(item.summary, conversation.summary);
        assert_eq!(item.status, conversation.status);
        assert_eq!(item.archived_at, conversation.archived_at);
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn set_conversation_preferred_model_should_schedule_meta_only_persist() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-meta-only-model", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        state_mark_conversation_direct_persisted(&state, &conversation)
            .expect("mark persisted");

        let updated = conversation_service()
            .set_conversation_preferred_api_config_id(
                &state,
                &conversation.id,
                Some("api-model-b".to_string()),
            )
            .expect("set preferred model");

        assert_eq!(
            updated.preferred_api_config_id.as_deref(),
            Some("api-model-b")
        );
        let pending = state
            .conversation_persist_pending
            .lock()
            .expect("lock pending");
        let pending = pending.as_ref().expect("pending meta persist");
        assert!(pending.conversations.is_empty());
        assert!(pending.metadata_conversation_ids.contains(&conversation.id));
        assert!(!pending.deleted_conversation_ids.contains(&conversation.id));
    }

    #[test]
    fn set_conversation_preferred_model_should_update_existing_full_pending_snapshot() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-pending-model", "active", &now);
        state_schedule_conversation_persist(&state, &conversation).expect("schedule full persist");

        conversation_service()
            .set_conversation_preferred_api_config_id(
                &state,
                &conversation.id,
                Some("api-model-c".to_string()),
            )
            .expect("set preferred model");

        let pending = state
            .conversation_persist_pending
            .lock()
            .expect("lock pending");
        let pending = pending.as_ref().expect("pending full persist");
        assert!(pending.metadata_conversation_ids.is_empty());
        assert_eq!(
            pending
                .conversations
                .get(&conversation.id)
                .and_then(|item| item.preferred_api_config_id.as_deref()),
            Some("api-model-c")
        );
    }

    #[test]
    fn flush_pending_persists_should_write_meta_only_preferred_model() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-flush-meta-model", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        state_mark_conversation_direct_persisted(&state, &conversation)
            .expect("mark persisted");
        conversation_service()
            .set_conversation_preferred_api_config_id(
                &state,
                &conversation.id,
                Some("api-model-flush".to_string()),
            )
            .expect("set preferred model");

        let wrote = flush_pending_persists_blocking(&state).expect("flush pending");
        let restored = read_conversation_shard(&state.data_path, &conversation.id)
            .expect("read restored conversation");

        assert!(wrote);
        assert_eq!(
            restored.preferred_api_config_id.as_deref(),
            Some("api-model-flush")
        );
        assert!(restored.messages.is_empty());
    }

    #[test]
    fn foreground_snapshot_should_include_cached_preferred_model() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-snapshot-model", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        state_mark_conversation_direct_persisted(&state, &conversation)
            .expect("mark persisted");

        conversation_service()
            .set_conversation_preferred_api_config_id(
                &state,
                &conversation.id,
                Some("api-model-snapshot".to_string()),
            )
            .expect("set preferred model");

        let snapshot = conversation_service()
            .read_foreground_snapshot(&state, Some(&conversation.id), None, 4)
            .expect("read foreground snapshot");
        let summaries = conversation_service()
            .list_unarchived_conversation_summaries(&state)
            .expect("list summaries")
            .summaries;
        let summaries_json = serde_json::to_string(&summaries).expect("serialize summaries");

        assert_eq!(
            snapshot.preferred_api_config_id.as_deref(),
            Some("api-model-snapshot")
        );
        assert!(
            !summaries_json.contains("preferredApiConfigId"),
            "conversation overview must not carry model metadata"
        );
    }

    #[test]
    fn state_schedule_conversation_persist_should_preserve_field_level_metadata() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-preserve-model", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        state_mark_conversation_direct_persisted(&state, &conversation)
            .expect("mark persisted");
        conversation_service()
            .set_conversation_preferred_api_config_id(
                &state,
                &conversation.id,
                Some("api-model-d".to_string()),
            )
            .expect("set preferred model");
        conversation_service()
            .set_conversation_title_metadata(&state, &conversation.id, "字段级标题")
            .expect("set title");
        conversation_service()
            .set_conversation_shell_workspace_metadata(
                &state,
                &conversation.id,
                Some(Some(state.llm_workspace_path.to_string_lossy().to_string())),
                None,
                Some(true),
            )
            .expect("set workspace metadata");
        conversation_service()
            .set_conversation_lifecycle_metadata(
                &state,
                &conversation.id,
                Some("archived"),
                Some(""),
                Some(Some(now.clone())),
                Some(now.clone()),
            )
            .expect("set lifecycle metadata");
        conversation_service()
            .set_conversation_current_todos_metadata(
                &state,
                &conversation.id,
                vec![ConversationTodoItem {
                    content: "字段级 todo".to_string(),
                    status: "in_progress".to_string(),
                }],
            )
            .expect("set todos metadata");
        state_update_conversation_metadata_cached(
            &state,
            &conversation.id,
            |conversation| {
                conversation.agent_id = "字段级agent".to_string();
                conversation.department_id = "字段级department".to_string();
                conversation.root_conversation_id = Some("字段级root".to_string());
                conversation.conversation_kind = CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string();
                conversation.user_profile_snapshot = "字段级画像".to_string();
                conversation.memory_recall_table = vec!["memory-a".to_string()];
                Ok(())
            },
        )
        .expect("set runtime metadata");

        let mut stale_full_snapshot = conversation.clone();
        stale_full_snapshot.title = "过期标题".to_string();
        stale_full_snapshot.shell_workspace_path = None;
        stale_full_snapshot.shell_autonomous_mode = false;
        stale_full_snapshot.status = "active".to_string();
        stale_full_snapshot.archived_at = None;
        stale_full_snapshot.agent_id = DEFAULT_AGENT_ID.to_string();
        stale_full_snapshot.department_id = ASSISTANT_DEPARTMENT_ID.to_string();
        stale_full_snapshot.root_conversation_id = None;
        stale_full_snapshot.conversation_kind = CONVERSATION_KIND_CHAT.to_string();
        stale_full_snapshot.current_todos = Vec::new();
        stale_full_snapshot.user_profile_snapshot.clear();
        stale_full_snapshot.memory_recall_table.clear();
        stale_full_snapshot.messages.push(test_text_message("user", "hello", &now));
        stale_full_snapshot.updated_at = "2020-01-01T00:00:00Z".to_string();
        stale_full_snapshot.last_user_at = Some("2020-01-01T00:00:00Z".to_string());
        state_schedule_conversation_persist(&state, &stale_full_snapshot)
            .expect("schedule stale full persist");

        let cached = state_read_conversation_cached(&state, &conversation.id)
            .expect("read cached conversation");
        assert_eq!(
            cached.preferred_api_config_id.as_deref(),
            Some("api-model-d")
        );
        assert_eq!(cached.title, "字段级标题");
        assert_eq!(
            cached.shell_workspace_path.as_deref(),
            Some(state.llm_workspace_path.to_string_lossy().as_ref())
        );
        assert!(cached.shell_autonomous_mode);
        assert_eq!(cached.status, "archived");
        assert_eq!(cached.archived_at.as_deref(), Some(now.as_str()));
        assert_eq!(cached.current_todos.len(), 1);
        assert_eq!(cached.current_todos[0].content, "字段级 todo");
        assert_eq!(cached.user_profile_snapshot, "字段级画像");
        assert_eq!(cached.memory_recall_table, vec!["memory-a".to_string()]);
        assert_eq!(cached.agent_id, "字段级agent");
        assert_eq!(cached.department_id, "字段级department");
        assert_eq!(cached.root_conversation_id.as_deref(), Some("字段级root"));
        assert_eq!(
            cached.conversation_kind,
            CONVERSATION_KIND_REMOTE_IM_CONTACT
        );
        assert_eq!(cached.updated_at, now);
        assert_ne!(
            cached.last_user_at.as_deref(),
            Some("2020-01-01T00:00:00Z")
        );
        assert_eq!(cached.messages.len(), 1);
    }

    #[test]
    fn state_schedule_conversation_persist_should_not_decrease_cumulative_usage() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-usage-add-only", "active", &now);
        state_schedule_conversation_persist(&state, &conversation).expect("schedule persist");
        conversation_service()
            .add_conversation_cumulative_usage_delta(
                &state,
                &conversation.id,
                &serde_json::json!({
                    "completionTokens": 20,
                    "cachedTokens": 100,
                    "cacheCreationTokens": 5
                }),
            )
            .expect("add cumulative usage");

        let mut stale_full_snapshot = conversation.clone();
        stale_full_snapshot
            .messages
            .push(test_text_message("user", "hello", &now));
        state_schedule_conversation_persist(&state, &stale_full_snapshot)
            .expect("schedule stale full persist");

        let cached = state_read_conversation_cached(&state, &conversation.id)
            .expect("read cached conversation");
        assert_eq!(cached.cumulative_usage.output_tokens, 20);
        assert_eq!(cached.cumulative_usage.cache_read_tokens, 100);
        assert_eq!(cached.cumulative_usage.cache_write_tokens, 5);
        assert_eq!(
            conversation_cumulative_usage_weighted_tokens(&cached.cumulative_usage),
            47
        );
        assert_eq!(cached.messages.len(), 1);
    }

    #[test]
    fn state_mark_conversation_direct_persisted_should_update_memory_chat_index_only() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-direct-upsert", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        conversation.summary = "direct summary".to_string();
        conversation.status = "archived".to_string();
        conversation.archived_at = Some(now.clone());
        state_mark_conversation_direct_persisted(&state, &conversation)
            .expect("mark direct persisted");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert_eq!(chat_index.conversations.len(), 1);
        assert_eq!(chat_index.conversations[0].id, conversation.id);
        assert_eq!(chat_index.conversations[0].summary, conversation.summary);
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn state_schedule_conversation_delete_should_remove_memory_chat_index_item() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-memory-delete", "active", &now);
        state_schedule_conversation_persist(&state, &conversation).expect("schedule persist");

        state_schedule_conversation_delete(&state, &conversation.id).expect("schedule delete");

        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert!(chat_index.conversations.is_empty());
        assert!(!app_layout_chat_index_path(&state.data_path).exists());
    }

    #[test]
    fn collect_unarchived_conversation_summaries_cached_should_remove_orphan_index_items() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-existing", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");

        let summaries = conversation_service()
            .collect_unarchived_conversation_summaries_cached(&state, &AppConfig::default())
            .expect("collect summaries");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].conversation_id, conversation.id);
        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert_eq!(chat_index.conversations.len(), 1);
        assert_eq!(chat_index.conversations[0].id, conversation.id);
    }

    #[test]
    fn collect_unarchived_conversation_summaries_cached_should_repair_visible_summary_mismatch() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-visible-summary-mismatch", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");

        let summaries = conversation_service()
            .collect_unarchived_conversation_summaries_cached(&state, &AppConfig::default())
            .expect("collect summaries");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].conversation_id, conversation.id);
        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert_eq!(chat_index.conversations.len(), 1);
        assert_eq!(chat_index.conversations[0].id, conversation.id);
        assert_eq!(chat_index.conversations[0].summary, conversation.summary);
    }

    #[test]
    fn collect_unarchived_conversation_summaries_cached_should_keep_archived_index_items() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let active = test_chat_conversation("conversation-active", "active", &now);
        let mut archived = test_chat_conversation("conversation-archived", "active", &now);
        archived.summary = "归档摘要".to_string();
        archived.archived_at = Some(now.clone());
        archived.status = "archived".to_string();

        write_conversation_shard(&state.data_path, &active).expect("write active conversation");
        write_conversation_shard(&state.data_path, &archived).expect("write archived conversation");

        let summaries = conversation_service()
            .collect_unarchived_conversation_summaries_cached(&state, &AppConfig::default())
            .expect("collect summaries");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].conversation_id, active.id);
        let chat_index = state_read_chat_index_cached(&state).expect("read memory chat index");
        assert_eq!(chat_index.conversations.len(), 2);
        assert!(chat_index
            .conversations
            .iter()
            .any(|item| item.id == archived.id));
    }

    #[test]
    fn instant_archive_conversation_should_remove_from_unarchived_and_keep_messages() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let active = test_chat_conversation("conversation-active-next", "active", &now);
        let mut source = test_chat_conversation("conversation-instant-archive", "active", &now);
        source.messages = vec![
            test_text_message("user", "第一轮问题", &now),
            test_text_message("assistant", "第一轮回复", &now),
            test_text_message("user", "第二轮问题", &now),
            test_text_message("assistant", "第二轮回复", &now),
        ];
        write_conversation_shard(&state.data_path, &active).expect("write active conversation");
        write_conversation_shard(&state.data_path, &source).expect("write source conversation");

        let result = conversation_service()
            .instant_archive_conversation(&state, &ApiConfig::default(), &source, "test")
            .expect("instant archive");

        assert_eq!(result.active_conversation_id, active.id);
        assert!(!result.already_archived);
        assert_eq!(result.archived_conversation.status, "archived");
        assert!(result.archived_conversation.archived_at.is_some());
        assert!(result
            .overview_payload
            .unarchived_conversations
            .iter()
            .all(|item| item.conversation_id != source.id));

        let summaries = conversation_service()
            .list_unarchived_conversation_summaries(&state)
            .expect("list unarchived")
            .summaries;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].conversation_id, active.id);

        let archives = conversation_service().list_archives(&state).expect("list archives");
        assert!(archives.iter().any(|item| item.archive_id == source.id));
        let archive_messages = conversation_service()
            .read_archive_messages(&state, &source.id)
            .expect("read archive messages");
        assert_eq!(archive_messages.len(), source.messages.len());

        let second = conversation_service()
            .instant_archive_conversation(&state, &ApiConfig::default(), &source, "test")
            .expect("idempotent instant archive");
        assert!(second.already_archived);
        assert_eq!(second.archived_conversation.status, "archived");

        flush_pending_persists_blocking(&state).expect("flush pending persists");
        let restored = read_conversation_shard(&state.data_path, &source.id)
            .expect("read restored archived conversation");
        assert_eq!(restored.status, "archived");
        assert!(restored.archived_at.is_some());
        assert_eq!(restored.messages.len(), source.messages.len());
    }

    #[test]
    fn list_remote_im_contact_conversations_should_create_and_bind_missing_conversation() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let mut runtime = RuntimeStateFile::default();
        runtime.remote_im_contacts.push(RemoteImContact {
            id: "contact-a".to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "group".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "测试群".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "never".to_string(),
            activation_keywords: Vec::new(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
            bound_conversation_id: None,
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: Some(now),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        });
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");

        let items = conversation_service()
            .list_remote_im_contact_conversations(&state)
            .expect("list remote im contact conversations");

        assert_eq!(items.len(), 1);
        let updated_runtime = state_read_runtime_state_cached(&state).expect("read runtime state");
        let contact = updated_runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id == "contact-a")
            .expect("contact exists");
        let conversation_id = contact
            .bound_conversation_id
            .as_deref()
            .expect("bound conversation id");
        assert_eq!(items[0].conversation_id, conversation_id);
        let conversation =
            state_read_conversation_cached(&state, conversation_id).expect("read conversation");
        assert!(conversation_is_remote_im_contact(&conversation));
        assert_eq!(conversation.department_id, REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID);
        assert_eq!(conversation.agent_id, DEFAULT_AGENT_ID);
        let expected_key = remote_im_contact_conversation_key(contact);
        assert_eq!(conversation.root_conversation_id.as_deref(), Some(expected_key.as_str()));
    }

    #[test]
    fn list_remote_im_contact_conversations_should_reuse_existing_history_and_rebind_contact() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let contact = RemoteImContact {
            id: "contact-a".to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "group".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "测试群".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "never".to_string(),
            activation_keywords: Vec::new(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
            bound_conversation_id: None,
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: Some(now.clone()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        };
        let mut runtime = RuntimeStateFile::default();
        runtime.remote_im_contacts.push(contact.clone());
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");

        let mut conversation = build_conversation_record(
            "",
            "",
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID,
            "联系人 · 测试群",
            CONVERSATION_KIND_REMOTE_IM_CONTACT,
            Some(remote_im_contact_conversation_key(&contact)),
            None,
        );
        conversation.id = "conversation-contact-old".to_string();
        conversation.messages.push(test_text_message("user", "历史消息", &now));
        conversation.updated_at = now.clone();
        conversation.last_user_at = Some(now.clone());
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");

        let items = conversation_service()
            .list_remote_im_contact_conversations(&state)
            .expect("list remote im contact conversations");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].conversation_id, "conversation-contact-old");
        assert_eq!(items[0].message_count, 1);
        let updated_runtime = state_read_runtime_state_cached(&state).expect("read runtime state");
        let updated_contact = updated_runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id == "contact-a")
            .expect("contact exists");
        assert_eq!(
            updated_contact.bound_conversation_id.as_deref(),
            Some("conversation-contact-old")
        );
        let updated_conversation =
            state_read_conversation_cached(&state, "conversation-contact-old")
                .expect("read rebound conversation");
        assert_eq!(
            updated_conversation.department_id,
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID
        );
        assert_eq!(updated_conversation.agent_id, DEFAULT_AGENT_ID);
    }

    #[test]
    fn remote_im_contact_conversation_should_be_readable_and_writable_as_unarchived() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let contact = RemoteImContact {
            id: "contact-a".to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "group".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "测试群".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "never".to_string(),
            activation_keywords: Vec::new(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
            bound_conversation_id: Some("conversation-contact-old".to_string()),
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: Some(now.clone()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        };
        let mut runtime = RuntimeStateFile::default();
        runtime.remote_im_contacts.push(contact.clone());
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");

        let mut conversation = build_conversation_record(
            "",
            "",
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID,
            "联系人 · 测试群",
            CONVERSATION_KIND_REMOTE_IM_CONTACT,
            Some(remote_im_contact_conversation_key(&contact)),
            None,
        );
        conversation.id = "conversation-contact-old".to_string();
        conversation.messages.push(test_text_message("user", "历史消息", &now));
        conversation.updated_at = now.clone();
        conversation.last_user_at = Some(now.clone());
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");

        let messages = conversation_service()
            .read_unarchived_messages(&state, "conversation-contact-old")
            .expect("read remote im contact conversation as unarchived");
        assert_eq!(messages.len(), 1);
        match &messages[0].parts[0] {
            MessagePart::Text { text, .. } => assert_eq!(text, "历史消息"),
            _ => panic!("expected text message"),
        }

        conversation_service()
            .update_unarchived_conversation_by_id(
                &state,
                "conversation-contact-old",
                |conversation| {
                    conversation.title = "联系人 · 已改名".to_string();
                    Ok(())
                },
            )
            .expect("update remote im contact conversation as unarchived");
        let updated = state_read_conversation_cached(&state, "conversation-contact-old")
            .expect("read updated conversation");
        assert_eq!(updated.title, "联系人 · 已改名");
    }

    #[test]
    fn rewind_remote_im_contact_conversation_should_hydrate_messages_from_store() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let mut first_user = test_text_message("user", "第一句", &now);
        first_user.id = "user-1".to_string();
        let mut first_assistant = test_text_message("assistant", "第一句回复", &now);
        first_assistant.id = "assistant-1".to_string();
        let mut recalled_user = test_text_message("user", "需要撤回", &now);
        recalled_user.id = "user-2".to_string();
        let mut trailing_assistant = test_text_message("assistant", "后续回复", &now);
        trailing_assistant.id = "assistant-2".to_string();
        let mut conversation = build_conversation_record(
            "",
            DEFAULT_AGENT_ID,
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID,
            "联系人 · 测试群",
            CONVERSATION_KIND_REMOTE_IM_CONTACT,
            Some("remote_im_contact:channel-a:group:remote-a".to_string()),
            None,
        );
        conversation.id = "conversation-contact-rewind".to_string();
        conversation.status = "inactive".to_string();
        conversation.messages = vec![
            first_user,
            first_assistant,
            recalled_user.clone(),
            trailing_assistant,
        ];
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist full conversation");
        let store_paths =
            message_store::message_store_paths(&state.data_path, &conversation.id)
                .expect("message store paths");
        message_store::write_jsonl_snapshot_directory_shard(&store_paths, &conversation)
            .expect("write message store");
        conversation.messages = Vec::new();
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist slim conversation");

        let input = RewindConversationInput {
            session: SessionSelector {
                api_config_id: None,
                department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
                agent_id: DEFAULT_AGENT_ID.to_string(),
                conversation_id: Some(conversation.id.clone()),
            },
            message_id: recalled_user.id.clone(),
            undo_apply_patch: false,
        };
        let result = conversation_service()
            .rewind_conversation_from_message(
                &state,
                &input,
                DEFAULT_AGENT_ID,
                &recalled_user.id,
                &std::time::Instant::now(),
            )
            .expect("rewind remote im contact conversation");

        assert_eq!(result.removed_count, 2);
        assert_eq!(result.remaining_count, 2);
        assert_eq!(
            result
                .recalled_user_message
                .as_ref()
                .map(|message| message.id.as_str()),
            Some("user-2")
        );
        let stored = message_store::read_ready_message_store_all_messages(&store_paths)
            .expect("read truncated message store")
            .expect("message store exists");
        assert_eq!(
            stored
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["user-1", "assistant-1"]
        );
    }

    fn setup_rewind_busy_test_conversation(
        conversation_id: &str,
    ) -> (AppState, RewindConversationInput, String) {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_iso();
        let mut first_user = test_text_message("user", "第一句", &now);
        first_user.id = "user-1".to_string();
        let mut first_assistant = test_text_message("assistant", "第一句回复", &now);
        first_assistant.id = "assistant-1".to_string();
        let mut recalled_user = test_text_message("user", "需要撤回", &now);
        recalled_user.id = "user-2".to_string();
        let mut trailing_assistant = test_text_message("assistant", "后续回复", &now);
        trailing_assistant.id = "assistant-2".to_string();
        let mut conversation = build_conversation_record(
            "",
            DEFAULT_AGENT_ID,
            ASSISTANT_DEPARTMENT_ID,
            "撤回忙碌态测试",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        conversation.id = conversation_id.to_string();
        conversation.status = "active".to_string();
        conversation.messages = vec![
            first_user,
            first_assistant,
            recalled_user.clone(),
            trailing_assistant,
        ];
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");
        let store_paths =
            message_store::message_store_paths(&state.data_path, &conversation.id)
                .expect("message store paths");
        message_store::write_jsonl_snapshot_directory_shard(&store_paths, &conversation)
            .expect("write message store");
        let input = RewindConversationInput {
            session: SessionSelector {
                api_config_id: None,
                department_id: Some(ASSISTANT_DEPARTMENT_ID.to_string()),
                agent_id: DEFAULT_AGENT_ID.to_string(),
                conversation_id: Some(conversation.id.clone()),
            },
            message_id: recalled_user.id.clone(),
            undo_apply_patch: false,
        };
        (state, input, recalled_user.id)
    }

    #[test]
    fn rewind_conversation_should_fail_without_mutation_while_assistant_streaming() {
        let (state, input, recalled_user_id) =
            setup_rewind_busy_test_conversation("conversation-rewind-streaming");
        set_conversation_runtime_state(
            &state,
            "conversation-rewind-streaming",
            MainSessionState::AssistantStreaming,
        )
        .expect("set runtime state");

        let result = conversation_service().rewind_conversation_from_message(
            &state,
            &input,
            DEFAULT_AGENT_ID,
            &recalled_user_id,
            &std::time::Instant::now(),
        );

        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap_or_default()
                .contains("当前会话正在运行或整理上下文")
        );
        let cached = state_read_conversation_cached(&state, "conversation-rewind-streaming")
            .expect("read cached conversation");
        assert_eq!(cached.messages.len(), 4);
        let store_paths =
            message_store::message_store_paths(&state.data_path, "conversation-rewind-streaming")
                .expect("message store paths");
        let stored = message_store::read_ready_message_store_all_messages(&store_paths)
            .expect("read message store")
            .expect("message store exists");
        assert_eq!(stored.len(), 4);
    }

    #[test]
    fn rewind_conversation_should_fail_without_mutation_while_organizing_context() {
        let (state, input, recalled_user_id) =
            setup_rewind_busy_test_conversation("conversation-rewind-organizing");
        set_conversation_runtime_state(
            &state,
            "conversation-rewind-organizing",
            MainSessionState::OrganizingContext,
        )
        .expect("set runtime state");

        let result = conversation_service().rewind_conversation_from_message(
            &state,
            &input,
            DEFAULT_AGENT_ID,
            &recalled_user_id,
            &std::time::Instant::now(),
        );

        assert!(result.is_err());
        let cached = state_read_conversation_cached(&state, "conversation-rewind-organizing")
            .expect("read cached conversation");
        assert_eq!(cached.messages.len(), 4);
        let store_paths =
            message_store::message_store_paths(&state.data_path, "conversation-rewind-organizing")
                .expect("message store paths");
        let stored = message_store::read_ready_message_store_all_messages(&store_paths)
            .expect("read message store")
            .expect("message store exists");
        assert_eq!(stored.len(), 4);
    }

    #[test]
    fn scheduler_should_allow_two_conversations_to_run_in_parallel() {
        let state = test_chat_runtime_state();
        let ingress_a =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("ingress a");
        let ingress_b =
            ingress_chat_event(&state, test_pending_event("conversation-b")).expect("ingress b");

        assert!(matches!(ingress_a, ChatEventIngress::Direct(_)));
        assert!(matches!(ingress_b, ChatEventIngress::Direct(_)));
        assert_eq!(total_queue_len(&state).expect("queue len"), 0);

        let claims = state
            .conversation_processing_claims
            .lock()
            .expect("lock claims");
        assert!(claims.contains("conversation-a"));
        assert!(claims.contains("conversation-b"));
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn scheduler_should_keep_same_conversation_serial() {
        let state = test_chat_runtime_state();
        let ingress_a1 =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("ingress a1");
        let ingress_a2 =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("ingress a2");

        assert!(matches!(ingress_a1, ChatEventIngress::Direct(_)));
        assert!(matches!(ingress_a2, ChatEventIngress::Queued { .. }));
        assert_eq!(total_queue_len(&state).expect("queue len"), 1);
    }

    #[test]
    fn scheduler_should_ignore_duplicate_user_event_after_message_persisted() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut conversation = test_chat_conversation("conversation-a", "active", &now);
        let mut persisted = test_text_message("user", "hello", &now);
        persisted.provider_meta = build_user_message_provider_meta(
            None,
            &[],
            &[],
            Some("chat-same-request"),
        );
        conversation.messages.push(persisted);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::AssistantStreaming)
            .expect("set streaming state");

        let mut duplicate = test_pending_event("conversation-a");
        duplicate.runtime_context = Some(RuntimeContext {
            request_id: Some("chat-same-request".to_string()),
            ..RuntimeContext::default()
        });

        let ingress = ingress_chat_event(&state, duplicate).expect("ingress duplicate");

        assert!(matches!(ingress, ChatEventIngress::Duplicate { .. }));
        assert_eq!(total_queue_len(&state).expect("queue len"), 0);
    }

    #[test]
    fn queue_snapshot_should_keep_full_message_text_for_recall() {
        let state = test_chat_runtime_state();
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::AssistantStreaming)
            .expect("set streaming state");
        let long_text = "0123456789".repeat(8);
        let created_at = now_iso();
        let mut event = test_pending_event("conversation-a");
        event.messages = vec![test_text_message("user", &long_text, &created_at)];

        let ingress = ingress_chat_event(&state, event).expect("queue event");
        assert!(matches!(ingress, ChatEventIngress::Queued { .. }));

        let snapshot = get_queue_snapshot(&state).expect("queue snapshot");
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot[0].message_preview.ends_with("..."));
        assert_eq!(snapshot[0].message_text, long_text);
    }

    #[test]
    fn mark_queue_event_guided_should_force_activation_and_guided_dispatch_reason() {
        let state = test_chat_runtime_state();
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::AssistantStreaming)
            .expect("set streaming state");
        let mut event = test_pending_event("conversation-a");
        event.activate_assistant = false;
        event.runtime_context = Some(runtime_context_new("user_message", "user_send"));

        let ingress = ingress_chat_event(&state, event).expect("queue event");
        let event_id = match ingress {
            ChatEventIngress::Queued { event_id } => event_id,
            _ => panic!("expected queued guided candidate"),
        };

        let updated_conversation_id =
            mark_queue_event_guided(&state, &event_id).expect("mark guided");
        assert_eq!(updated_conversation_id.as_deref(), Some("conversation-a"));

        let slots = state
            .conversation_runtime_slots
            .lock()
            .expect("lock slots");
        let slot = slots.get("conversation-a").expect("conversation slot");
        let queued = slot
            .pending_queue
            .iter()
            .find(|item| item.id == event_id)
            .expect("guided event still queued");
        assert_eq!(queued.queue_mode, ChatQueueMode::Guided);
        assert!(queued.activate_assistant);
        assert_eq!(
            queued
                .runtime_context
                .as_ref()
                .and_then(|ctx| ctx.dispatch_reason.as_deref()),
            Some("guided_queue")
        );
    }

    #[test]
    fn claim_guided_queue_events_for_conversation_should_remove_guided_and_keep_normal_events() {
        let state = test_chat_runtime_state();
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::AssistantStreaming)
            .expect("set streaming state");

        let guided_ingress =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("queue guided");
        let guided_event_id = match guided_ingress {
            ChatEventIngress::Queued { event_id } => event_id,
            _ => panic!("expected first event queued"),
        };
        let normal_ingress =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("queue normal");
        let normal_event_id = match normal_ingress {
            ChatEventIngress::Queued { event_id } => event_id,
            _ => panic!("expected second event queued"),
        };

        mark_queue_event_guided(&state, &guided_event_id).expect("mark guided");
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::Idle)
            .expect("restore idle");

        let guided_events =
            claim_guided_queue_events_for_conversation(&state, "conversation-a").expect("claim guided");

        assert_eq!(guided_events.len(), 1);
        assert_eq!(guided_events[0].id, guided_event_id);
        assert_eq!(guided_events[0].queue_mode, ChatQueueMode::Guided);
        assert!(guided_events[0].activate_assistant);
        assert_eq!(
            guided_events[0]
                .runtime_context
                .as_ref()
                .and_then(|ctx| ctx.dispatch_reason.as_deref()),
            Some("guided_queue")
        );

        let slots = state
            .conversation_runtime_slots
            .lock()
            .expect("lock slots");
        let slot = slots.get("conversation-a").expect("conversation slot");
        assert_eq!(slot.state, MainSessionState::OrganizingContext);
        assert_eq!(slot.pending_queue.len(), 1);
        assert_eq!(slot.pending_queue[0].id, normal_event_id);
        drop(slots);

        let claims = state
            .conversation_processing_claims
            .lock()
            .expect("lock claims");
        assert!(claims.contains("conversation-a"));
    }

    #[test]
    fn process_guided_queue_when_idle_should_remove_claimed_guided_event_after_failure() {
        let state = test_chat_runtime_state();
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::AssistantStreaming)
            .expect("set streaming state");
        let ingress =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("queue event");
        let event_id = match ingress {
            ChatEventIngress::Queued { event_id } => event_id,
            _ => panic!("expected queued event"),
        };
        mark_queue_event_guided(&state, &event_id).expect("mark guided");
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::Idle)
            .expect("restore idle");

        let err = test_runtime()
            .block_on(process_guided_queue_when_idle(&state, "conversation-a"))
            .expect_err("guided processing should fail without conversation");
        assert!(err.contains("目标会话不存在"));
        assert_eq!(total_queue_len(&state).expect("queue len"), 0);
        assert_eq!(
            get_conversation_runtime_state(&state, "conversation-a")
                .expect("runtime state"),
            MainSessionState::Idle
        );
        let claims = state
            .conversation_processing_claims
            .lock()
            .expect("lock claims");
        assert!(!claims.contains("conversation-a"));
    }

    #[test]
    fn guided_batch_should_fail_when_history_flushed_without_activation() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let conversation = test_chat_conversation("conversation-guided-no-activation", "active", &now);
        write_conversation_shard(&state.data_path, &conversation).expect("write conversation");

        let mut guided_event = test_pending_event("conversation-guided-no-activation");
        guided_event.queue_mode = ChatQueueMode::Guided;
        guided_event.activate_assistant = false;
        guided_event.runtime_context = Some(runtime_context_new("user_message", "guided_queue"));

        let err = test_runtime()
            .block_on(process_conversation_batch(
                &state,
                "conversation-guided-no-activation",
                vec![guided_event],
            ))
            .expect_err("guided batch should fail");
        assert_eq!(err, "引导消息未能触发助理回复");

        let updated = state_read_conversation_cached(&state, "conversation-guided-no-activation")
            .expect("read updated conversation");
        assert!(updated.messages.len() >= 1);
        assert_eq!(
            updated
                .messages
                .last()
                .map(|message| message.role.as_str()),
            Some("user")
        );
    }

    #[test]
    fn scheduler_should_allow_eight_conversations_and_queue_the_ninth() {
        let state = test_chat_runtime_state();
        for idx in 0..8 {
            let conversation_id = format!("conversation-{idx}");
            let ingress = ingress_chat_event(&state, test_pending_event(&conversation_id))
                .unwrap_or_else(|_| panic!("ingress {conversation_id}"));
            assert!(
                matches!(ingress, ChatEventIngress::Direct(_)),
                "expected direct ingress for {conversation_id}"
            );
        }

        let ninth = ingress_chat_event(&state, test_pending_event("conversation-8"))
            .expect("ingress ninth");
        assert!(matches!(ninth, ChatEventIngress::Queued { .. }));
        assert_eq!(total_queue_len(&state).expect("queue len"), 1);

        let claims = state
            .conversation_processing_claims
            .lock()
            .expect("lock claims");
        assert_eq!(claims.len(), 8);
        assert!(!claims.contains("conversation-8"));
    }

    #[test]
    fn compaction_state_should_only_block_its_own_conversation() {
        let state = test_chat_runtime_state();
        set_conversation_runtime_state(&state, "conversation-a", MainSessionState::OrganizingContext)
            .expect("set conversation state");

        let ingress_same =
            ingress_chat_event(&state, test_pending_event("conversation-a")).expect("same ingress");
        let ingress_other =
            ingress_chat_event(&state, test_pending_event("conversation-b")).expect("other ingress");

        assert!(matches!(ingress_same, ChatEventIngress::Queued { .. }));
        assert!(matches!(ingress_other, ChatEventIngress::Direct(_)));
    }

    #[test]
    fn ensure_main_conversation_index_should_keep_notification_home_stable() {
        let now = now_iso();
        let later = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format later");
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![
            test_chat_conversation("conversation-main", "inactive", &now),
            test_chat_conversation("conversation-sub", "active", &later),
        ];

        let idx = ensure_main_conversation_index(&mut data, "", DEFAULT_AGENT_ID);

        assert_eq!(data.conversations[idx].id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(data.conversations[idx].title, "P-ai系统");
        assert_eq!(
            data.conversations[idx].conversation_kind,
            CONVERSATION_KIND_SYSTEM_NOTIFICATION
        );
        assert_eq!(
            data.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        let previous_main = data
            .conversations
            .iter()
            .find(|conversation| conversation.id == "conversation-main")
            .expect("previous main conversation should stay as a normal chat");
        assert!(conversation_is_local_normal_chat(previous_main));
    }

    #[test]
    fn normalize_single_active_main_conversation_should_keep_all_foreground_chats_active() {
        let now = now_iso();
        let later = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format later");
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![
            test_chat_conversation("conversation-main", "inactive", &now),
            test_chat_conversation("conversation-sub", "active", &later),
        ];

        let changed = normalize_single_active_main_conversation(&mut data);

        assert!(changed);
        assert_eq!(data.conversations[0].status, "active");
        assert_eq!(data.conversations[1].status, "active");
    }

    #[test]
    fn collect_unarchived_conversation_summaries_should_include_last_two_preview_messages() {
        let state = test_chat_runtime_state();
        let first = now_iso();
        let second = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format second");
        let third = (now_utc() + time::Duration::minutes(2))
            .format(&Rfc3339)
            .expect("format third");
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        let mut conversation = test_chat_conversation("conversation-main", "active", &third);
        conversation.messages = vec![
            ChatMessage {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                created_at: first.clone(),
                speaker_agent_id: Some("user-persona".to_string()),
                parts: vec![MessagePart::Text {
                    text: "第一条".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: "msg-2".to_string(),
                role: "assistant".to_string(),
                created_at: second.clone(),
                speaker_agent_id: Some(DEFAULT_AGENT_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "第二条".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            },
            ChatMessage {
                id: "msg-3".to_string(),
                role: "user".to_string(),
                created_at: third.clone(),
                speaker_agent_id: Some("user-persona".to_string()),
                parts: vec![MessagePart::Text {
                    text: "第三条".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "attachments": [
                        {
                            "fileName": "notes.txt",
                            "relativePath": "downloads/notes.txt"
                        }
                    ]
                })),
                tool_call: None,
                mcp_call: None,
            },
        ];
        data.conversations = vec![conversation];

        let summaries = collect_unarchived_conversation_summaries(&state, &AppConfig::default(), &data);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].workspace_label, "");
        assert!(!summaries[0].detached_window_open);
        assert_eq!(summaries[0].detached_window_label, None);
        assert_eq!(summaries[0].preview_messages.len(), 2);
        assert_eq!(summaries[0].preview_messages[0].message_id, "msg-2");
        assert_eq!(summaries[0].preview_messages[0].text_preview, "第二条");
        assert_eq!(summaries[0].preview_messages[1].message_id, "msg-3");
        assert_eq!(summaries[0].preview_messages[1].text_preview, "第三条");
        assert!(summaries[0].preview_messages[1].has_attachment);
    }

    #[test]
    fn task_resolve_dispatch_session_should_prefer_task_bound_conversation() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");

        let data = test_user_switched_to_sub_conversation_data();
        state_write_app_data_cached(&state, &data).expect("write app data");
        let task = TaskRecordStored {
            task_id: "task-a".to_string(),
            conversation_id: Some("conversation-sub".to_string()),
            target_scope: TASK_TARGET_SCOPE_DESKTOP.to_string(),
            order_index: 1,
            title: "t".to_string(),
            cause: String::new(),
            goal: String::new(),
            flow: String::new(),
            todos: Vec::new(),
            status_summary: String::new(),
            completion_state: TASK_STATE_ACTIVE.to_string(),
            completion_conclusion: String::new(),
            progress_notes: Vec::new(),
            stage_key: String::new(),
            stage_updated_at_utc: None,
            trigger: TaskTriggerStored {
                run_at_utc: None,
                cron_expression: None,
                legacy_every_minutes: None,
                end_at_utc: None,
                next_run_at_utc: None,
            },
            created_at_utc: now_utc_rfc3339(),
            updated_at_utc: now_utc_rfc3339(),
            last_triggered_at_utc: None,
            completed_at_utc: None,
        };

        let session = task_resolve_dispatch_session(&state, &task)
            .expect("resolve task session")
            .expect("dispatch session");

        assert_eq!(session.conversation_id, "conversation-sub");
        assert_eq!(session.target_scope, TASK_TARGET_SCOPE_DESKTOP);
    }

    #[test]
    fn task_resolve_dispatch_session_should_create_task_conversation_when_bound_conversation_missing() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let data = test_user_switched_to_sub_conversation_data();
        state_write_app_data_cached(&state, &data).expect("write app data");
        let task = TaskRecordStored {
            task_id: "task-b".to_string(),
            conversation_id: Some("conversation-missing".to_string()),
            target_scope: TASK_TARGET_SCOPE_DESKTOP.to_string(),
            order_index: 1,
            title: "t".to_string(),
            cause: String::new(),
            goal: String::new(),
            flow: String::new(),
            todos: Vec::new(),
            status_summary: String::new(),
            completion_state: TASK_STATE_ACTIVE.to_string(),
            completion_conclusion: String::new(),
            progress_notes: Vec::new(),
            stage_key: String::new(),
            stage_updated_at_utc: None,
            trigger: TaskTriggerStored {
                run_at_utc: None,
                cron_expression: None,
                legacy_every_minutes: None,
                end_at_utc: None,
                next_run_at_utc: None,
            },
            created_at_utc: now_utc_rfc3339(),
            updated_at_utc: now_utc_rfc3339(),
            last_triggered_at_utc: None,
            completed_at_utc: None,
        };

        let session = task_resolve_dispatch_session(&state, &task)
            .expect("resolve task session")
            .expect("dispatch session");
        let runtime = state_read_runtime_state_cached(&state).expect("read runtime");
        let task_conversation =
            state_read_conversation_cached(&state, &session.conversation_id).expect("read task conversation");
        let sub_conversation =
            state_read_conversation_cached(&state, "conversation-sub").expect("read sub conversation");

        assert_ne!(session.conversation_id, "conversation-main");
        assert_ne!(session.conversation_id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert!(session.fallback_to_main);
        assert_eq!(
            runtime.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        assert!(conversation_is_local_normal_chat(&task_conversation));
        assert_eq!(sub_conversation.status, "active");
    }

    #[test]
    fn task_resolve_dispatch_session_should_skip_missing_contact_conversation() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");

        let mut data = test_user_switched_to_sub_conversation_data();
        data.remote_im_contacts.push(RemoteImContact {
            id: "contact-a".to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "group".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "测试群".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "never".to_string(),
            activation_keywords: Vec::new(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
            bound_conversation_id: Some("conversation-contact-missing".to_string()),
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        });
        state_write_app_data_cached(&state, &data).expect("write app data");
        let task = TaskRecordStored {
            task_id: "task-contact".to_string(),
            conversation_id: Some("conversation-contact-missing".to_string()),
            target_scope: TASK_TARGET_SCOPE_CONTACT.to_string(),
            order_index: 1,
            title: "t".to_string(),
            cause: String::new(),
            goal: String::new(),
            flow: String::new(),
            todos: Vec::new(),
            status_summary: String::new(),
            completion_state: TASK_STATE_ACTIVE.to_string(),
            completion_conclusion: String::new(),
            progress_notes: Vec::new(),
            stage_key: String::new(),
            stage_updated_at_utc: None,
            trigger: TaskTriggerStored {
                run_at_utc: None,
                cron_expression: None,
                legacy_every_minutes: None,
                end_at_utc: None,
                next_run_at_utc: None,
            },
            created_at_utc: now_utc_rfc3339(),
            updated_at_utc: now_utc_rfc3339(),
            last_triggered_at_utc: None,
            completed_at_utc: None,
        };

        let session = task_resolve_dispatch_session(&state, &task).expect("resolve task session");

        assert!(session.is_none());
    }

    #[test]
    fn task_resolve_dispatch_session_should_use_private_department_for_runtime_assistant() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let private_departments_dir = app_root_from_data_path(&state.data_path)
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&private_departments_dir)
            .expect("create private departments dir");
        std::fs::write(
            private_departments_dir.join("dept-private.json"),
            r#"{
  "id": "dept-private",
  "name": "私域任务部门",
  "agentIds": ["private-agent"]
}"#,
        )
        .expect("write private department");
        state_write_agents_cached(
            &state,
            &[{
                let mut agent = default_agent();
                agent.id = "private-agent".to_string();
                agent.name = "私域任务助理".to_string();
                agent
            }, default_user_persona()],
        )
        .expect("write agents");
        let mut runtime = RuntimeStateFile::default();
        runtime.assistant_department_agent_id = "private-agent".to_string();
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");
        let task = TaskRecordStored {
            task_id: "task-private-dept".to_string(),
            conversation_id: None,
            target_scope: TASK_TARGET_SCOPE_DESKTOP.to_string(),
            order_index: 1,
            title: "t".to_string(),
            cause: String::new(),
            goal: String::new(),
            flow: String::new(),
            todos: Vec::new(),
            status_summary: String::new(),
            completion_state: TASK_STATE_ACTIVE.to_string(),
            completion_conclusion: String::new(),
            progress_notes: Vec::new(),
            stage_key: String::new(),
            stage_updated_at_utc: None,
            trigger: TaskTriggerStored {
                run_at_utc: None,
                cron_expression: None,
                legacy_every_minutes: None,
                end_at_utc: None,
                next_run_at_utc: None,
            },
            created_at_utc: now_utc_rfc3339(),
            updated_at_utc: now_utc_rfc3339(),
            last_triggered_at_utc: None,
            completed_at_utc: None,
        };

        let session = task_resolve_dispatch_session(&state, &task)
            .expect("resolve task session")
            .expect("dispatch session");
        let conversation =
            state_read_conversation_cached(&state, &session.conversation_id).expect("read conversation");

        assert_eq!(session.department_id, "dept-private");
        assert_eq!(conversation.department_id, "dept-private");
        assert_eq!(conversation.agent_id, "private-agent");
    }

    #[test]
    fn task_conversation_last_message_is_system_persona_should_detect_system_message() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![Conversation {
            id: "conversation-main".to_string(),
            title: "main".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_user_at: None,
            last_assistant_at: Some(now.clone()),
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                created_at: now.clone(),
                speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
                parts: vec![MessagePart::Text {
                    text: "任务提醒".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            }],
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }];
        state_write_app_data_cached(&state, &data).expect("write app data");

        let blocked = task_conversation_last_message_is_system_persona(&state, "conversation-main")
            .expect("check last system message");

        assert!(blocked);
    }

    #[test]
    fn task_build_dispatch_candidates_should_keep_oldest_due_task_per_conversation() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let data = test_user_switched_to_sub_conversation_data();
        state_write_app_data_cached(&state, &data).expect("write app data");

        let create_task = |goal: &str, conversation_id: &str| {
            task_store_create_task(&state.data_path, &TaskCreateInput {
                goal: goal.to_string(),
                conversation_id: Some(conversation_id.to_string()),
                target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
                why: String::new(),
                todo: String::new(),
                trigger: TaskTriggerInputLocal {
                    run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                    cron_expression: Some("0,30 * * * *".to_string()),
                    end_at: Some("2099-04-10T12:00:00+08:00".to_string()),
                    legacy_every_minutes: None,
                },
            })
            .expect("create task")
        };
        let task_1 = create_task("t1", "conversation-main");
        let task_2 = create_task("t2", "conversation-main");
        let task_3 = create_task("t3", "conversation-sub");
        let tasks = vec![
            task_store_get_task_record(&state.data_path, &task_1.task_id).expect("get task 1"),
            task_store_get_task_record(&state.data_path, &task_2.task_id).expect("get task 2"),
            task_store_get_task_record(&state.data_path, &task_3.task_id).expect("get task 3"),
        ];

        let candidates =
            task_build_dispatch_candidates(&state, tasks, now_utc()).expect("build dispatch candidates");

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].task.task_id, task_1.task_id);
        assert_eq!(candidates[0].session.conversation_id, "conversation-main");
        assert_eq!(candidates[1].task.task_id, task_3.task_id);
        assert_eq!(candidates[1].session.conversation_id, "conversation-sub");
    }

    #[test]
    fn task_build_dispatch_candidates_should_skip_busy_conversation() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let data = test_user_switched_to_sub_conversation_data();
        state_write_app_data_cached(&state, &data).expect("write app data");
        set_conversation_runtime_state(&state, "conversation-main", MainSessionState::OrganizingContext)
            .expect("set busy");

        let created = task_store_create_task(&state.data_path, &TaskCreateInput {
            goal: "busy".to_string(),
            conversation_id: Some("conversation-main".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: String::new(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: Some("0,30 * * * *".to_string()),
                end_at: Some("2099-04-10T12:00:00+08:00".to_string()),
                legacy_every_minutes: None,
            },
        })
        .expect("create busy task");
        let tasks = vec![task_store_get_task_record(&state.data_path, &created.task_id)
            .expect("get busy task record")];

        let candidates =
            task_build_dispatch_candidates(&state, tasks, now_utc()).expect("build dispatch candidates");

        assert!(candidates.is_empty());
        let skipped = task_store_get_task_record(&state.data_path, &created.task_id)
            .expect("get skipped task record");
        assert!(skipped.last_triggered_at_utc.is_some());
    }

    #[test]
    fn task_try_ingress_chat_event_direct_should_skip_when_queue_not_empty() {
        let state = test_chat_runtime_state();
        let existing = ingress_chat_event(&state, test_pending_event("conversation-main"))
            .expect("existing ingress");
        assert!(matches!(existing, ChatEventIngress::Direct(_)));
        let queued = ingress_chat_event(&state, test_pending_event("conversation-main"))
            .expect("queued ingress");
        assert!(matches!(queued, ChatEventIngress::Queued { .. }));

        let result = task_try_ingress_chat_event_direct(&state, test_pending_event("conversation-main"))
            .expect("task direct ingress");

        assert!(matches!(result, Err("conversation_queue_not_empty")));
    }

    #[test]
    fn delegate_parse_session_parts_should_preserve_conversation_in_two_segment_session() {
        let (api_config_id, agent_id, conversation_id) =
            delegate_parse_session_parts("default-agent::conversation-sub");

        assert_eq!(api_config_id, "");
        assert_eq!(agent_id, "default-agent");
        assert_eq!(conversation_id.as_deref(), Some("conversation-sub"));
    }

    #[test]
    fn delegate_parse_session_parts_should_reject_legacy_three_segment_session() {
        let (api_config_id, agent_id, conversation_id) =
            delegate_parse_session_parts("api-config-a::default-agent::conversation-sub");

        assert_eq!(api_config_id, "");
        assert_eq!(agent_id, "");
        assert_eq!(conversation_id, None);
    }

    #[test]
    fn assemble_runtime_tools_should_gate_delegate_by_session_conversation_department() {
        let state = test_chat_runtime_state();
        let mut shared_agent = default_agent();
        shared_agent.id = "shared-agent".to_string();
        shared_agent.name = "共享人格".to_string();

        let mut selected_api = ApiConfig::default();
        selected_api.id = "api-a".to_string();
        selected_api.name = "测试模型".to_string();
        selected_api.request_format = RequestFormat::OpenAI;
        selected_api.enable_text = true;
        selected_api.enable_tools = true;
        selected_api.base_url = "https://api.openai.com/v1".to_string();
        selected_api.api_key = "k".to_string();
        selected_api.model = "gpt-4o-mini".to_string();

        let mut parent = default_assistant_department(&selected_api.id);
        parent.id = "dept-parent".to_string();
        parent.name = "父部门".to_string();
        parent.is_built_in_assistant = false;
        parent.agent_ids = vec![shared_agent.id.clone()];
        parent.child_department_ids = vec!["dept-child".to_string()];

        let mut child = default_assistant_department(&selected_api.id);
        child.id = "dept-child".to_string();
        child.name = "子部门".to_string();
        child.is_built_in_assistant = false;
        child.agent_ids = vec![shared_agent.id.clone()];
        child.child_department_ids = Vec::new();

        let config = AppConfig {
            departments: vec![parent, child],
            api_configs: vec![selected_api.clone()],
            api_providers: Vec::new(),
            ..AppConfig::default()
        };
        write_config(&state.config_path, &config).expect("write config");
        state_write_agents_cached(&state, &[shared_agent.clone(), default_user_persona()])
            .expect("write agents");

        let conversation = build_conversation_record(
            &selected_api.id,
            &shared_agent.id,
            "dept-child",
            "叶子部门会话",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");
        refresh_global_tool_schema_cache(&state);

        let session_id = format!("{}::{}", shared_agent.id, conversation.id);
        let assembly = test_runtime()
            .block_on(assemble_runtime_tools(
                &config,
                &selected_api,
                &shared_agent,
                Some(&state),
                &session_id,
            ))
            .expect("assemble runtime tools");

        assert!(assembly.tool_manifest.iter().any(|item| {
            item.get("source").and_then(Value::as_str) == Some("runtime_policy")
                && item.get("name").and_then(Value::as_str) == Some("delegate")
                && item.get("enabled").and_then(Value::as_bool) == Some(false)
                && item.get("reason").and_then(Value::as_str)
                    == Some("当前部门没有直接下级，无法使用委托")
        }));
        assert!(!assembly.tools.iter().any(|tool| tool.name() == "delegate"));
    }

    #[test]
    fn common_delegate_preflight_should_resolve_source_department_from_delegate_thread() {
        let state = test_chat_runtime_state();
        let mut shared_agent = default_agent();
        shared_agent.id = "shared-agent".to_string();
        shared_agent.name = "共享人格".to_string();

        let mut target_agent = default_agent();
        target_agent.id = "target-agent".to_string();
        target_agent.name = "孙部门人格".to_string();

        let mut parent = default_assistant_department("api-a");
        parent.id = "dept-parent".to_string();
        parent.name = "父部门".to_string();
        parent.is_built_in_assistant = false;
        parent.agent_ids = vec![shared_agent.id.clone()];
        parent.child_department_ids = vec!["dept-child".to_string()];

        let mut child = default_assistant_department("api-a");
        child.id = "dept-child".to_string();
        child.name = "子部门".to_string();
        child.is_built_in_assistant = false;
        child.agent_ids = vec![shared_agent.id.clone()];
        child.child_department_ids = vec!["dept-grandchild".to_string()];

        let mut grandchild = default_assistant_department("api-a");
        grandchild.id = "dept-grandchild".to_string();
        grandchild.name = "孙部门".to_string();
        grandchild.is_built_in_assistant = false;
        grandchild.agent_ids = vec![target_agent.id.clone()];
        grandchild.child_department_ids = Vec::new();

        let config = AppConfig {
            departments: vec![parent, child, grandchild],
            ..AppConfig::default()
        };
        write_config(&state.config_path, &config).expect("write config");
        state_write_agents_cached(
            &state,
            &[
                shared_agent.clone(),
                target_agent.clone(),
                default_user_persona(),
            ],
        )
        .expect("write agents");

        let mut delegate_conversation = build_conversation_record(
            "api-a",
            &shared_agent.id,
            "dept-child",
            "子部门委托线程",
            CONVERSATION_KIND_DELEGATE,
            Some("conversation-root".to_string()),
            Some("delegate-child".to_string()),
        );
        delegate_conversation.id = "delegate-child".to_string();
        let thread = DelegateRuntimeThread {
            delegate_id: "delegate-child".to_string(),
            root_conversation_id: "conversation-root".to_string(),
            target_agent_id: shared_agent.id.clone(),
            title: "子部门委托线程".to_string(),
            call_stack: vec!["dept-parent".to_string(), "dept-child".to_string()],
            parent_chat_session_key: Some(format!("{}::conversation-root", shared_agent.id)),
            archived_at: None,
            conversation: delegate_conversation,
        };
        state
            .delegate_runtime_threads
            .lock()
            .expect("delegate runtime threads")
            .insert(thread.delegate_id.clone(), thread);

        let preflight = common_delegate_preflight(
            &state,
            &shared_agent.id,
            Some("delegate-child"),
            "dept-grandchild",
        )
        .expect("resolve nested sync delegate preflight");
        let call_stack = resolve_delegate_call_stack(
            preflight.current_thread.as_ref(),
            &preflight.source_department,
            &preflight.target_department,
        )
        .expect("resolve call stack");

        assert_eq!(preflight.source_department.id, "dept-child");
        assert_eq!(preflight.target_department.id, "dept-grandchild");
        assert_eq!(preflight.target_agent_id, target_agent.id);
        assert_eq!(preflight.root_conversation_id, "conversation-root");
        assert_eq!(
            call_stack,
            vec![
                "dept-parent".to_string(),
                "dept-child".to_string(),
                "dept-grandchild".to_string(),
            ]
        );
    }

    #[test]
    fn common_delegate_preflight_should_not_require_direct_child_department() {
        let state = test_chat_runtime_state();
        let mut source_agent = default_agent();
        source_agent.id = "source-agent".to_string();
        source_agent.name = "源部门人格".to_string();

        let mut target_agent = default_agent();
        target_agent.id = "target-agent".to_string();
        target_agent.name = "目标部门人格".to_string();

        let mut source_department = default_assistant_department("api-a");
        source_department.id = "dept-source".to_string();
        source_department.name = "源部门".to_string();
        source_department.is_built_in_assistant = false;
        source_department.agent_ids = vec![source_agent.id.clone()];
        source_department.child_department_ids = Vec::new();

        let mut target_department = default_assistant_department("api-a");
        target_department.id = "dept-target".to_string();
        target_department.name = "目标部门".to_string();
        target_department.is_built_in_assistant = false;
        target_department.agent_ids = vec![target_agent.id.clone()];

        let config = AppConfig {
            departments: vec![source_department, target_department],
            ..AppConfig::default()
        };
        write_config(&state.config_path, &config).expect("write config");
        state_write_agents_cached(
            &state,
            &[
                source_agent.clone(),
                target_agent.clone(),
                default_user_persona(),
            ],
        )
        .expect("write agents");

        let conversation = build_conversation_record(
            "api-a",
            &source_agent.id,
            "dept-source",
            "源部门会话",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");

        let preflight = common_delegate_preflight(
            &state,
            &source_agent.id,
            Some(&conversation.id),
            "dept-target",
        )
        .expect("delegate scheduling should allow any target department");

        assert_eq!(preflight.source_department.id, "dept-source");
        assert_eq!(preflight.target_department.id, "dept-target");
        assert_eq!(preflight.target_agent_id, target_agent.id);
        assert_eq!(preflight.root_conversation_id, conversation.id);
        assert!(validate_delegate_tool_direct_child_target(&preflight).is_err());
    }

    #[test]
    fn common_delegate_preflight_should_accept_private_child_department_agent() {
        let state = test_chat_runtime_state();
        let private_departments_dir = app_root_from_data_path(&state.data_path)
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&private_departments_dir)
            .expect("create private departments dir");
        std::fs::write(
            private_departments_dir.join("dept-private.json"),
            r#"{
  "id": "dept-private",
  "name": "私域子部门",
  "agentIds": ["private-agent"]
}"#,
        )
        .expect("write private department");

        let mut parent_agent = default_agent();
        parent_agent.id = "parent-agent".to_string();
        parent_agent.name = "主部门人格".to_string();

        let mut private_agent = default_agent();
        private_agent.id = "private-agent".to_string();
        private_agent.name = "私域部门人格".to_string();

        let mut parent = default_assistant_department("api-a");
        parent.id = "dept-parent".to_string();
        parent.name = "主部门".to_string();
        parent.is_built_in_assistant = false;
        parent.agent_ids = vec![parent_agent.id.clone()];
        parent.child_department_ids = vec!["dept-private".to_string()];

        let config = AppConfig {
            departments: vec![parent],
            ..AppConfig::default()
        };
        write_config(&state.config_path, &config).expect("write config");
        state_write_agents_cached(
            &state,
            &[
                parent_agent.clone(),
                private_agent.clone(),
                default_user_persona(),
            ],
        )
        .expect("write agents");

        let conversation = build_conversation_record(
            "api-a",
            &parent_agent.id,
            "dept-parent",
            "主部门会话",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");

        let preflight = common_delegate_preflight(
            &state,
            &parent_agent.id,
            Some(&conversation.id),
            "dept-private",
        )
        .expect("resolve private child delegate preflight");

        assert_eq!(preflight.source_department.id, "dept-parent");
        assert_eq!(preflight.target_department.id, "dept-private");
        assert_eq!(preflight.target_agent_id, private_agent.id);
        assert_eq!(preflight.root_conversation_id, conversation.id);
    }

    #[test]
    fn resolve_user_async_delegate_plan_should_accept_private_child_department() {
        let state = test_chat_runtime_state();
        let private_departments_dir = app_root_from_data_path(&state.data_path)
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&private_departments_dir)
            .expect("create private departments dir");
        std::fs::write(
            private_departments_dir.join("dept-private.json"),
            r#"{
  "id": "dept-private",
  "name": "私域子部门",
  "agentIds": ["private-agent"]
}"#,
        )
        .expect("write private department");

        let mut selected_api = ApiConfig::default();
        selected_api.id = "api-a".to_string();
        selected_api.name = "测试模型".to_string();
        selected_api.request_format = RequestFormat::OpenAI;
        selected_api.enable_text = true;
        selected_api.enable_tools = true;
        selected_api.base_url = "https://api.openai.com/v1".to_string();
        selected_api.api_key = "k".to_string();
        selected_api.model = "gpt-4o-mini".to_string();

        let mut parent_agent = default_agent();
        parent_agent.id = "parent-agent".to_string();
        parent_agent.name = "主部门人格".to_string();

        let mut private_agent = default_agent();
        private_agent.id = "private-agent".to_string();
        private_agent.name = "私域部门人格".to_string();

        let mut parent = default_assistant_department(&selected_api.id);
        parent.id = "dept-parent".to_string();
        parent.name = "主部门".to_string();
        parent.is_built_in_assistant = false;
        parent.agent_ids = vec![parent_agent.id.clone()];
        parent.child_department_ids = vec!["dept-private".to_string()];

        let config = AppConfig {
            departments: vec![parent],
            api_configs: vec![selected_api.clone()],
            api_providers: Vec::new(),
            selected_api_config_id: selected_api.id.clone(),
            assistant_department_api_config_id: selected_api.id.clone(),
            ..AppConfig::default()
        };
        write_config(&state.config_path, &config).expect("write config");
        state_write_agents_cached(
            &state,
            &[
                parent_agent.clone(),
                private_agent.clone(),
                default_user_persona(),
            ],
        )
        .expect("write agents");

        let conversation = build_conversation_record(
            &selected_api.id,
            &parent_agent.id,
            "dept-parent",
            "主部门会话",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        state_schedule_conversation_persist(&state, &conversation)
            .expect("persist conversation");

        let (plan, selected_count) = resolve_user_async_delegate_plan(
            &state,
            &SubmitUserAsyncDelegateInput {
                conversation_id: conversation.id.clone(),
                target_department_id: "dept-private".to_string(),
                preset_id: None,
                background: String::new(),
                question: "请调查这个问题".to_string(),
                focus: String::new(),
                selected_message_ids: Vec::new(),
            },
        )
        .expect("resolve async delegate plan");

        assert_eq!(selected_count, 0);
        assert_eq!(plan.root_conversation_id, conversation.id);
        assert_eq!(plan.source_department_id, "dept-parent");
        assert_eq!(plan.source_agent_id, parent_agent.id);
        assert_eq!(plan.target_department_id, "dept-private");
        assert_eq!(plan.target_agent_id, private_agent.id);
        assert_eq!(plan.target_agent_name, "私域部门人格");
        assert_eq!(
            plan.target_api_config_ids,
            vec![api_endpoint_id("api-a", "api-a-model-default")]
        );
    }

    #[test]
    fn delegate_target_chat_api_config_ids_should_only_keep_current_department_models() {
        let app_config = AppConfig {
            api_configs: vec![ApiConfig {
                id: "provider-a::model-a".to_string(),
                name: "provider-a/model-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                model: "gpt-4o-mini".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 1.0,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            api_providers: Vec::new(),
            ..AppConfig::default()
        };
        let department = DepartmentConfig {
            id: "dept-a".to_string(),
            name: "部门 A".to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: vec!["provider-a".to_string(), "provider-a::model-a".to_string()],
            api_config_id: "provider-a".to_string(),
            model_failure_fallback_enabled: true,
            agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
            child_department_ids: Vec::new(),
            created_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
            order_index: 1,
            is_built_in_assistant: false,
            is_deputy: false,
            source: "main_config".to_string(),
            scope: "global".to_string(),
            permission_control: DepartmentPermissionControl::default(),
        };

        let resolved = delegate_target_chat_api_config_ids(&app_config, &department);

        assert_eq!(resolved, vec!["provider-a::model-a".to_string()]);
    }

    #[test]
    fn delegate_target_chat_api_config_ids_should_resolve_model_roles() {
        let expert_id = "provider-a::expert";
        let quick_id = "provider-a::quick";
        let app_config = AppConfig {
            api_configs: vec![
                ApiConfig {
                    id: expert_id.to_string(),
                    name: "provider-a/expert".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_tools: true,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    model: "expert".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
                ApiConfig {
                    id: quick_id.to_string(),
                    name: "provider-a/quick".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_tools: true,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    model: "quick".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
            ],
            api_providers: Vec::new(),
            assistant_department_api_config_id: expert_id.to_string(),
            tool_review_api_config_id: Some(quick_id.to_string()),
            ..AppConfig::default()
        };
        let department = DepartmentConfig {
            id: "dept-a".to_string(),
            name: "部门 A".to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: vec![
                MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string(),
                MODEL_ROLE_QUICK_API_CONFIG_ID.to_string(),
            ],
            api_config_id: MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string(),
            model_failure_fallback_enabled: true,
            agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
            child_department_ids: Vec::new(),
            created_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
            order_index: 1,
            is_built_in_assistant: false,
            is_deputy: false,
            source: "main_config".to_string(),
            scope: "global".to_string(),
            permission_control: DepartmentPermissionControl::default(),
        };

        let resolved = delegate_target_chat_api_config_ids(&app_config, &department);

        assert_eq!(resolved, vec![expert_id.to_string(), quick_id.to_string()]);
    }

    #[test]
    fn delegate_target_chat_api_config_ids_should_keep_only_primary_when_fallback_disabled() {
        let expert_id = "provider-a::expert";
        let quick_id = "provider-a::quick";
        let app_config = AppConfig {
            api_configs: vec![
                ApiConfig {
                    id: expert_id.to_string(),
                    name: "provider-a/expert".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_tools: true,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    model: "expert".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
                ApiConfig {
                    id: quick_id.to_string(),
                    name: "provider-a/quick".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_tools: true,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    model: "quick".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
            ],
            api_providers: Vec::new(),
            assistant_department_api_config_id: expert_id.to_string(),
            tool_review_api_config_id: Some(quick_id.to_string()),
            ..AppConfig::default()
        };
        let department = DepartmentConfig {
            id: "dept-a".to_string(),
            name: "部门 A".to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: vec![
                MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string(),
                MODEL_ROLE_QUICK_API_CONFIG_ID.to_string(),
            ],
            api_config_id: MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string(),
            model_failure_fallback_enabled: false,
            agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
            child_department_ids: Vec::new(),
            created_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
            order_index: 1,
            is_built_in_assistant: false,
            is_deputy: false,
            source: "main_config".to_string(),
            scope: "global".to_string(),
            permission_control: DepartmentPermissionControl::default(),
        };

        let resolved = delegate_target_chat_api_config_ids(&app_config, &department);

        assert_eq!(resolved, vec![expert_id.to_string()]);
    }

    #[test]
    fn build_user_mention_dispatch_plans_should_allow_non_child_department_mentions() {
        let mut source_agent = default_agent();
        source_agent.id = "source-agent".to_string();
        source_agent.name = "主负责人".to_string();

        let mut target_agent = default_agent();
        target_agent.id = "target-agent".to_string();
        target_agent.name = "被@人格".to_string();

        let mut source_department = default_assistant_department("provider-a::model-a");
        source_department.id = "dept-source".to_string();
        source_department.name = "源部门".to_string();
        source_department.is_built_in_assistant = false;
        source_department.agent_ids = vec![source_agent.id.clone()];
        source_department.child_department_ids = Vec::new();

        let mut target_department = default_assistant_department("provider-a::model-a");
        target_department.id = "dept-target".to_string();
        target_department.name = "目标部门".to_string();
        target_department.is_built_in_assistant = false;
        target_department.agent_ids = vec![target_agent.id.clone()];
        target_department.child_department_ids = Vec::new();

        let app_config = AppConfig {
            api_configs: vec![ApiConfig {
                id: "provider-a::model-a".to_string(),
                name: "provider-a/model-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                model: "gpt-4o-mini".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 1.0,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            api_providers: Vec::new(),
            departments: vec![source_department, target_department],
            ..AppConfig::default()
        };
        let conversation = build_conversation_record(
            "provider-a::model-a",
            &source_agent.id,
            "dept-source",
            "主会话",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        let mentions = vec![UserMentionTargetInput {
            agent_id: target_agent.id.clone(),
            agent_name: Some(target_agent.name.clone()),
            department_id: "dept-target".to_string(),
            department_name: Some("目标部门".to_string()),
        }];

        let (plans, failures) = build_user_mention_dispatch_plans(
            &app_config,
            &conversation,
            &[source_agent, target_agent],
            "dept-source",
            "source-agent",
            "请你异步处理这个问题",
            Some(&mentions),
        )
        .expect("build mention dispatch plans");

        assert_eq!(plans.len(), 1);
        assert!(failures.is_empty());
        assert_eq!(plans[0].target_department_id, "dept-target");
        assert_eq!(plans[0].target_agent_id, "target-agent");
    }

    #[test]
    fn build_departments_prompt_block_should_keep_same_persona_child_departments() {
        let agent = default_agent();
        let conversation = build_conversation_record(
            "测试会话",
            &agent.id,
            "dept-parent",
            "",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );

        let mut parent = default_assistant_department("api-a");
        parent.id = "dept-parent".to_string();
        parent.name = "父部门".to_string();
        parent.summary = "当任务需要总控时叫我".to_string();
        parent.guide = "你负责统筹和推进".to_string();
        parent.is_built_in_assistant = false;
        parent.agent_ids = vec![agent.id.clone()];
        parent.child_department_ids = vec!["dept-child".to_string()];

        let mut child = default_assistant_department("api-a");
        child.id = "dept-child".to_string();
        child.name = "同人格子部门".to_string();
        child.summary = "当任务需要专项摸底时叫我".to_string();
        child.is_built_in_assistant = false;
        child.agent_ids = vec![agent.id.clone()];

        let block = build_departments_prompt_block(
            &conversation,
            &agent,
            &[parent, child],
            "zh-CN",
        );

        assert!(block.contains("部门：父部门"));
        assert!(block.contains("部门办事指南：你负责统筹和推进"));
        assert!(!block.contains("部门概述：当任务需要总控时叫我"));
        assert!(block.contains("同人格子部门"));
        assert!(block.contains("概述：当任务需要专项摸底时叫我"));
    }

    #[test]
    fn delegate_target_chat_api_config_ids_should_not_fallback_when_department_binding_invalid() {
        let app_config = AppConfig {
            api_configs: vec![ApiConfig {
                id: "provider-a::model-a".to_string(),
                name: "provider-a/model-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                model: "gpt-4o-mini".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 1.0,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            api_providers: Vec::new(),
            ..AppConfig::default()
        };
        let department = DepartmentConfig {
            id: "dept-a".to_string(),
            name: "部门 A".to_string(),
            summary: String::new(),
            guide: String::new(),
            api_config_ids: vec!["provider-a".to_string()],
            api_config_id: "provider-a".to_string(),
            model_failure_fallback_enabled: false,
            agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
            child_department_ids: Vec::new(),
            created_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
            order_index: 1,
            is_built_in_assistant: false,
            is_deputy: false,
            source: "main_config".to_string(),
            scope: "global".to_string(),
            permission_control: DepartmentPermissionControl::default(),
        };

        let resolved = delegate_target_chat_api_config_ids(&app_config, &department);

        assert!(resolved.is_empty());
    }

    #[test]
    fn update_conversation_todos_and_emit_should_persist_conversation_todos() {
        let state = test_chat_runtime_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        let now = now_utc_rfc3339();
        let mut data = AppData::default();
        data.conversations.push(Conversation {
            id: "conversation-main".to_string(),
            title: "主会话".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: Vec::new(),
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        });
        state_write_app_data_cached(&state, &data).expect("write app data");

        update_conversation_todos_and_emit(
            &state,
            "conversation-main",
            vec![
                ConversationTodoItem {
                    content: "第一步".to_string(),
                    status: "completed".to_string(),
                },
                ConversationTodoItem {
                    content: "第二步".to_string(),
                    status: "in_progress".to_string(),
                },
            ],
        )
        .expect("update conversation todos");

        let data = state_read_app_data_cached(&state).expect("read app data");
        let conversation = data
            .conversations
            .iter()
            .find(|item| item.id == "conversation-main")
            .expect("conversation exists");
        assert_eq!(conversation.current_todos.len(), 2);
        assert_eq!(conversation.current_todos[0].content, "第一步");
        assert_eq!(conversation.current_todos[0].status, "completed");
        assert_eq!(conversation.current_todos[1].content, "第二步");
        assert_eq!(conversation.current_todos[1].status, "in_progress");
        assert_eq!(
            conversation_current_todo_text(conversation).as_deref(),
            Some("第二步")
        );
    }

    #[test]
    fn update_conversation_todos_and_emit_should_clear_todos_when_all_completed() {
        let state = test_chat_runtime_state();
        let now = now_iso();
        let mut data = AppData::default();
        data.conversations.push(Conversation {
            id: "conversation-main".to_string(),
            title: "主会话".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now.clone(),
            updated_at: now,
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: Vec::new(),
            current_todos: vec![ConversationTodoItem {
                content: "旧步骤".to_string(),
                status: "in_progress".to_string(),
            }],
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        });
        state_write_app_data_cached(&state, &data).expect("write app data");

        update_conversation_todos_and_emit(
            &state,
            "conversation-main",
            vec![
                ConversationTodoItem {
                    content: "第一步".to_string(),
                    status: "completed".to_string(),
                },
                ConversationTodoItem {
                    content: "第二步".to_string(),
                    status: "completed".to_string(),
                },
            ],
        )
        .expect("update completed conversation todos");

        let data = state_read_app_data_cached(&state).expect("read app data");
        let conversation = data
            .conversations
            .iter()
            .find(|item| item.id == "conversation-main")
            .expect("conversation exists");
        assert!(conversation.current_todos.is_empty());
        assert_eq!(conversation_current_todo_text(conversation), None);
    }

    #[test]
    fn runtime_context_request_id_or_new_should_prefer_runtime_context() {
        let runtime_context = RuntimeContext {
            request_id: Some("request-from-context".to_string()),
            ..RuntimeContext::default()
        };

        let request_id = runtime_context_request_id_or_new(
            Some(&runtime_context),
            Some("trace-from-input"),
            "chat",
        );

        assert_eq!(request_id, "request-from-context");
    }

    #[test]
    fn runtime_context_new_should_seed_event_source_and_dispatch_reason() {
        let runtime_context = runtime_context_new("task_trigger", "task_due");

        assert_eq!(runtime_context.event_source.as_deref(), Some("task_trigger"));
        assert_eq!(runtime_context.dispatch_reason.as_deref(), Some("task_due"));
    }

    #[test]
    fn resolve_unarchived_conversation_index_with_fallback_should_use_requested_conversation_when_available() {
        let mut data = test_user_switched_to_sub_conversation_data();
        let idx = resolve_unarchived_conversation_index_with_fallback(
            &mut data,
            &AppConfig::default(),
            DEFAULT_AGENT_ID,
            Some("conversation-main"),
        )
        .expect("resolve requested conversation");

        assert_eq!(data.conversations[idx].id, "conversation-main");
    }

    #[test]
    fn resolve_unarchived_conversation_index_with_fallback_should_error_when_requested_missing() {
        let mut data = test_user_switched_to_sub_conversation_data();
        let err = resolve_unarchived_conversation_index_with_fallback(
            &mut data,
            &AppConfig::default(),
            DEFAULT_AGENT_ID,
            Some("conversation-missing"),
        )
        .expect_err("missing requested conversation should fail");

        assert!(err.contains("Requested conversation not found"));
    }

    #[test]
    fn delete_main_conversation_should_promote_existing_sub_conversation() {
        let state = test_chat_runtime_state();
        let config = AppConfig::default();
        write_config(&state.config_path, &config).expect("write config");
        let selected_api = resolve_selected_api_config(&config, None)
            .expect("selected api")
            .clone();

        let now = now_iso();
        let later = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format later");
        let mut source = test_chat_conversation("conversation-main", "active", &now);
        source.summary = "archived summary".to_string();
        let mut data = AppData::default();
        data.main_conversation_id = Some(source.id.clone());
        data.conversations = vec![
            source.clone(),
            test_chat_conversation("conversation-sub", "inactive", &later),
        ];
        state_write_app_data_cached(&state, &data).expect("write app data");

        let next_id = delete_main_conversation_and_activate_latest(&state, &selected_api, &source)
            .expect("delete main conversation");
        let runtime = state_read_runtime_state_cached(&state).expect("read runtime");
        let system_notification = state_read_conversation_cached(
            &state,
            SYSTEM_NOTIFICATION_CONVERSATION_ID,
        )
        .expect("read system notification conversation");
        let promoted = state_read_conversation_cached(&state, &next_id)
            .expect("read promoted conversation");

        assert_eq!(next_id, "conversation-sub");
        assert_eq!(
            runtime.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        assert!(conversation_is_system_notification(&system_notification));
        assert_eq!(promoted.status, "inactive");
        assert_eq!(promoted.summary, "");
    }

    #[test]
    fn delete_last_main_conversation_should_create_replacement_main_conversation() {
        let state = test_chat_runtime_state();
        let config = AppConfig::default();
        write_config(&state.config_path, &config).expect("write config");
        let selected_api = resolve_selected_api_config(&config, None)
            .expect("selected api")
            .clone();

        let now = now_iso();
        let mut source = test_chat_conversation("conversation-main", "active", &now);
        source.summary = "archived summary".to_string();
        let mut data = AppData::default();
        data.main_conversation_id = Some(source.id.clone());
        data.conversations = vec![source.clone()];
        state_write_app_data_cached(&state, &data).expect("write app data");

        let next_id = delete_main_conversation_and_activate_latest(&state, &selected_api, &source)
            .expect("delete last main conversation");
        let runtime = state_read_runtime_state_cached(&state).expect("read runtime");
        let system_notification = state_read_conversation_cached(
            &state,
            SYSTEM_NOTIFICATION_CONVERSATION_ID,
        )
        .expect("read system notification conversation");
        let replacement = state_read_conversation_cached(&state, &next_id)
            .expect("read replacement conversation");

        assert_ne!(next_id, "conversation-main");
        assert_ne!(next_id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(
            runtime.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        assert!(conversation_is_system_notification(&system_notification));
        assert_eq!(replacement.status, "active");
        assert!(replacement.summary.is_empty());
    }

    #[test]
    fn archiving_main_conversation_should_promote_existing_sub_conversation() {
        let now = now_iso();
        let later = (now_utc() + time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("format later");
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![
            test_chat_conversation("conversation-main", "active", &now),
            test_chat_conversation("conversation-sub", "inactive", &later),
        ];

        archive_conversation_now(&mut data, "conversation-main", "test", "archived summary")
            .expect("archive current main");
        let idx = ensure_main_conversation_index(&mut data, "", DEFAULT_AGENT_ID);

        assert_eq!(data.conversations[idx].id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(
            data.conversations[idx].conversation_kind,
            CONVERSATION_KIND_SYSTEM_NOTIFICATION
        );
        assert_eq!(
            data.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
    }

    #[test]
    fn archiving_last_main_conversation_should_create_replacement_main_conversation() {
        let now = now_iso();
        let mut data = AppData::default();
        data.main_conversation_id = Some("conversation-main".to_string());
        data.conversations = vec![test_chat_conversation("conversation-main", "active", &now)];

        archive_conversation_now(&mut data, "conversation-main", "test", "archived summary")
            .expect("archive last main");
        let idx = ensure_main_conversation_index(&mut data, "api-default", DEFAULT_AGENT_ID);

        assert_eq!(data.conversations[idx].id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(
            data.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        assert_eq!(data.conversations[idx].status, "active");
        assert!(data.conversations[idx].summary.is_empty());
    }

    #[test]
    #[ignore = "压测探针：本地按需运行 cargo test prepared_prompt_to_messages_json_large_context_probe -- --ignored --nocapture"]
    fn prepared_prompt_to_messages_json_large_context_probe() {
        let large_text = "上下文片段。".repeat(220_000);
        let prepared = PreparedPrompt {
            preamble: "系统提示词".to_string(),
            history_messages: vec![PreparedHistoryMessage {
                role: "user".to_string(),
                text: large_text.clone(),
                extra_text_blocks: Vec::new(),
                user_time_text: None,
                images: Vec::new(),
                audios: Vec::new(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            latest_user_text: large_text,
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        };

        let started = std::time::Instant::now();
        let messages = prepared_prompt_to_messages_json(&prepared);
        let json_bytes = serde_json::to_vec(&messages).expect("serialize large prepared messages");
        let elapsed_ms = started.elapsed().as_millis();

        eprintln!(
            "[压测] prepared_prompt_to_messages_json 大上下文结果：messages={}，bytes={}，elapsed={}ms",
            messages.len(),
            json_bytes.len(),
            elapsed_ms
        );

        assert!(json_bytes.len() > 1_500_000);
    }

    #[test]
    #[ignore = "压测探针：本地按需运行 cargo test llm_round_log_large_response_probe -- --ignored --nocapture"]
    fn llm_round_log_large_response_probe() {
        let state = test_chat_runtime_state();
        let large_response = "响应片段。".repeat(220_000);
        let response = serde_json::json!({
            "assistantText": large_response,
            "toolHistoryEvents": []
        });

        let started = std::time::Instant::now();
        push_llm_round_log(
            Some(&state),
            Some("trace-large-response".to_string()),
            None,
            "Archive summary",
            RequestFormat::OpenAI,
            "archive-summary",
            "deepseek-chat",
            "http://localhost:5001/v1",
            masked_auth_headers("sk-test"),
            None,
            Some(response),
            None,
            1234,
            None,
        );
        let elapsed_ms = started.elapsed().as_millis();
        let logs = state.llm_round_logs.lock().expect("llm round logs");
        let stored = logs.back().expect("stored log entry");
        let response_bytes = serde_json::to_vec(&stored.response).expect("serialize stored response");

        eprintln!(
            "[压测] llm_round_log_large_response 大响应结果：stored_logs={}，response_bytes={}，elapsed={}ms",
            logs.len(),
            response_bytes.len(),
            elapsed_ms
        );

        assert_eq!(logs.len(), 1);
        assert!(response_bytes.len() > 1_500_000);
    }

    #[test]
    fn model_reply_log_value_should_keep_activity_reasoning_text() {
        let reply = ModelReply {
            assistant_text: "最终答复".to_string(),
            final_response_text: "最终答复".to_string(),
            activity_reasoning_text: "完整思维链".to_string(),
            assistant_provider_meta: None,
            tool_history_events: Vec::new(),
            suppress_assistant_message: false,
            usage: None,
            trusted_input_tokens: None,
            round_logs_recorded_internally: false,
        };

        let value = model_reply_to_log_value(&reply);

        assert_eq!(value["assistantText"].as_str(), Some("最终答复"));
        assert_eq!(
            value["activityReasoningText"].as_str(),
            Some("完整思维链")
        );
    }

    #[test]
    #[ignore = "性能探针：本地按需运行 cargo test build_prepared_prompt_for_mode_perf_probe -- --ignored --nocapture"]
    fn build_prepared_prompt_for_mode_perf_probe() {
        let state = test_chat_runtime_state();
        let agent = default_agent();
        let user = default_user_persona();
        let drafts = (0..12)
            .map(|idx| MemoryDraftInput {
                memory_type: "knowledge".to_string(),
                judgment: format!("用户偏好样本{}", idx),
                reasoning: format!("这是第{}条用于提示词性能探针的记忆。", idx),
                tags: vec!["性能".to_string(), format!("tag{}", idx)],
                owner_agent_id: None,
            })
            .collect::<Vec<_>>();
        let (saved, _) =
            memory_store_upsert_drafts(&state.data_path, &drafts).expect("seed perf probe memories");
        let memory_ids = saved
            .iter()
            .filter_map(|item| item.id.clone())
            .collect::<Vec<_>>();

        let base_time = now_utc();
        let mut messages = Vec::<ChatMessage>::new();
        for idx in 0..80 {
            let created_at = (base_time + time::Duration::seconds(idx as i64))
                .format(&Rfc3339)
                .expect("format probe message time");
            let is_user = idx % 2 == 0;
            let role = if is_user { "user" } else { "assistant" };
            let speaker_agent_id = if is_user {
                Some(USER_PERSONA_ID.to_string())
            } else {
                Some(agent.id.clone())
            };
            let mut provider_meta = None;
            let mut extra_text_blocks = Vec::<String>::new();
            if is_user && idx >= 60 {
                let picked = memory_ids
                    .iter()
                    .skip((idx / 2) % 4)
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>();
                provider_meta = Some(serde_json::json!({
                    "retrieved_memory_ids": picked
                }));
                extra_text_blocks.push(format!("补充上下文块{}", idx));
            }
            messages.push(ChatMessage {
                id: Uuid::new_v4().to_string(),
                role: role.to_string(),
                created_at,
                speaker_agent_id,
                parts: vec![MessagePart::Text {
                    text: format!("这是第{}条{}消息，用于测量提示词主结构构建速度。", idx, role),
                    reasoning_content: None,
                }],
                extra_text_blocks,
                provider_meta,
                tool_call: None,
                mcp_call: None,
            });
        }

        let last_user_at = messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| message.created_at.clone());
        let conversation = test_active_conversation_with_messages(messages, last_user_at);
        let overrides = ChatPromptOverrides {
            latest_user_intent: Some(LatestUserPayloadIntent::Explicit {
                text: String::new(),
                meta_text: String::new(),
                extra_blocks: vec![
                    "这是一个额外的任务板块。".to_string(),
                    "这是一个额外的前台工具提示块。".to_string(),
                ],
            }),
            ..Default::default()
        };

        let runs = 20u32;
        let started = std::time::Instant::now();
        let mut latest_extra_len = 0usize;
        let mut history_len = 0usize;
        for _ in 0..runs {
            let prepared = build_prepared_prompt_for_mode(
                PromptBuildMode::Chat,
                &conversation,
                &agent,
                &[agent.clone(), user.clone()],
                &[],
                "用户",
                "我是性能探针里的用户。",
                DEFAULT_RESPONSE_STYLE_ID,
                "zh-CN",
                Some(&state.data_path),
                None,
                None,
                Some(overrides.clone()),
                Some(&state),
                Some(&ApiConfig::default()),
                None,
                Some(false),
            );
            latest_extra_len = prepared.latest_user_extra_text.len();
            history_len = prepared.history_messages.len();
            assert!(!prepared.preamble.trim().is_empty());
        }
        let total_ms = started.elapsed().as_millis() as u64;
        let avg_ms = total_ms / u64::from(runs);
        eprintln!(
            "[提示词性能探针] build_prepared_prompt_for_mode 平均耗时={}ms, total={}ms, runs={}, history_len={}, latest_extra_len={}",
            avg_ms,
            total_ms,
            runs,
            history_len,
            latest_extra_len
        );
    }

    #[test]
    fn normalize_image_for_chat_upload_should_resize_large_png_and_encode_webp() {
        let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            4000,
            2000,
            image::Rgb([12, 34, 56]),
        ));
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        source
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("encode png");

        let normalized =
            normalize_image_for_chat_upload(&cursor.into_inner()).expect("normalize image");

        assert_eq!(
            image::guess_format(&normalized.bytes).expect("guess format"),
            image::ImageFormat::WebP
        );

        let decoded = image::load_from_memory(&normalized.bytes).expect("decode webp");
        assert_eq!(decoded.width(), normalized.output_width);
        assert_eq!(decoded.height(), normalized.output_height);
        assert!(
            u64::from(decoded.width()) * u64::from(decoded.height())
                <= IMAGE_NORMALIZE_FOR_LLM_REQUEST_DEFAULT_PIXEL_BUDGET
        );
    }

    #[test]
    fn build_user_parts_should_downgrade_bad_image_to_text_notice() {
        let payload = ChatInputPayload {
            text: None,
            display_text: None,
            images: Some(vec![BinaryPart {
                mime: "image/png".to_string(),
                bytes_base64: "%%%bad-base64%%%".to_string(),
                saved_path: Some("downloads/bad-image.png".to_string()),
            }]),
            audios: None,
            attachments: None,
            model: None,
            extra_text_blocks: None,
            mentions: None,
            provider_meta: None,
        };

        let api = ApiConfig {
            enable_image: true,
            ..ApiConfig::default()
        };
        let parts = build_user_parts(&payload, &api).expect("build parts");

        assert_eq!(parts.len(), 1);
        match &parts[0] {
            MessagePart::Text { text, .. } => {
                assert!(text.contains("用户发送了一个附件，位于 {Self Directory}/"));
                assert!(text.contains("downloads/bad-image.png"));
            }
            other => panic!("expected text fallback, got {other:?}"),
        }
    }
