    #[test]
    fn remote_im_upsert_contact_for_inbound_should_create_with_communication_follow_channel_activation() {
        let channel = RemoteImChannelConfig {
            id: "c1".to_string(),
            name: "qq".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            enabled: true,
            credentials: serde_json::json!({}),
            activate_assistant: true,
            receive_files: true,
            streaming_send: false,
            show_tool_calls: false,
            allow_send_files: false,
        };
        let mut runtime = RuntimeStateFile::default();
        let input = RemoteImEnqueueInput {
            channel_id: "c1".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "qq".to_string(),
            remote_contact_type: "group".to_string(),
            remote_contact_id: "g1".to_string(),
            remote_contact_name: Some("测试群".to_string()),
            sender_id: "u1".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: None,
            platform_message_id: Some("m1".to_string()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            activate_assistant: Some(true),
            session: SessionSelector {
                api_config_id: None,
                department_id: None,
                agent_id: "agent".to_string(),
                conversation_id: Some("conv-1".to_string()),
            },
            payload: ChatInputPayload {
                text: Some("hello".to_string()),
                display_text: None,
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: None,
            },
        };
        let now = now_iso();
        let contact_id = remote_im_upsert_contact_for_inbound(&mut runtime, &channel, &input, &now);
        assert_eq!(runtime.remote_im_contacts.len(), 1);
        let contact = runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id == contact_id)
            .expect("contact exists");
        assert!(contact.allow_send);
        assert!(contact.allow_receive);
        assert_eq!(contact.activation_mode, "never");
        assert!(contact.activation_keywords.is_empty());
        assert_eq!(contact.activation_cooldown_seconds, 0);

        // 第二次入队应复用同一联系人
        let now2 = now_iso();
        let contact_id_2 = remote_im_upsert_contact_for_inbound(&mut runtime, &channel, &input, &now2);
        assert_eq!(contact_id, contact_id_2);
        assert_eq!(runtime.remote_im_contacts.len(), 1);
    }

    #[test]
    fn remote_im_resolve_inbound_activate_should_prefer_message_level_flag() {
        let channel = RemoteImChannelConfig {
            id: "c1".to_string(),
            name: "qq".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            enabled: true,
            credentials: serde_json::json!({}),
            activate_assistant: false,
            receive_files: true,
            streaming_send: false,
            show_tool_calls: false,
            allow_send_files: false,
        };
        assert!(!remote_im_resolve_inbound_activate(&channel, None));
        assert!(remote_im_resolve_inbound_activate(&channel, Some(true)));
        assert!(!remote_im_resolve_inbound_activate(&channel, Some(false)));
    }

    #[test]
    fn resolve_department_agent_pair_should_validate_explicit_pair_and_keep_legacy_department_fallback() {
        let mut api = ApiConfig::default();
        api.id = "api-a".to_string();
        api.enable_text = true;
        api.model = "gpt-4o-mini".to_string();
        let mut department = default_assistant_department(&api.id);
        department.id = "dept-a".to_string();
        department.is_built_in_assistant = false;
        department.agent_ids = vec!["agent-a".to_string()];
        let config = AppConfig {
            departments: vec![department],
            api_configs: vec![api],
            ..AppConfig::default()
        };

        let explicit = resolve_department_agent_pair(
            Some("dept-a"),
            Some("agent-a"),
            &config,
        )
        .expect("explicit pair");
        assert_eq!(explicit, ("dept-a".to_string(), "agent-a".to_string()));

        let legacy = resolve_department_agent_pair(Some("dept-a"), None, &config)
            .expect("legacy department-only binding should be solidified");
        assert_eq!(legacy, ("dept-a".to_string(), "agent-a".to_string()));

        let err = resolve_department_agent_pair(
            Some("dept-a"),
            Some("agent-b"),
            &config,
        )
        .expect_err("mismatched explicit pair should fail");
        assert!(err.contains("agentId 与部门不匹配"));
    }

    #[test]
    fn remote_im_filter_channel_logs_for_contact_should_only_keep_matching_contact() {
        let logs = vec![
            ChannelLogEntry {
                timestamp: chrono::Utc::now(),
                level: "info".to_string(),
                message: "[联系人消息] 收到: contact=甲[group:10001], preview=hello".to_string(),
            },
            ChannelLogEntry {
                timestamp: chrono::Utc::now(),
                level: "info".to_string(),
                message: "[联系人消息] 收到: contact=乙[group:10002], preview=world".to_string(),
            },
            ChannelLogEntry {
                timestamp: chrono::Utc::now(),
                level: "info".to_string(),
                message: "事件消费器已启动".to_string(),
            },
        ];

        let filtered = remote_im_filter_channel_logs_for_contact(logs, "[group:10001]");

        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].message.contains("[group:10001]"));
    }

    #[test]
    fn resolve_conversation_id_should_route_remote_im_to_contact_conversation() {
        let state = remote_im_test_state();
        let mut runtime = RuntimeStateFile::default();
        runtime.main_conversation_id = Some("conversation-main".to_string());
        let conversations = vec![
            Conversation {
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
                created_at: now_iso(),
                updated_at: now_iso(),
                last_user_at: None,
                last_assistant_at: None,
                status: "inactive".to_string(),
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
            auto_push_remote_contact_id: None,
            active_goal: None,
                cumulative_usage: ConversationCumulativeUsage::default(),
            },
            Conversation {
                id: "conversation-sub".to_string(),
                title: "sub".to_string(),
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
                created_at: now_iso(),
                updated_at: now_iso(),
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
            auto_push_remote_contact_id: None,
            active_goal: None,
                cumulative_usage: ConversationCumulativeUsage::default(),
            },
        ];
        let input = RemoteImEnqueueInput {
            channel_id: "c1".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "qq".to_string(),
            remote_contact_type: "group".to_string(),
            remote_contact_id: "g1".to_string(),
            remote_contact_name: Some("测试群".to_string()),
            sender_id: "u1".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: None,
            platform_message_id: Some("m1".to_string()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            activate_assistant: Some(true),
            session: SessionSelector {
                api_config_id: None,
                department_id: None,
                agent_id: DEFAULT_AGENT_ID.to_string(),
                conversation_id: Some("conversation-sub".to_string()),
            },
            payload: ChatInputPayload {
                text: Some("hello".to_string()),
                display_text: None,
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: None,
            },
        };

        let mut contact = RemoteImContact {
            id: "contact-1".to_string(),
            channel_id: input.channel_id.clone(),
            platform: input.platform,
            remote_contact_type: input.remote_contact_type.clone(),
            remote_contact_id: input.remote_contact_id.clone(),
            remote_contact_name: input.remote_contact_name.clone().unwrap_or_default(),
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
            bound_department_id: None,
            bound_agent_id: None,
            bound_conversation_id: None,
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        };
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");
        for conversation in &conversations {
            state_write_conversation_cached(&state, conversation).expect("write conversation");
        }

        let (_, _, conversation_id) =
            resolve_contact_session_target(&state, &mut runtime, &mut contact)
                .expect("resolve route");

        assert_ne!(conversation_id, "conversation-main");
        assert_eq!(contact.bound_conversation_id.as_deref(), Some(conversation_id.as_str()));
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn remote_im_should_still_route_to_contact_conversation_after_user_switches() {
        let state = remote_im_test_state();
        let mut runtime = RuntimeStateFile::default();
        runtime.main_conversation_id = Some("conversation-main".to_string());
        let conversations = vec![
            Conversation {
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
                created_at: now_iso(),
                updated_at: now_iso(),
                last_user_at: None,
                last_assistant_at: None,
                status: "inactive".to_string(),
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
            auto_push_remote_contact_id: None,
            active_goal: None,
                cumulative_usage: ConversationCumulativeUsage::default(),
            },
            Conversation {
                id: "conversation-sub".to_string(),
                title: "sub".to_string(),
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
                created_at: now_iso(),
                updated_at: now_iso(),
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
            auto_push_remote_contact_id: None,
            active_goal: None,
                cumulative_usage: ConversationCumulativeUsage::default(),
            },
        ];
        let input = RemoteImEnqueueInput {
            channel_id: "c1".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "qq".to_string(),
            remote_contact_type: "group".to_string(),
            remote_contact_id: "g1".to_string(),
            remote_contact_name: Some("测试群".to_string()),
            sender_id: "u1".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: None,
            platform_message_id: Some("m1".to_string()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            activate_assistant: Some(true),
            session: SessionSelector {
                api_config_id: None,
                department_id: None,
                agent_id: DEFAULT_AGENT_ID.to_string(),
                conversation_id: Some("conversation-sub".to_string()),
            },
            payload: ChatInputPayload {
                text: Some("hello".to_string()),
                display_text: None,
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: None,
            },
        };

        let mut contact = RemoteImContact {
            id: "contact-1".to_string(),
            channel_id: input.channel_id.clone(),
            platform: input.platform,
            remote_contact_type: input.remote_contact_type.clone(),
            remote_contact_id: input.remote_contact_id.clone(),
            remote_contact_name: input.remote_contact_name.clone().unwrap_or_default(),
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
            bound_department_id: None,
            bound_agent_id: None,
            bound_conversation_id: None,
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        };
        state_write_runtime_state_cached(&state, &runtime).expect("write runtime state");
        for conversation in &conversations {
            state_write_conversation_cached(&state, conversation).expect("write conversation");
        }

        let (_, _, conversation_id) =
            resolve_contact_session_target(&state, &mut runtime, &mut contact)
                .expect("resolve route");

        assert_ne!(conversation_id, "conversation-main");
        assert_eq!(contact.bound_conversation_id.as_deref(), Some(conversation_id.as_str()));
        assert_eq!(runtime.main_conversation_id.as_deref(), Some("conversation-main"));
        assert_eq!(
            conversations
                .iter()
                .find(|item| item.id == "conversation-sub")
                .map(|item| item.status.as_str()),
            Some("active")
        );
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn conversation_has_remote_im_platform_message_should_match_snake_case_origin_meta() {
        let conversation = Conversation {
            id: "conv-1".to_string(),
            title: "联系人".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now_iso(),
            updated_at: now_iso(),
            last_user_at: None,
            last_assistant_at: None,
            status: "inactive".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![ChatMessage {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                created_at: now_iso(),
                speaker_agent_id: None,
                parts: vec![MessagePart::Text {
                    text: "hello".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "origin": {
                        "kind": "remote_im",
                        "channel_id": "c1",
                        "contact_type": "private",
                        "contact_id": "u1",
                        "platform_message_id": "m1"
                    }
                })),
                tool_call: None,
                mcp_call: None,
            meme_annotations: None,
            }],
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        };

        assert!(conversation.messages.iter().any(|message| {
            message_has_remote_im_platform_message(message, "c1", "private", "u1", "m1")
        }));
        assert!(!conversation.messages.iter().any(|message| {
            message_has_remote_im_platform_message(message, "c1", "private", "u1", "m2")
        }));
    }

    #[test]
    fn conversation_has_remote_im_platform_message_should_ignore_legacy_origin_meta() {
        let conversation = Conversation {
            id: "conv-1".to_string(),
            title: "联系人".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now_iso(),
            updated_at: now_iso(),
            last_user_at: None,
            last_assistant_at: None,
            status: "inactive".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![ChatMessage {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                created_at: now_iso(),
                speaker_agent_id: None,
                parts: vec![MessagePart::Text {
                    text: "hello".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: Some(serde_json::json!({
                    "origin": {
                        "kind": "remote_im",
                        "channelId": "c1",
                        "remoteContactType": "private",
                        "remoteContactId": "u1",
                        "platformMessageId": "m1"
                    }
                })),
                tool_call: None,
                mcp_call: None,
            meme_annotations: None,
            }],
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        };

        assert!(!conversation.messages.iter().any(|message| {
            message_has_remote_im_platform_message(message, "c1", "private", "u1", "m1")
        }));
    }

    #[test]
    fn remote_im_set_sender_origin_meta_should_write_snake_case_remote_identity() {
        let input = RemoteImEnqueueInput {
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            im_name: "qq".to_string(),
            remote_contact_type: "private".to_string(),
            remote_contact_id: "remote-user-1".to_string(),
            remote_contact_name: Some("张三".to_string()),
            sender_id: "remote-user-1".to_string(),
            sender_name: "张三".to_string(),
            sender_avatar_url: Some("https://example.com/avatar.png".to_string()),
            platform_message_id: Some("msg-1".to_string()),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            activate_assistant: Some(true),
            session: SessionSelector {
                api_config_id: None,
                department_id: None,
                agent_id: String::new(),
                conversation_id: None,
            },
            payload: ChatInputPayload {
                text: Some("hello".to_string()),
                display_text: None,
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: None,
            },
        };

        let value = remote_im_set_sender_origin_meta(&input, "conversation-1", "record-1");
        let origin = value.get("origin").expect("origin");

        assert_eq!(origin.get("channel_id").and_then(Value::as_str), Some("channel-a"));
        assert_eq!(origin.get("contact_id").and_then(Value::as_str), Some("remote-user-1"));
        assert_eq!(origin.get("contact_record_id").and_then(Value::as_str), Some("record-1"));
        assert_eq!(origin.get("sender_name").and_then(Value::as_str), Some("张三"));
        assert_eq!(origin.get("platform_message_id").and_then(Value::as_str), Some("msg-1"));
        assert!(origin.get("channelId").is_none());
        assert!(origin.get("contactId").is_none());
    }

    #[test]
    fn weixin_oc_parse_media_aes_key_should_accept_base64_encoded_hex_text() {
        let encoded = B64.encode("00112233445566778899aabbccddeeff");
        let decoded = weixin_oc_parse_media_aes_key(&encoded).expect("decode aes key");
        assert_eq!(
            decoded,
            vec![
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb,
                0xcc, 0xdd, 0xee, 0xff,
            ]
        );
    }

    #[test]
    fn weixin_oc_decrypt_media_ecb_should_remove_pkcs7_padding() {
        use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

        let key = vec![
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff,
        ];
        let plain = b"wechat-image-bytes".to_vec();
        let pad_len = 16 - (plain.len() % 16);
        let mut padded = plain.clone();
        padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));

        let cipher = aes::Aes128::new_from_slice(&key).expect("create cipher");
        let mut encrypted = padded.clone();
        for chunk in encrypted.chunks_exact_mut(16) {
            let block = GenericArray::from_mut_slice(chunk);
            cipher.encrypt_block(block);
        }

        let decrypted = weixin_oc_decrypt_media_ecb(&encrypted, &key).expect("decrypt");
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn onebot_cq_string_should_extract_group_image_media_refs() {
        let (text, media_refs, embedded_refs) = parse_onebot_cq_string(
            "看看这个[CQ:image,file=https://example.com/a.png,file_id=img-1]图片",
        );
        assert_eq!(text, "看看这个图片");
        assert_eq!(media_refs.len(), 1);
        assert!(embedded_refs.is_empty());
        assert!(matches!(media_refs[0].kind, OnebotInboundMediaKind::Image));
        assert_eq!(media_refs[0].file_ref, "https://example.com/a.png");
        assert_eq!(media_refs[0].file_id.as_deref(), Some("img-1"));
    }

    #[test]
    fn extract_message_content_should_keep_media_when_message_is_cq_string() {
        let event = serde_json::json!({
            "message": "你好[CQ:image,file=base64://YWJj,file_id=image-2]"
        });
        let (text, media_refs, embedded_refs) = extract_message_content(&event);
        assert_eq!(text, "你好");
        assert_eq!(media_refs.len(), 1);
        assert!(embedded_refs.is_empty());
        assert!(matches!(media_refs[0].kind, OnebotInboundMediaKind::Image));
        assert_eq!(media_refs[0].file_ref, "base64://YWJj");
        assert_eq!(media_refs[0].file_id.as_deref(), Some("image-2"));
    }

    #[test]
    fn onebot_message_array_should_extract_forward_and_reply_refs() {
        let payload = serde_json::json!([
            { "type": "reply", "data": { "id": "123" } },
            { "type": "forward", "data": { "id": "456" } }
        ]);
        let (text, media_refs, embedded_refs) =
            parse_onebot_message_array(payload.as_array().expect("array"));
        assert!(text.is_empty());
        assert!(media_refs.is_empty());
        assert_eq!(embedded_refs.len(), 2);
        assert!(matches!(embedded_refs[0].kind, OnebotEmbeddedRefKind::Reply));
        assert_eq!(embedded_refs[0].id, "123");
        assert!(matches!(embedded_refs[1].kind, OnebotEmbeddedRefKind::Forward));
        assert_eq!(embedded_refs[1].id, "456");
    }

    #[test]
    fn onebot_forward_payload_should_prefer_sender_nickname_then_card_then_user_id() {
        let payload = serde_json::json!({
            "messages": [
                {
                    "sender": {
                        "nickname": "昵称甲",
                        "card": "群名片甲",
                        "user_id": "10001"
                    },
                    "message": [{ "type": "text", "data": { "text": "第一条" } }]
                },
                {
                    "sender": {
                        "card": "群名片乙",
                        "user_id": "10002"
                    },
                    "message": [{ "type": "text", "data": { "text": "第二条" } }]
                },
                {
                    "sender": {
                        "user_id": "10003"
                    },
                    "message": [{ "type": "text", "data": { "text": "第三条" } }]
                }
            ]
        });

        let (text, media_refs) = onebot_parse_forward_payload(&payload);

        assert!(media_refs.is_empty());
        assert_eq!(text, "昵称甲：第一条\n群名片乙：第二条\n10003：第三条");
    }

    #[test]
    fn onebot_forward_payload_should_fallback_to_node_name_when_sender_missing() {
        let payload = serde_json::json!({
            "messages": [
                {
                    "data": {
                        "name": "节点名称",
                        "content": [{ "type": "text", "data": { "text": "转发内容" } }]
                    }
                }
            ]
        });

        let (text, media_refs) = onebot_parse_forward_payload(&payload);

        assert!(media_refs.is_empty());
        assert_eq!(text, "节点名称：转发内容");
    }

    fn remote_im_test_state() -> AppState {
        let root = std::env::temp_dir().join(format!("eca-remote-im-test-{}", Uuid::new_v4()));
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
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            goal_continue_suppressed_conversation_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            task_scheduler_notify: Arc::new(tokio::sync::Notify::new()),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(std::collections::HashMap::new())),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(std::collections::HashSet::new())),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new("github".to_string())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn remote_im_test_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            width,
            height,
            image::Rgba([12, 34, 56, 255]),
        ));
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("encode png");
        cursor.into_inner()
    }

    #[tokio::test]
    async fn onebot_event_consumer_should_remain_singleton_per_channel() {
        let manager = OnebotV11WsManager::new();
        let state = remote_im_test_state();

        manager
            .start_event_consumer("channel-a".to_string(), state.clone())
            .await
            .expect("start consumer 1");
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(manager.event_consumer_tasks.read().await.len(), 1);
        assert_eq!(manager.event_consumer_stop_senders.read().await.len(), 1);

        manager
            .start_event_consumer("channel-a".to_string(), state)
            .await
            .expect("start consumer 2");
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(manager.event_consumer_tasks.read().await.len(), 1);
        assert_eq!(manager.event_consumer_stop_senders.read().await.len(), 1);

        manager
            .stop_channel("channel-a")
            .await
            .expect("stop channel");
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(manager.event_consumer_tasks.read().await.is_empty());
        assert!(manager.event_consumer_stop_senders.read().await.is_empty());
    }

    #[tokio::test]
    async fn onebot_channel_event_bus_should_exist_before_client_connection() {
        use futures_util::SinkExt as _;

        let manager = OnebotV11WsManager::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        manager
            .start(
                "channel-a".to_string(),
                OnebotV11WsCredentials {
                    ws_host: "127.0.0.1".to_string(),
                    ws_port: port,
                    ws_token: None,
                },
            )
            .await
            .expect("start onebot channel");

        let mut event_rx = manager
            .subscribe_events("channel-a")
            .await
            .expect("event bus should be available before client connects");

        let url = format!("ws://127.0.0.1:{port}");
        let (mut client, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .expect("connect client");
        let event = serde_json::json!({
            "post_type": "message",
            "message_type": "private",
            "user_id": 10001,
            "message_id": 42,
            "message": "hello"
        });
        client
            .send(tokio_tungstenite::tungstenite::Message::Text(
                event.to_string().into(),
            ))
            .await
            .expect("send event");

        let received = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("event should arrive")
            .expect("event bus open");
        assert_eq!(received.get("message_id").and_then(Value::as_i64), Some(42));

        manager
            .stop_channel("channel-a")
            .await
            .expect("stop onebot channel");
    }

    #[tokio::test]
    async fn onebot_channel_start_should_be_serialized_per_channel() {
        let manager = OnebotV11WsManager::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        let credentials = OnebotV11WsCredentials {
            ws_host: "127.0.0.1".to_string(),
            ws_port: port,
            ws_token: None,
        };
        let first = manager.start("channel-a".to_string(), credentials.clone());
        let second = manager.start("channel-a".to_string(), credentials);
        let (first_result, second_result) = tokio::join!(first, second);

        first_result.expect("first start should succeed");
        second_result.expect("second start should wait and succeed");
        assert_eq!(manager.channel_tasks.read().await.len(), 1);
        assert_eq!(manager.channel_runtimes.read().await.len(), 1);
        let logs = manager.get_logs("channel-a").await;
        assert_eq!(
            logs.iter()
                .filter(|entry| entry.message.contains("服务器启动，监听"))
                .count(),
            1
        );
        assert!(
            logs.iter()
                .any(|entry| entry.message.contains("跳过重复启动")),
            "second start should reuse the existing listener"
        );

        manager
            .stop_channel("channel-a")
            .await
            .expect("stop onebot channel");
    }

    #[tokio::test]
    async fn onebot_stop_channel_should_cancel_bind_retry_without_waiting_for_lifecycle_lock() {
        let manager = OnebotV11WsManager::new();
        let addr = "127.0.0.1:6199".parse().expect("valid onebot addr");
        let runtime = {
            let _guard = manager.port_service.lifecycle_guard("channel-a").await;
            manager.prepare_start_after_stop_at("channel-a", addr).await
        };
        manager
            .port_service
            .set_status_text("channel-a", Some("binding_retry".to_string()))
            .await;

        tokio::time::timeout(Duration::from_secs(1), manager.stop_channel("channel-a"))
            .await
            .expect("stop should not wait for the bind retry timeout")
            .expect("stop channel");

        assert!(runtime.cancel.is_cancelled());
        assert!(manager.channel_runtimes.read().await.is_empty());
    }

    #[tokio::test]
    async fn onebot_channel_should_replace_existing_connection_on_second_handshake() {
        let manager = OnebotV11WsManager::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        manager
            .start(
                "channel-a".to_string(),
                OnebotV11WsCredentials {
                    ws_host: "127.0.0.1".to_string(),
                    ws_port: port,
                    ws_token: None,
                },
            )
            .await
            .expect("start onebot channel");

        let url = format!("ws://127.0.0.1:{port}");
        let (mut first, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .expect("first connection");
        tokio::time::sleep(Duration::from_millis(80)).await;

        let (mut second, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .expect("second connection");
        tokio::time::sleep(Duration::from_millis(120)).await;

        let status = manager.get_connection_status("channel-a").await;
        assert!(status.connected);
        let logs = manager.get_logs("channel-a").await;
        assert!(
            logs.iter()
                .any(|entry| entry.message.contains("新连接已接管旧连接")),
            "second connection should replace the old one"
        );
        assert_eq!(manager.connections.read().await.len(), 1);
        assert_eq!(manager.connection_stop_senders.read().await.len(), 1);
        assert_eq!(manager.channel_runtimes.read().await.len(), 1);

        let _ = first.close(None).await;
        let _ = second.close(None).await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        manager
            .stop_channel("channel-a")
            .await
            .expect("stop onebot channel");
        assert!(manager.channel_runtimes.read().await.is_empty());
    }

    #[tokio::test]
    async fn onebot_stop_channel_should_cancel_active_connection_task() {
        let manager = OnebotV11WsManager::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        manager
            .start(
                "channel-a".to_string(),
                OnebotV11WsCredentials {
                    ws_host: "127.0.0.1".to_string(),
                    ws_port: port,
                    ws_token: None,
                },
            )
            .await
            .expect("start onebot channel");

        let url = format!("ws://127.0.0.1:{port}");
        let (_client, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .expect("connect client");
        tokio::time::sleep(Duration::from_millis(80)).await;

        assert_eq!(manager.connections.read().await.len(), 1);
        assert_eq!(manager.channel_runtimes.read().await.len(), 1);

        manager
            .stop_channel("channel-a")
            .await
            .expect("stop onebot channel");
        tokio::time::sleep(Duration::from_millis(80)).await;

        assert!(manager.connections.read().await.is_empty());
        assert!(manager.connection_stop_senders.read().await.is_empty());
        assert!(manager.channel_runtimes.read().await.is_empty());
    }

    fn remote_im_test_contact(contact_id: &str, conversation_id: &str) -> RemoteImContact {
        RemoteImContact {
            id: contact_id.to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "private".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "张三".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: true,
            allow_send_files: false,
            allow_receive: true,
            activation_mode: "keyword".to_string(),
            activation_keywords: vec!["派".to_string()],
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            route_mode: "dedicated_contact_conversation".to_string(),
            bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
            bound_agent_id: None,
            bound_conversation_id: Some(conversation_id.to_string()),
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        }
    }

    #[test]
    fn dingtalk_push_normalized_image_or_attachment_should_fallback_to_attachment_on_oversized_image() {
        let state = remote_im_test_state();
        let raw = remote_im_test_png(10_001, 8);
        let mut images = Vec::<BinaryPart>::new();
        let mut attachments = Vec::<AttachmentMetaInput>::new();

        let notice = dingtalk_push_normalized_image_or_attachment(
            &state,
            &raw,
            "image/png",
            "dingtalk-oversized.png",
            &mut images,
            &mut attachments,
            "",
        );

        assert!(images.is_empty());
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].mime, "image/png");
        assert!(notice.unwrap_or_default().contains("用户发送了一个附件，位于 {Assistant Space}/"));
        assert!(
            state
                .llm_workspace_path
                .join(&attachments[0].relative_path)
                .exists()
        );
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn weixin_oc_push_normalized_image_and_attachment_should_fallback_to_attachment_on_oversized_image() {
        let state = remote_im_test_state();
        let raw = remote_im_test_png(10_001, 8);
        let mut images = Vec::<BinaryPart>::new();
        let mut attachments = Vec::<AttachmentMetaInput>::new();

        let notice = weixin_oc_push_normalized_image_and_attachment(
            &state,
            "weixin-oversized.png",
            &raw,
            "image/png",
            &mut images,
            &mut attachments,
        );

        assert!(images.is_empty());
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].mime, "image/png");
        assert!(notice.unwrap_or_default().contains("用户发送了一个附件，位于 {Assistant Space}/"));
        assert!(
            state
                .llm_workspace_path
                .join(&attachments[0].relative_path)
                .exists()
        );
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    fn remote_im_test_secretary_assistant_context() -> RemoteImConversationAssistantContext {
        RemoteImConversationAssistantContext {
            department_id: "dept-sales".to_string(),
            department_name: "售前部门".to_string(),
            agent_id: "agent-sales".to_string(),
            agent_name: "售前助理".to_string(),
        }
    }

    fn remote_im_test_agent(agent_id: &str, agent_name: &str) -> AgentProfile {
        AgentProfile {
            id: agent_id.to_string(),
            name: agent_name.to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now_iso(),
            updated_at: now_iso(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "manual".to_string(),
            scope: "global".to_string(),
        }
    }

    #[test]
    fn remote_im_secretary_current_assistant_context_should_read_from_runtime_slot() {
        let state = remote_im_test_state();
        let assistant = remote_im_test_secretary_assistant_context();

        set_conversation_remote_im_assistant_context(
            &state,
            "conversation-a",
            Some(assistant.clone()),
        )
        .expect("set runtime assistant");

        let resolved = remote_im_secretary_current_assistant_context(&state, "conversation-a")
            .expect("resolve runtime assistant");

        assert_eq!(resolved.department_id, assistant.department_id);
        assert_eq!(resolved.agent_id, assistant.agent_id);
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn remote_im_resolve_contact_assistant_context_should_require_bound_department() {
        let state = remote_im_test_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        state_write_agents_cached(
            &state,
            &[remote_im_test_agent(DEFAULT_AGENT_ID, "主助理"), default_user_persona()],
        )
        .expect("write agents");
        let mut contact = remote_im_test_contact("contact-a", "conversation-a");
        contact.bound_department_id = None;

        let err = remote_im_resolve_contact_assistant_context(&state, &contact)
            .expect_err("missing department should fail");

        assert!(err.contains("未设置应答部门"));
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn remote_im_resolve_contact_assistant_context_should_resolve_department_and_agent_names() {
        let state = remote_im_test_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        state_write_agents_cached(
            &state,
            &[remote_im_test_agent(DEFAULT_AGENT_ID, "主助理"), default_user_persona()],
        )
        .expect("write agents");
        let contact = remote_im_test_contact("contact-a", "conversation-a");

        let resolved = remote_im_resolve_contact_assistant_context(&state, &contact)
            .expect("resolve assistant context");

        assert_eq!(resolved.department_id, REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID);
        assert_eq!(resolved.department_name, "远程客服");
        assert_eq!(resolved.agent_id, DEFAULT_AGENT_ID);
        assert_eq!(resolved.agent_name, "主助理");
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn remote_im_resolve_contact_assistant_context_should_reject_missing_agent_profile() {
        let state = remote_im_test_state();
        write_config(&state.config_path, &AppConfig::default()).expect("write config");
        state_write_agents_cached(
            &state,
            &[remote_im_test_agent("agent-other", "其他助理"), default_user_persona()],
        )
        .expect("write agents");
        let contact = remote_im_test_contact("contact-a", "conversation-a");

        let err = remote_im_resolve_contact_assistant_context(&state, &contact)
            .expect_err("missing agent profile should fail");

        assert!(err.contains("路由人格不存在"));
        assert!(err.contains(DEFAULT_AGENT_ID));
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn ensure_remote_im_contact_conversation_id_should_accept_private_bound_department() {
        let state = remote_im_test_state();
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
  "name": "私域客服",
  "agentIds": ["private-agent"]
}"#,
        )
        .expect("write private department");
        state_write_agents_cached(
            &state,
            &[
                remote_im_test_agent("private-agent", "私域助理"),
                default_user_persona(),
            ],
        )
        .expect("write agents");

        let mut contact = remote_im_test_contact("contact-private", "");
        contact.bound_department_id = Some("dept-private".to_string());
        contact.bound_conversation_id = None;

        let conversation_id = ensure_remote_im_contact_conversation_id(&state, &mut contact)
            .expect("ensure contact conversation");
        let conversation =
            state_read_conversation_cached(&state, &conversation_id).expect("read conversation");

        assert_eq!(conversation.department_id, "dept-private");
        assert_eq!(conversation.agent_id, "private-agent");
        let _ = std::fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn remote_im_collect_secretary_recent_messages_should_keep_last_seven_and_truncate_each_item() {
        let mut contact = remote_im_test_contact("contact-a", "conversation-a");
        contact.remote_contact_type = "private".to_string();
        contact.remote_contact_id = "contact-42".to_string();
        contact.remote_contact_name = "陈先生".to_string();
        let agents = vec![AgentProfile {
            id: "agent-sales".to_string(),
            name: "售前助理".to_string(),
            system_prompt: String::new(),
            tools: Vec::new(),
            created_at: now_iso(),
            updated_at: now_iso(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: "manual".to_string(),
            scope: "global".to_string(),
        }];
        let current_assistant = remote_im_test_secretary_assistant_context();
        let mut messages = Vec::<ChatMessage>::new();
        for idx in 0..8 {
            messages.push(ChatMessage {
                id: format!("msg-{idx}"),
                role: if idx % 2 == 0 { "user".to_string() } else { "assistant".to_string() },
                created_at: now_iso(),
                speaker_agent_id: if idx % 2 == 0 {
                    None
                } else {
                    Some("agent-sales".to_string())
                },
                parts: vec![MessagePart::Text {
                    text: format!("第{}条{}", idx, "很长的内容".repeat(30)),
                    reasoning_content: None,
                }],
                extra_text_blocks: Vec::new(),
                provider_meta: if idx % 2 == 0 {
                    Some(serde_json::json!({
                        "origin": {
                            "kind": "remote_im",
                            "contact_type": "private",
                            "contact_id": "contact-42",
                            "contact_name": "陈先生",
                            "sender_id": "contact-42",
                            "sender_name": "陈先生"
                        }
                    }))
                } else {
                    None
                },
                tool_call: None,
                mcp_call: None,
            meme_annotations: None,
            });
        }

        let digests = remote_im_collect_secretary_recent_messages(
            &messages,
            7,
            &contact,
            &agents,
            &current_assistant,
        );

        assert_eq!(digests.len(), 7);
        assert_eq!(
            digests.first().map(|item| item.speaker.as_str()),
            Some("你 售前助理(agent-sales)")
        );
        assert!(digests.iter().all(|item| item.text.chars().count() <= 100));
    }

    #[test]
    fn remote_im_secretary_message_digest_should_include_group_member_identity_and_media_placeholder() {
        let mut contact = remote_im_test_contact("contact-a", "conversation-a");
        contact.remote_contact_type = "group".to_string();
        contact.remote_contact_id = "group-88".to_string();
        contact.remote_contact_name = "项目群".to_string();
        let current_assistant = remote_im_test_secretary_assistant_context();
        let digest = remote_im_secretary_message_digest(&ChatMessage {
            id: "msg-image".to_string(),
            role: "user".to_string(),
            created_at: now_iso(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Image {
                mime: "image/png".to_string(),
                bytes_base64: "abc".to_string(),
                name: None,
                compressed: false,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "origin": {
                    "kind": "remote_im",
                    "contact_type": "group",
                    "contact_id": "group-88",
                    "contact_name": "项目群",
                    "sender_id": "user-7",
                    "sender_name": "张三"
                }
            })),
            tool_call: None,
            mcp_call: None,
        meme_annotations: None,
        }, &contact, &Vec::new(), &current_assistant)
        .expect("digest");

        assert_eq!(digest.speaker, "群友 张三(user-7)");
        assert_eq!(digest.text, "[图片]");
    }

    #[test]
    fn build_remote_im_secretary_prepared_prompt_should_include_boundary_and_latest_marker() {
        let mut contact = remote_im_test_contact("contact-a", "conversation-a");
        contact.remote_contact_type = "group".to_string();
        contact.remote_contact_id = "group-88".to_string();
        contact.remote_contact_name = "项目群".to_string();
        let current_assistant = remote_im_test_secretary_assistant_context();
        let history_messages = vec![
            RemoteImSecretaryMessageDigest {
                speaker: "群友 张三(user-7)".to_string(),
                text: "这个报价我先看一下".to_string(),
            },
            RemoteImSecretaryMessageDigest {
                speaker: "你 售前助理(agent-sales)".to_string(),
                text: "好的，有问题随时提".to_string(),
            },
        ];
        let new_batch_messages = vec![
            RemoteImSecretaryMessageDigest {
                speaker: "群友 李四(user-8)".to_string(),
                text: "交期今天能不能定".to_string(),
            },
            RemoteImSecretaryMessageDigest {
                speaker: "群友 张三(user-7)".to_string(),
                text: "老板现在就等结论".to_string(),
            },
        ];

        let prompt = build_remote_im_secretary_prepared_prompt(
            "简体中文",
            &contact,
            &current_assistant,
            &history_messages,
            &new_batch_messages,
        );

        assert!(prompt.latest_user_text.contains("当前应答部门："));
        assert!(prompt.latest_user_text.contains("名称：售前部门"));
        assert!(prompt.latest_user_text.contains("ID：dept-sales"));
        assert!(prompt.latest_user_text.contains("当前助理："));
        assert!(prompt.latest_user_text.contains("名称：售前助理"));
        assert!(prompt.latest_user_text.contains("ID：agent-sales"));
        assert!(prompt.latest_user_text.contains("当前联系人："));
        assert!(prompt.latest_user_text.contains("名称：项目群"));
        assert!(prompt.latest_user_text.contains("ID：group-88"));
        assert!(!prompt.latest_user_text.contains("当前人格："));
        assert!(prompt.latest_user_text.contains("最近 7 条已处理历史消息"));
        assert!(prompt
            .latest_user_text
            .contains("================ 未处理边界 ================"));
        assert!(prompt.latest_user_text.contains("最后一条是最新消息"));
        assert!(prompt
            .latest_user_text
            .contains("2. 群友 张三(user-7)（最新）：老板现在就等结论"));
    }


    #[test]
    fn remote_im_prepare_enqueue_runtime_state_should_mute_and_block_when_mute_keyword_matched() {
        let state = remote_im_test_state();
        let mut contact = remote_im_test_contact("contact-a", "conversation-a");
        contact.mute_keywords = vec!["闭嘴".to_string()];
        contact.unmute_keywords = vec!["张嘴".to_string()];
        contact.mute_duration_seconds = 600;

        let (activate_assistant, reason) =
            remote_im_prepare_enqueue_runtime_state(&state, &contact, "现在闭嘴")
                .expect("prepare runtime state");

        assert!(!activate_assistant);
        assert!(reason.contains("闭嘴词"));
        let runtime_states =
            lock_remote_im_contact_runtime_states(&state).expect("lock runtime states");
        let runtime = runtime_states.get("contact-a").expect("runtime exists");
        assert!(runtime.mute_until.is_some());
        assert_eq!(runtime.presence_state, RemoteImPresenceState::Away);
    }
