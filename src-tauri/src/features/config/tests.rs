    #[test]
    fn image_text_cache_upsert_and_find_should_work() {
        let state = storage_and_stt_test_state();
        state_service_upsert_image_text_cache(&state, "h1", "vision-a", "image", "", "text-a")
            .expect("upsert cache");
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-a", "image", "")
                .expect("find cache"),
            Some("text-a".to_string())
        );

        state_service_upsert_image_text_cache(&state, "h1", "vision-a", "image", "", "text-b")
            .expect("upsert cache");
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-a", "image", "")
                .expect("find cache"),
            Some("text-b".to_string())
        );
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-b", "image", "")
                .expect("find cache"),
            None
        );
    }

    #[test]
    fn image_text_cache_should_isolate_entries_by_image_type() {
        let state = storage_and_stt_test_state();
        state_service_upsert_image_text_cache(&state, "h1", "vision-a", "image", "", "text-image")
            .expect("upsert image type cache");
        state_service_upsert_image_text_cache(&state, "h1", "vision-a", "chart", "", "text-chart")
            .expect("upsert chart type cache");
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-a", "image", "")
                .expect("find image type cache"),
            Some("text-image".to_string())
        );
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-a", "chart", "")
                .expect("find chart type cache"),
            Some("text-chart".to_string())
        );
        assert_eq!(
            state_service_find_image_text_cache(&state, "h1", "vision-a", "screenshot", "")
                .expect("find other type cache"),
            None
        );
    }

    #[test]
    fn compute_image_hash_hex_should_be_stable() {
        let png_1x1_red = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO9WfXkAAAAASUVORK5CYII=";
        let part = BinaryPart {
            mime: "image/png".to_string(),
            bytes_base64: png_1x1_red.to_string(),
            saved_path: None,
        };
        let h1 = compute_image_hash_hex(&part).expect("hash1");
        let h2 = compute_image_hash_hex(&part).expect("hash2");
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
    }

    #[test]
    fn startup_window_label_should_open_main_without_usable_text_llm() {
        let mut cfg = AppConfig::default();
        normalize_app_config(&mut cfg);
        assert_eq!(startup_window_label_for_config(&cfg), "main");

        let api_id = cfg.assistant_department_api_config_id.clone();
        let api = cfg
            .api_configs
            .iter_mut()
            .find(|item| item.id == api_id)
            .expect("default chat api exists");
        api.base_url = "https://api.deepseek.com/v1".to_string();
        api.model = "deepseek-chat".to_string();
        api.api_key = "sk-test".to_string();
        assert_eq!(startup_window_label_for_config(&cfg), "chat");
    }

    #[test]
    fn normalize_app_config_should_not_promote_legacy_video_capability_to_audio() {
        let mut cfg = AppConfig::default();
        let provider = cfg
            .api_providers
            .first_mut()
            .expect("default provider exists");
        provider.enable_audio = true;
        let model = provider.models.first_mut().expect("default model exists");
        model.enable_audio = false;
        model.enable_video = true;

        normalize_app_config(&mut cfg);

        let provider = cfg
            .api_providers
            .first()
            .expect("default provider exists after normalization");
        assert!(!provider.models[0].enable_audio);
        assert!(!provider.enable_audio);
        assert!(provider.models[0].enable_video);
    }

    #[test]
    fn legacy_api_config_video_capability_should_not_migrate_to_audio() {
        let mut cfg = AppConfig::default();
        cfg.api_providers.clear();
        let api = cfg.api_configs.first_mut().expect("default API config exists");
        api.enable_audio = true;
        api.enable_video = true;

        normalize_app_config(&mut cfg);

        let provider = cfg
            .api_providers
            .first()
            .expect("legacy API config should migrate to provider");
        assert!(!provider.models[0].enable_audio);
        assert!(provider.models[0].enable_video);
    }

    #[test]
    fn startup_window_label_should_allow_codex_local_auth_without_api_key() {
        let mut cfg = AppConfig::default();
        let api_id = cfg.assistant_department_api_config_id.clone();
        let api = cfg
            .api_configs
            .iter_mut()
            .find(|item| item.id == api_id)
            .expect("default chat api exists");
        api.request_format = RequestFormat::Codex;
        api.base_url = DEFAULT_CODEX_BASE_URL.to_string();
        api.model = "gpt-5.4".to_string();
        api.api_key.clear();
        api.codex_auth_mode = CODEX_AUTH_MODE_READ_LOCAL.to_string();
        normalize_app_config(&mut cfg);
        assert_eq!(startup_window_label_for_config(&cfg), "chat");
    }

    #[test]
    fn startup_window_label_should_require_assistant_department_binding() {
        let mut cfg = AppConfig::default();
        let api_id = cfg.assistant_department_api_config_id.clone();
        let api = cfg
            .api_configs
            .iter_mut()
            .find(|item| item.id == api_id)
            .expect("default chat api exists");
        api.api_key = "sk-test".to_string();
        for department in &mut cfg.departments {
            if department.id == ASSISTANT_DEPARTMENT_ID {
                department.api_config_id.clear();
                department.api_config_ids.clear();
            }
        }
        cfg.assistant_department_api_config_id.clear();
        assert_eq!(startup_window_label_for_config(&cfg), "main");
    }

    #[test]
    fn normalize_app_config_should_fix_invalid_record_and_stt_fields() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            code_font: default_code_font(),
            ui_size_scale: default_ui_size_scale(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            skipped_github_update_version: String::new(),
            record_hotkey: "".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 0,
            max_record_seconds: 0,
            tool_max_iterations: 0,
            llm_round_log_capacity: 9,
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            desktop_operation_notice_enabled: default_desktop_operation_notice_enabled(),
            desktop_operate_enabled: default_desktop_operate_enabled(),
            selected_api_config_id: "a1".to_string(),
            assistant_department_api_config_id: "a1".to_string(),
            simple_setup_mode: false,
            vision_api_config_id: None,
            image_generation_model_id: None,
            image_providers: Vec::new(),
            stt_api_config_id: None,
            stt_auto_send: false,
            provider_non_stream_base_urls: Vec::new(),
            terminal_shell_kind: default_terminal_shell_kind(),
            shell_workspaces: Vec::new(),
            mcp_servers: Vec::new(),
            remote_im_channels: Vec::new(),
            departments: Vec::new(),
            api_configs: vec![
                ApiConfig {
                    id: "a1".to_string(),
                    name: "chat".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: true,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "m".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 999,
                },
                ApiConfig {
                    id: "a2".to_string(),
                    name: "bad-stt".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: true,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "m".to_string(),
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
            tool_review_api_config_id: None,
        };
        normalize_app_config(&mut cfg);
        assert_eq!(cfg.record_hotkey, "");
        assert_eq!(cfg.min_record_seconds, 1);
        assert!(cfg.max_record_seconds >= cfg.min_record_seconds);
        assert_eq!(cfg.tool_max_iterations, 1);
        assert_eq!(cfg.llm_round_log_capacity, 3);
        assert_eq!(cfg.api_configs[0].failure_retry_count, 20);
        assert!(!cfg.stt_auto_send);
    }

    #[test]
    fn normalize_app_config_should_not_bind_chat_api_to_selected_api() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            code_font: default_code_font(),
            ui_size_scale: default_ui_size_scale(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            skipped_github_update_version: String::new(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            desktop_operation_notice_enabled: default_desktop_operation_notice_enabled(),
            desktop_operate_enabled: default_desktop_operate_enabled(),
            selected_api_config_id: "edit-b".to_string(),
            assistant_department_api_config_id: "chat-a".to_string(),
            vision_api_config_id: None,
            image_generation_model_id: None,
            image_providers: Vec::new(),
            stt_api_config_id: None,
            simple_setup_mode: false,
            stt_auto_send: false,
            provider_non_stream_base_urls: Vec::new(),
            terminal_shell_kind: default_terminal_shell_kind(),
            shell_workspaces: Vec::new(),
            mcp_servers: Vec::new(),
            remote_im_channels: Vec::new(),
            departments: Vec::new(),
            api_configs: vec![
                ApiConfig {
                    id: "chat-a".to_string(),
                    name: "chat-a".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: true,
                    enable_audio: true,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "m".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
                ApiConfig {
                    id: "edit-b".to_string(),
                    name: "edit-b".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "m".to_string(),
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
            tool_review_api_config_id: None,
        };
        normalize_app_config(&mut cfg);
        assert_eq!(cfg.selected_api_config_id, "edit-b::edit-b-model-default".to_string());
        assert_eq!(
            cfg.assistant_department_api_config_id,
            "chat-a::chat-a-model-default".to_string()
        );
    }

    #[test]
    fn normalize_app_config_should_preserve_shared_child_departments_and_keep_unresolved_refs() {
        let mut cfg = AppConfig::default();
        let mut primary = default_assistant_department("");
        primary.id = "department-primary".to_string();
        primary.name = "主部门".to_string();
        primary.is_built_in_assistant = false;
        primary.agent_ids = vec!["agent-a".to_string()];
        primary.child_department_ids = vec![
            "department-shared".to_string(),
            "department-primary".to_string(),
            "missing-department".to_string(),
        ];

        let mut parent_b = default_assistant_department("");
        parent_b.id = "department-parent-b".to_string();
        parent_b.name = "项目二".to_string();
        parent_b.is_built_in_assistant = false;
        parent_b.agent_ids = vec!["agent-b".to_string()];
        parent_b.child_department_ids = vec!["department-shared".to_string()];

        let mut shared = default_assistant_department("");
        shared.id = "department-shared".to_string();
        shared.name = "共享施工队".to_string();
        shared.is_built_in_assistant = false;
        shared.agent_ids = vec!["agent-c".to_string()];

        cfg.departments = vec![primary, parent_b, shared];

        normalize_app_config(&mut cfg);

        let primary = cfg
            .departments
            .iter()
            .find(|item| item.id == "department-primary")
            .expect("primary department");
        assert_eq!(
            primary.child_department_ids,
            vec![
                "department-shared".to_string(),
                "missing-department".to_string()
            ]
        );

        let parent_b = cfg
            .departments
            .iter()
            .find(|item| item.id == "department-parent-b")
            .expect("department-parent-b");
        assert_eq!(
            parent_b.child_department_ids,
            vec!["department-shared".to_string()]
        );
    }

    #[test]
    fn runtime_organization_snapshot_should_filter_missing_children_after_private_merge() {
        let root = std::env::temp_dir().join(format!("eca-runtime-org-{}", Uuid::new_v4()));
        let data_path = root.join("config").join("app_data.json");
        let departments_dir = root
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&departments_dir).expect("create private departments dir");
        std::fs::write(
            departments_dir.join("department-private.json"),
            r#"{
  "id": "department-private",
  "name": "私域部门",
  "agentIds": ["private-agent"]
}"#,
        )
        .expect("write private department");

        let mut cfg = AppConfig::default();
        let mut primary = default_assistant_department(&cfg.assistant_department_api_config_id);
        primary.id = "department-primary".to_string();
        primary.name = "主部门".to_string();
        primary.is_built_in_assistant = false;
        primary.agent_ids = vec!["parent-agent".to_string()];
        primary.child_department_ids = vec![
            "department-private".to_string(),
            "missing-department".to_string(),
            "department-primary".to_string(),
        ];
        cfg.departments.push(primary);

        let mut parent_agent = default_agent();
        parent_agent.id = "parent-agent".to_string();
        parent_agent.name = "主部门人格".to_string();
        let mut private_agent = default_agent();
        private_agent.id = "private-agent".to_string();
        private_agent.name = "私域部门人格".to_string();

        let snapshot = build_runtime_organization_snapshot_from_parts(
            &data_path,
            &cfg,
            &[parent_agent, private_agent, default_user_persona()],
        )
        .expect("build runtime organization snapshot");
        let primary = runtime_department_by_id(&snapshot, "department-primary")
            .expect("runtime primary department");

        assert_eq!(
            primary.child_department_ids,
            vec!["department-private".to_string()]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_self_check_should_be_noop_after_deputy_semantics_removed() {
        let mut cfg = AppConfig::default();
        let snapshot = serde_json::to_string(&cfg.departments).expect("departments snapshot");
        assert!(!run_startup_self_checks(&mut cfg));
        assert_eq!(
            snapshot,
            serde_json::to_string(&cfg.departments).expect("departments snapshot after self check")
        );
    }

    #[test]
    fn normalize_app_config_should_restore_missing_deputy_without_rewriting_assistant_children() {
        let mut cfg = AppConfig::default();
        cfg.departments
            .retain(|item| item.id != DEPUTY_DEPARTMENT_ID);
        if let Some(assistant) = cfg
            .departments
            .iter_mut()
            .find(|item| item.id == ASSISTANT_DEPARTMENT_ID || item.is_built_in_assistant)
        {
            assistant.child_department_ids.clear();
        }

        normalize_app_config(&mut cfg);

        let deputy = cfg
            .departments
            .iter()
            .find(|item| item.id == DEPUTY_DEPARTMENT_ID)
            .expect("deputy department");
        assert!(!deputy.is_deputy);
        assert_eq!(deputy.name, "explorer");
        assert!(deputy.summary.contains("大范围摸底"));
        assert_eq!(deputy.agent_ids, vec![DEPUTY_AGENT_ID.to_string()]);

        let assistant = cfg
            .departments
            .iter()
            .find(|item| item.id == ASSISTANT_DEPARTMENT_ID || item.is_built_in_assistant)
            .expect("assistant department");
        assert!(assistant.child_department_ids.is_empty());
    }

    #[test]
    fn normalize_app_config_should_preserve_preset_department_customizations_and_multi_parent_tree() {
        let mut cfg = AppConfig::default();
        for department in &mut cfg.departments {
            if department.id == ASSISTANT_DEPARTMENT_ID || department.is_built_in_assistant {
                department.child_department_ids.clear();
            }
            if department.id == DEPUTY_DEPARTMENT_ID {
                department.name = "自定义探索".to_string();
                department.summary = "自定义概述".to_string();
                department.guide = "自定义指南".to_string();
                department.api_config_ids = vec![MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string()];
                department.api_config_id = MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string();
                department.model_failure_fallback_enabled = true;
                department.permission_control = DepartmentPermissionControl::default();
                department.child_department_ids = vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()];
            }
            if department.id == REVIEWER_DEPARTMENT_ID {
                department.name = "自定义审查".to_string();
                department.summary = "自定义概述".to_string();
                department.guide = "自定义指南".to_string();
                department.api_config_ids = vec![MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string()];
                department.api_config_id = MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string();
                department.model_failure_fallback_enabled = true;
                department.permission_control = department_whitelist_permission_control(&["read"], &[]);
                department.child_department_ids = vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()];
            }
            if department.id == SADDLER_DEPARTMENT_ID {
                department.name = "自定义能力资产".to_string();
                department.summary = "自定义概述".to_string();
                department.guide = "自定义指南".to_string();
                department.api_config_ids = vec![MODEL_ROLE_QUICK_API_CONFIG_ID.to_string()];
                department.api_config_id = MODEL_ROLE_QUICK_API_CONFIG_ID.to_string();
                department.permission_control = DepartmentPermissionControl::default();
                department.child_department_ids = vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()];
            }
        }

        let mut parent = default_assistant_department(MODEL_ROLE_EXPERT_API_CONFIG_ID);
        parent.id = "department-other".to_string();
        parent.is_built_in_assistant = false;
        parent.child_department_ids = vec![
            DEPUTY_DEPARTMENT_ID.to_string(),
            REVIEWER_DEPARTMENT_ID.to_string(),
            SADDLER_DEPARTMENT_ID.to_string(),
        ];
        cfg.departments.push(parent);

        normalize_app_config(&mut cfg);

        let assistant = cfg
            .departments
            .iter()
            .find(|item| item.id == ASSISTANT_DEPARTMENT_ID)
            .expect("assistant department");
        assert!(assistant.child_department_ids.is_empty());

        let explorer = cfg
            .departments
            .iter()
            .find(|item| item.id == DEPUTY_DEPARTMENT_ID)
            .expect("explorer department");
        assert_eq!(explorer.name, "自定义探索");
        assert_eq!(explorer.summary, "自定义概述");
        assert_eq!(explorer.guide, "自定义指南");
        assert_eq!(explorer.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert!(explorer.model_failure_fallback_enabled);
        assert_eq!(
            explorer.child_department_ids,
            vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()]
        );
        assert_eq!(explorer.permission_control, DepartmentPermissionControl::default());

        let reviewer = cfg
            .departments
            .iter()
            .find(|item| item.id == REVIEWER_DEPARTMENT_ID)
            .expect("reviewer department");
        assert_eq!(reviewer.name, "自定义审查");
        assert_eq!(reviewer.summary, "自定义概述");
        assert_eq!(reviewer.guide, "自定义指南");
        assert_eq!(reviewer.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert!(reviewer.model_failure_fallback_enabled);
        assert_eq!(
            reviewer.child_department_ids,
            vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()]
        );
        assert_eq!(
            reviewer.permission_control,
            department_whitelist_permission_control(&["read"], &[])
        );
        assert!(department_permission_allows_any_name(
            Some(reviewer),
            DepartmentPermissionCategory::BuiltinTool,
            &["read"],
        ));
        assert!(!department_permission_allows_any_name(
            Some(reviewer),
            DepartmentPermissionCategory::BuiltinTool,
            &["exec"],
        ));

        let saddler = cfg
            .departments
            .iter()
            .find(|item| item.id == SADDLER_DEPARTMENT_ID)
            .expect("saddler department");
        assert_eq!(saddler.name, "自定义能力资产");
        assert_eq!(saddler.summary, "自定义概述");
        assert_eq!(saddler.guide, "自定义指南");
        assert_eq!(saddler.api_config_id, MODEL_ROLE_QUICK_API_CONFIG_ID);
        assert_eq!(
            saddler.child_department_ids,
            vec![REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()]
        );
        assert_eq!(saddler.permission_control, DepartmentPermissionControl::default());

        let other = cfg
            .departments
            .iter()
            .find(|item| item.id == "department-other")
            .expect("other department");
        assert_eq!(
            other.child_department_ids,
            vec![
                DEPUTY_DEPARTMENT_ID.to_string(),
                REVIEWER_DEPARTMENT_ID.to_string(),
                SADDLER_DEPARTMENT_ID.to_string(),
            ]
        );
    }

    #[test]
    fn default_department_draft_should_return_backend_preset() {
        let leader = default_department_draft(LEADER_DEPARTMENT_ID, "zh-CN")
            .expect("leader default draft");
        assert_eq!(leader.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert_eq!(
            leader.permission_control,
            leader_department_permission_control()
        );

        let reviewer = default_department_draft(REVIEWER_DEPARTMENT_ID, "zh-CN")
            .expect("reviewer default draft");
        assert_eq!(reviewer.api_config_id, MODEL_ROLE_QUICK_API_CONFIG_ID);
        assert_eq!(
            reviewer.permission_control,
            reviewer_department_permission_control()
        );

        let saddler = default_department_draft(SADDLER_DEPARTMENT_ID, "zh-CN")
            .expect("saddler default draft");
        assert_eq!(saddler.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert_eq!(
            saddler.permission_control,
            saddler_department_permission_control()
        );

        let remote_customer_service =
            default_department_draft(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID, "zh-CN")
                .expect("remote customer service default draft");
        assert_eq!(
            remote_customer_service.permission_control,
            remote_customer_service_department_permission_control()
        );

        let assistant = default_department_draft(ASSISTANT_DEPARTMENT_ID, "en-US")
            .expect("assistant default draft");
        assert_eq!(assistant.name, "Assistant Department");

        let config = AppConfig::default();
        for department_id in [
            ASSISTANT_DEPARTMENT_ID,
            LEADER_DEPARTMENT_ID,
            DEPUTY_DEPARTMENT_ID,
            REVIEWER_DEPARTMENT_ID,
            SADDLER_DEPARTMENT_ID,
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID,
        ] {
            let department = config
                .departments
                .iter()
                .find(|department| department.id == department_id)
                .expect("default preset department");
            assert!(
                department_permission_allows_any_name(
                    Some(department),
                    DepartmentPermissionCategory::Skill,
                    &["memory-generation"],
                ),
                "{department_id} should allow memory-generation",
            );
        }

        assert!(default_department_draft("department-custom", "zh-CN").is_err());
    }

    #[test]
    fn app_data_default_should_include_deputy_agent() {
        let data = AppData::default();
        assert!(data.agents.iter().any(|agent| agent.id == DEPUTY_AGENT_ID));
    }

    #[test]
    fn normalize_app_config_should_drop_invalid_department_models_without_clearing_expert_model() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            code_font: default_code_font(),
            ui_size_scale: default_ui_size_scale(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            skipped_github_update_version: String::new(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            desktop_operation_notice_enabled: default_desktop_operation_notice_enabled(),
            desktop_operate_enabled: default_desktop_operate_enabled(),
            selected_api_config_id: "embed-a".to_string(),
            assistant_department_api_config_id: "chat-a".to_string(),
            vision_api_config_id: None,
            image_generation_model_id: None,
            image_providers: Vec::new(),
            stt_api_config_id: None,
            stt_auto_send: false,
            simple_setup_mode: false,
            provider_non_stream_base_urls: Vec::new(),
            terminal_shell_kind: default_terminal_shell_kind(),
            shell_workspaces: Vec::new(),
            mcp_servers: Vec::new(),
            remote_im_channels: Vec::new(),
            departments: vec![
                DepartmentConfig {
                    id: ASSISTANT_DEPARTMENT_ID.to_string(),
                    name: "助理部门".to_string(),
                    summary: String::new(),
                    guide: String::new(),
                    api_config_ids: vec!["embed-a".to_string()],
                    api_config_id: "embed-a".to_string(),
                    model_failure_fallback_enabled: false,
                    agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
                    child_department_ids: Vec::new(),
                    created_at: "2026-03-10T00:00:00Z".to_string(),
                    updated_at: "2026-03-10T00:00:00Z".to_string(),
                    order_index: 1,
                    is_built_in_assistant: true,
                    is_deputy: false,
                    source: default_main_source(),
                    scope: default_global_scope(),
                    permission_control: DepartmentPermissionControl::default(),
                },
                DepartmentConfig {
                    id: "department-research".to_string(),
                    name: "资料部".to_string(),
                    summary: String::new(),
                    guide: String::new(),
                    api_config_ids: vec!["stt-a".to_string()],
                    api_config_id: "stt-a".to_string(),
                    model_failure_fallback_enabled: false,
                    agent_ids: vec![],
                    child_department_ids: Vec::new(),
                    created_at: "2026-03-10T00:00:00Z".to_string(),
                    updated_at: "2026-03-10T00:00:00Z".to_string(),
                    order_index: 2,
                    is_built_in_assistant: false,
                    is_deputy: false,
                    source: default_main_source(),
                    scope: default_global_scope(),
                    permission_control: DepartmentPermissionControl::default(),
                },
            ],
            api_configs: vec![
                ApiConfig {
                    id: "embed-a".to_string(),
                    name: "embed-a".to_string(),
                    request_format: RequestFormat::OpenAIEmbedding,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: false,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "embed".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
                ApiConfig {
                    id: "stt-a".to_string(),
                    name: "stt-a".to_string(),
                    request_format: RequestFormat::OpenAIStt,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: false,
                    enable_image: false,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "stt".to_string(),
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                    failure_retry_count: 0,
                },
                ApiConfig {
                    id: "chat-a".to_string(),
                    name: "chat-a".to_string(),
                    request_format: RequestFormat::OpenAI,
                    allow_concurrent_requests: false,
                    max_concurrent_requests: None,
                    enable_text: true,
                    enable_image: true,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: false,
                    tools: vec![],
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "k".to_string(),
                    codex_auth_mode: default_codex_auth_mode(),
                    codex_local_auth_path: default_codex_local_auth_path(),
                    codex_custom_url: None,
                    codex_custom_api_key: None,
                    codex_originator: default_codex_originator(),
                    codex_residency_requirement: None,
                    model: "chat".to_string(),
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
            tool_review_api_config_id: None,
        };

        normalize_app_config(&mut cfg);

        assert_eq!(
            cfg.assistant_department_api_config_id,
            "chat-a::chat-a-model-default"
        );
        let assistant = cfg
            .departments
            .iter()
            .find(|item| item.id == ASSISTANT_DEPARTMENT_ID)
            .expect("assistant department");
        assert_eq!(assistant.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert_eq!(assistant.api_config_ids, vec![MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string()]);
        let research = cfg
            .departments
            .iter()
            .find(|item| item.id == "department-research")
            .expect("research department");
        assert_eq!(research.api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert_eq!(research.api_config_ids, vec![MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string()]);
    }

    #[test]
    fn normalize_app_config_should_preserve_empty_expert_model_while_defaulting_department_role() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            code_font: default_code_font(),
            ui_size_scale: default_ui_size_scale(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            skipped_github_update_version: String::new(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            desktop_operation_notice_enabled: default_desktop_operation_notice_enabled(),
            desktop_operate_enabled: default_desktop_operate_enabled(),
            selected_api_config_id: "chat-a".to_string(),
            assistant_department_api_config_id: String::new(),
            vision_api_config_id: None,
            image_generation_model_id: None,
            image_providers: Vec::new(),
            stt_api_config_id: None,
            stt_auto_send: false,
            simple_setup_mode: false,
            provider_non_stream_base_urls: Vec::new(),
            terminal_shell_kind: default_terminal_shell_kind(),
            shell_workspaces: Vec::new(),
            mcp_servers: Vec::new(),
            remote_im_channels: Vec::new(),
            departments: vec![DepartmentConfig {
                id: ASSISTANT_DEPARTMENT_ID.to_string(),
                name: "助理部门".to_string(),
                summary: String::new(),
                guide: String::new(),
                api_config_ids: Vec::new(),
                api_config_id: String::new(),
                model_failure_fallback_enabled: false,
                agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
                child_department_ids: Vec::new(),
                created_at: "2026-03-10T00:00:00Z".to_string(),
                updated_at: "2026-03-10T00:00:00Z".to_string(),
                order_index: 1,
                is_built_in_assistant: true,
                is_deputy: false,
                source: default_main_source(),
                scope: default_global_scope(),
                permission_control: DepartmentPermissionControl::default(),
            }],
            api_configs: vec![ApiConfig {
                id: "chat-a".to_string(),
                name: "chat-a".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: true,
                enable_audio: false,
                enable_video: false,
                enable_tools: false,
                tools: vec![],
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "k".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                codex_custom_url: None,
                codex_custom_api_key: None,
                codex_originator: default_codex_originator(),
                codex_residency_requirement: None,
                model: "chat".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 1.0,
                custom_temperature_enabled: false,
                context_window_tokens: 128_000,
                max_output_tokens: 4_096,
                custom_max_output_tokens_enabled: false,
                failure_retry_count: 0,
            }],
            api_providers: Vec::new(),
            tool_review_api_config_id: None,
        };

        normalize_app_config(&mut cfg);

        assert_eq!(cfg.assistant_department_api_config_id, "");
        assert_eq!(cfg.departments[0].api_config_id, MODEL_ROLE_EXPERT_API_CONFIG_ID);
        assert_eq!(cfg.departments[0].api_config_ids, vec![MODEL_ROLE_EXPERT_API_CONFIG_ID.to_string()]);
    }

    #[test]
    fn resolve_api_config_should_preserve_openai_reasoning_none_for_runtime() {
        let mut cfg = AppConfig::default();
        let api = cfg
            .api_configs
            .iter_mut()
            .find(|item| item.id == cfg.selected_api_config_id)
            .expect("default api exists");
        api.request_format = RequestFormat::OpenAI;
        api.base_url = "https://api.deepseek.com/v1".to_string();
        api.api_key = "sk-test".to_string();
        api.model = "deepseek-v4-pro".to_string();
        api.reasoning_effort = "none".to_string();

        normalize_app_config(&mut cfg);

        let resolved = resolve_api_config(&cfg, Some(&cfg.selected_api_config_id))
            .expect("resolved api config");

        assert_eq!(resolved.reasoning_effort, Some("none".to_string()));
    }

    #[test]
    fn normalize_app_config_should_not_copy_builtin_department_model_to_expert_model() {
        let mut chat_a = ApiConfig::default();
        chat_a.id = "chat-a".to_string();
        chat_a.name = "chat-a".to_string();
        chat_a.request_format = RequestFormat::OpenAI;
        chat_a.enable_text = true;
        chat_a.base_url = "https://api.openai.com/v1".to_string();
        chat_a.api_key = "k".to_string();
        chat_a.model = "chat-a".to_string();

        let mut chat_b = chat_a.clone();
        chat_b.id = "chat-b".to_string();
        chat_b.name = "chat-b".to_string();
        chat_b.model = "chat-b".to_string();

        let mut assistant = default_assistant_department("chat-b");
        assistant.api_config_id = "chat-b".to_string();
        assistant.api_config_ids = vec!["chat-b".to_string()];

        let mut cfg = AppConfig {
            selected_api_config_id: "chat-a".to_string(),
            assistant_department_api_config_id: "chat-a".to_string(),
            departments: vec![assistant],
            api_configs: vec![chat_a, chat_b],
            api_providers: Vec::new(),
            ..AppConfig::default()
        };

        normalize_app_config(&mut cfg);

        assert_eq!(
            cfg.assistant_department_api_config_id,
            "chat-a::chat-a-model-default"
        );
        let assistant = cfg
            .departments
            .iter()
            .find(|item| item.id == ASSISTANT_DEPARTMENT_ID)
            .expect("assistant department");
        assert_eq!(
            assistant.api_config_id,
            "chat-b::chat-b-model-default"
        );
    }

    #[test]
    fn normalize_terminal_path_input_should_strip_wrapping_quotes() {
        let out = normalize_terminal_path_input_for_current_platform(r#""./repo""#);
        assert_eq!(out, "./repo".to_string());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn normalize_terminal_path_input_should_convert_git_bash_style_on_windows() {
        let out = normalize_terminal_path_input_for_current_platform("/e/work/repo");
        assert_eq!(out, r"E:\work\repo".to_string());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn normalize_shell_workspaces_should_convert_and_dedup_windows_paths() {
        let mut cfg = AppConfig::default();
        cfg.shell_workspaces = vec![
            ShellWorkspaceConfig {
                name: "A".to_string(),
                path: "/e/__easy_call_ai_path_norm_test__/repo".to_string(),
                built_in: false,
                ..Default::default()
            },
            ShellWorkspaceConfig {
                name: "a".to_string(),
                path: "E:/__easy_call_ai_path_norm_test__/repo".to_string(),
                built_in: false,
                ..Default::default()
            },
            ShellWorkspaceConfig {
                name: "B".to_string(),
                path: r#""E:\__easy_call_ai_path_norm_test__\repo""#.to_string(),
                built_in: false,
                ..Default::default()
            },
        ];
        normalize_shell_workspaces(&mut cfg);
        assert_eq!(cfg.shell_workspaces.len(), 1);
        assert_eq!(
            cfg.shell_workspaces[0].path,
            r"E:\__easy_call_ai_path_norm_test__\repo".to_string()
        );
    }

    #[test]
    fn normalize_app_config_should_migrate_legacy_api_configs_into_providers() {
        let mut cfg = AppConfig {
            selected_api_config_id: "legacy-openai".to_string(),
            assistant_department_api_config_id: "legacy-openai".to_string(),
            api_providers: Vec::new(),
            tool_review_api_config_id: None,
            api_configs: vec![ApiConfig {
                id: "legacy-openai".to_string(),
                name: "Legacy OpenAI".to_string(),
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_video: false,
                enable_tools: true,
                tools: default_api_tools(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "legacy-key".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                codex_custom_url: None,
                codex_custom_api_key: None,
                codex_originator: default_codex_originator(),
                codex_residency_requirement: None,
                model: "gpt-4.1".to_string(),
                reasoning_effort: default_reasoning_effort(),
                temperature: 0.7,
                custom_temperature_enabled: true,
                context_window_tokens: 256_000,
                max_output_tokens: 8_192,
                custom_max_output_tokens_enabled: true,
                failure_retry_count: 2,
            }],
            ..AppConfig::default()
        };

        normalize_app_config(&mut cfg);

        assert_eq!(cfg.api_providers.len(), 1);
        assert_eq!(cfg.api_providers[0].api_keys, vec!["legacy-key".to_string()]);
        assert_eq!(cfg.api_providers[0].models.len(), 1);
        assert_eq!(cfg.api_providers[0].models[0].model, "gpt-4.1".to_string());
        assert_eq!(
            cfg.selected_api_config_id,
            "legacy-openai::legacy-openai-model-default".to_string()
        );
        assert_eq!(cfg.api_configs.len(), 1);
        assert_eq!(cfg.api_configs[0].id, cfg.selected_api_config_id);
    }

    #[test]
    fn normalize_app_config_should_migrate_legacy_api_configs_when_serde_injected_default_provider() {
        let mut cfg: AppConfig = toml::from_str(
            r#"
hotkey = "Alt+·"
selectedApiConfigId = "legacy-openai"
assistantDepartmentApiConfigId = "legacy-openai"

[[apiConfigs]]
id = "legacy-openai"
name = "Legacy OpenAI"
requestFormat = "openai"
enableText = true
enableImage = false
enableAudio = false
enableTools = true
baseUrl = "https://api.openai.com/v1"
apiKey = "legacy-key"
model = "gpt-4.1"
temperature = 0.7
contextWindowTokens = 256000
maxOutputTokens = 8192
"#,
        )
        .expect("legacy toml should deserialize");

        normalize_app_config(&mut cfg);

        assert_eq!(cfg.api_providers.len(), 1);
        assert_eq!(cfg.api_providers[0].id, "legacy-openai".to_string());
        assert_eq!(cfg.api_providers[0].api_keys, vec!["legacy-key".to_string()]);
        assert_eq!(cfg.api_providers[0].models.len(), 1);
        assert_eq!(cfg.api_providers[0].models[0].model, "gpt-4.1".to_string());
        assert_eq!(
            cfg.selected_api_config_id,
            "legacy-openai::legacy-openai-model-default".to_string()
        );
    }

    #[test]
    fn read_config_should_materialize_missing_model_enable_audio_as_false() {
        let root = std::env::temp_dir().join(format!("eca-config-enable-audio-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp config dir");
        let config_path = root.join("app_config.toml");
        std::fs::write(
            &config_path,
            r#"
hotkey = "Alt+·"
selectedApiConfigId = "provider-a::model-a"
assistantDepartmentApiConfigId = "provider-a::model-a"

[[apiProviders]]
id = "provider-a"
name = "Provider A"
requestFormat = "openai"
enableText = true
enableImage = false
enableAudio = false
enableVideo = false
enableTools = true
baseUrl = "https://example.com/v1"
apiKeys = ["k"]
cachedModelOptions = ["mimo-v2.5"]

[[apiProviders.models]]
id = "model-a"
model = "mimo-v2.5"
enableImage = true
enableVideo = false
enableTools = true
"#,
        )
        .expect("write config");

        let cfg = read_config(&config_path).expect("read config");
        let provider = cfg
            .api_providers
            .iter()
            .find(|item| item.id == "provider-a")
            .expect("provider-a exists");
        let model = provider
            .models
            .iter()
            .find(|item| item.id == "model-a")
            .expect("model-a exists");
        assert!(!model.enable_audio);

        let persisted = std::fs::read_to_string(&config_path).expect("read persisted config");
        assert!(persisted.contains("enableAudio = false"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn app_config_should_map_legacy_ui_size_presets_to_scales() {
        for (preset, expected_scale) in [("small", 75), ("default", 100), ("large", 125), ("extraLarge", 150)] {
            let mut doc = toml::Value::try_from(AppConfig::default()).expect("serialize default config");
            let table = doc.as_table_mut().expect("config is a TOML table");
            table.remove("uiSizeScale");
            table.insert("uiSizePreset".to_string(), toml::Value::String(preset.to_string()));

            let config: AppConfig = doc.try_into().expect("legacy preset should deserialize");
            assert_eq!(config.ui_size_scale, expected_scale, "preset: {preset}");
        }
    }

    #[test]
    fn app_config_should_deserialize_legacy_departments_without_timestamps() {
        let mut cfg: AppConfig = toml::from_str(
            r#"
hotkey = "Alt+·"
selectedApiConfigId = "legacy-openai"
assistantDepartmentApiConfigId = "legacy-openai"

[[departments]]
id = "assistant-department"
name = "助理部门"
agentIds = ["default-agent"]
apiConfigIds = ["legacy-openai"]

[[apiConfigs]]
id = "legacy-openai"
name = "Legacy OpenAI"
requestFormat = "openai"
enableText = true
enableImage = false
enableAudio = false
enableTools = true
baseUrl = "https://api.openai.com/v1"
apiKey = "legacy-key"
model = "gpt-4.1"
"#,
        )
        .expect("legacy department toml should deserialize");

        normalize_app_config(&mut cfg);

        let assistant = cfg
            .departments
            .iter()
            .find(|department| department.id == ASSISTANT_DEPARTMENT_ID)
            .expect("assistant department should exist");
        assert!(!assistant.created_at.trim().is_empty());
        assert_eq!(assistant.updated_at, assistant.created_at);
        assert!(assistant.order_index > 0);
    }

    #[test]
    fn private_department_id_conflict_should_be_skipped_with_repair_hint() {
        let root = std::env::temp_dir().join(format!("eca-private-org-conflict-{}", Uuid::new_v4()));
        let data_path = root.join("config").join("app_data.json");
        let departments_dir = root
            .join("llm-workspace")
            .join("private-organization")
            .join("departments");
        std::fs::create_dir_all(&departments_dir).expect("create private departments dir");
        let conflict_id = "literature-knowledge-center";
        std::fs::write(
            departments_dir.join("literature-knowledge-center.json"),
            r#"{
  "id": "literature-knowledge-center",
  "name": "文学知识中心",
  "agentIds": ["default-agent"]
}"#,
        )
        .expect("write private department");

        let mut cfg = AppConfig::default();
        let mut conflicting_department = cfg.departments[0].clone();
        conflicting_department.id = conflict_id.to_string();
        conflicting_department.name = "主配置文学知识中心".to_string();
        conflicting_department.is_built_in_assistant = false;
        cfg.departments.push(conflicting_department);
        let mut data = AppData::default();

        let result = merge_private_organization_into_runtime_data(&data_path, &mut cfg, &mut data)
            .expect("merge private organization should not fail globally");

        assert!(result.private_departments_loaded.is_empty());
        assert_eq!(result.private_departments_failed.len(), 1);
        let error = &result.private_departments_failed[0];
        assert!(error.skipped);
        assert!(error.error.contains("私有部门 id 与主配置冲突"));
        assert!(error.hint.contains("修改该私有部门 id"));
        assert_eq!(
            cfg.departments.iter().filter(|department| department.id == conflict_id).count(),
            1
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn consume_api_key_for_request_should_rotate_provider_keys_across_same_provider_models() {
        let provider_id = format!(
            "provider-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or(0)
        );
        let model_a = "model-a".to_string();
        let model_b = "model-b".to_string();
        let mut cfg = AppConfig {
            selected_api_config_id: api_endpoint_id(&provider_id, &model_a),
            assistant_department_api_config_id: api_endpoint_id(&provider_id, &model_a),
            api_providers: vec![ApiProviderConfig {
                id: provider_id.clone(),
                name: "OpenAI".to_string(),
                deprecated: false,
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_video: false,
                enable_tools: true,
                tools: default_api_tools(),
                base_url: "https://api.openai.com/v1".to_string(),
                codex_auth_mode: default_codex_auth_mode(),
                codex_local_auth_path: default_codex_local_auth_path(),
                codex_custom_url: None,
                codex_custom_api_key: None,
                codex_originator: default_codex_originator(),
                codex_residency_requirement: None,
                api_keys: vec!["key-1".to_string(), "key-2".to_string()],
                key_cursor: 0,
                cached_model_options: vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()],
                models: vec![
                    ApiModelConfig {
                        id: model_a.clone(),
                        model: "gpt-4.1".to_string(),
                        display_name: String::new(),
                        deprecated: false,
                        enable_image: false,
                        enable_audio: false,
                        enable_video: false,
                        enable_tools: true,
                        reasoning_effort: default_reasoning_effort(),
                        temperature: 1.0,
                        custom_temperature_enabled: false,
                        context_window_tokens: 128_000,
                        max_output_tokens: 4_096,
                        custom_max_output_tokens_enabled: false,
                    },
                    ApiModelConfig {
                        id: model_b.clone(),
                        model: "gpt-4.1-mini".to_string(),
                        display_name: String::new(),
                        deprecated: false,
                        enable_image: false,
                        enable_audio: false,
                        enable_video: false,
                        enable_tools: true,
                        reasoning_effort: default_reasoning_effort(),
                        temperature: 1.0,
                        custom_temperature_enabled: false,
                        context_window_tokens: 128_000,
                        max_output_tokens: 4_096,
                        custom_max_output_tokens_enabled: false,
                    },
                ],
                failure_retry_count: 0,
            }],
            api_configs: Vec::new(),
            ..AppConfig::default()
        };
        normalize_app_config(&mut cfg);

        let first = resolve_api_config(&cfg, Some(&api_endpoint_id(&provider_id, &model_a)))
            .expect("first resolve");
        let second = resolve_api_config(&cfg, Some(&api_endpoint_id(&provider_id, &model_b)))
            .expect("second resolve");
        let third = resolve_api_config(&cfg, Some(&api_endpoint_id(&provider_id, &model_a)))
            .expect("third resolve");

        assert_eq!(first.api_key, "key-1".to_string());
        assert_eq!(second.api_key, "key-1".to_string());
        assert_eq!(third.api_key, "key-1".to_string());

        let first_sent = consume_api_key_for_request(&first);
        let second_sent = consume_api_key_for_request(&second);
        let third_sent = consume_api_key_for_request(&third);

        assert_eq!(first_sent, "key-1".to_string());
        assert_eq!(second_sent, "key-2".to_string());
        assert_eq!(third_sent, "key-1".to_string());
    }

    #[test]
    fn resolve_api_config_should_use_codex_custom_api_key_for_custom_url_mode() {
        let provider_id = "codex-custom-provider".to_string();
        let model_id = "codex-model".to_string();
        let mut cfg = AppConfig {
            selected_api_config_id: api_endpoint_id(&provider_id, &model_id),
            assistant_department_api_config_id: api_endpoint_id(&provider_id, &model_id),
            api_providers: vec![ApiProviderConfig {
                id: provider_id.clone(),
                name: "SharedChat".to_string(),
                deprecated: false,
                request_format: RequestFormat::Codex,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: false,
                enable_video: false,
                enable_tools: true,
                tools: default_api_tools(),
                base_url: "https://new.sharedchat.cc/codex".to_string(),
                codex_auth_mode: CODEX_AUTH_MODE_CUSTOM_URL.to_string(),
                codex_local_auth_path: default_codex_local_auth_path(),
                codex_custom_url: Some("https://new.sharedchat.cc/codex".to_string()),
                codex_custom_api_key: Some("sharedchat-key".to_string()),
                codex_originator: default_codex_originator(),
                codex_residency_requirement: None,
                api_keys: Vec::new(),
                key_cursor: 0,
                cached_model_options: vec!["gpt-5.4".to_string()],
                models: vec![ApiModelConfig {
                    id: model_id.clone(),
                    model: "gpt-5.4".to_string(),
                    display_name: String::new(),
                    deprecated: false,
                    enable_image: false,
                    enable_audio: false,
                    enable_video: false,
                    enable_tools: true,
                    reasoning_effort: default_reasoning_effort(),
                    temperature: 1.0,
                    custom_temperature_enabled: false,
                    context_window_tokens: 128_000,
                    max_output_tokens: 4_096,
                    custom_max_output_tokens_enabled: false,
                }],
                failure_retry_count: 0,
            }],
            api_configs: Vec::new(),
            ..AppConfig::default()
        };
        normalize_app_config(&mut cfg);

        let resolved = resolve_api_config(&cfg, Some(&api_endpoint_id(&provider_id, &model_id)))
            .expect("custom url codex resolve");

        assert_eq!(resolved.api_key, "sharedchat-key".to_string());
        assert!(resolved.codex_auth.is_none());
        assert!(resolved
            .extra_headers
            .iter()
            .any(|(key, value)| key == "Session-Id" && !value.trim().is_empty()));
    }

    #[test]
    fn write_agents_shard_should_not_touch_conversations() {
        let root = std::env::temp_dir().join(format!("eca-app-data-shards-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut data = AppData::default();
        data.conversations = vec![build_test_conversation("conv-a", "Conversation A")];
        seed_app_data_shards(&data_path, &data).expect("seed layout");

        let agents_path = app_layout_agents_path(&data_path);
        let conversation_paths =
            message_store::message_store_paths(&data_path, "conv-a").expect("conversation paths");

        let conversation_before = message_store::message_store_shard_write_signature(&conversation_paths);

        let mut agents = data.agents.clone();
        agents.push(AgentProfile {
            id: "agent-added".to_string(),
            name: "Agent Added".to_string(),
            system_prompt: "test".to_string(),
            tools: default_agent_tools(),
            created_at: "2026-04-15T00:00:00Z".to_string(),
            updated_at: "2026-04-15T00:00:00Z".to_string(),
            avatar_path: None,
            avatar_updated_at: None,
            is_built_in_user: false,
            is_built_in_system: false,
            private_memory_enabled: false,
            memory_recall_mode: default_agent_memory_recall_mode(),
            source: default_main_source(),
            scope: default_global_scope(),
        });
        assert!(write_agents_shard(&data_path, &agents).expect("write agents shard"));
        assert_eq!(
            message_store::message_store_shard_write_signature(&conversation_paths),
            conversation_before
        );
        assert!(!std::fs::read(&agents_path).expect("read agents after runtime").is_empty());
    }

    #[test]
    fn runtime_volatile_normalization_should_not_require_rewriting_after_migration_version_recorded() {
        let root = std::env::temp_dir().join(format!("eca-read-baseline-migration-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");
        let mut data = AppData::default();
        data.data_migration_version =
            DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES;
        data.conversations = vec![build_test_conversation("conv-baseline", "Baseline")];
        seed_app_data_shards(&data_path, &data).expect("seed layout");
        let paths = message_store::message_store_paths(&data_path, "conv-baseline")
            .expect("conversation paths");
        let before = message_store::message_store_shard_write_signature(&paths);

        let restored = read_layout_app_data(&data_path).expect("read app data");
        let after = message_store::message_store_shard_write_signature(&paths);

        assert_eq!(
            restored.data_migration_version,
            DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES
        );
        assert_eq!(after, before);
        let mut conversation = read_conversation_shard_raw(&data_path, "conv-baseline")
            .expect("read raw conversation shard");
        assert!(conversation.messages[0].speaker_agent_id.is_none());
        normalize_conversation_runtime_volatile_fields(&mut conversation);
        assert_eq!(
            conversation.messages[0].speaker_agent_id.as_deref(),
            Some(USER_PERSONA_ID)
        );
        // 迁移版本已记录时读取不重写 message store：上面 after == before 已钉死。
        // 不断言 restored.conversations[0].messages 为空——分片读取必然返回完整消息，
        // 该断言在旧布局删除后不可能成立（99f5b81d 合并测试时遗留的矛盾断言）。
    }

    #[test]
    fn write_conversation_shard_should_write_message_store_and_only_touch_target() {
        let root = std::env::temp_dir().join(format!("eca-conversation-shard-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut data = AppData::default();
        data.conversations = vec![
            build_test_conversation("conv-a", "Conversation A"),
            build_test_conversation("conv-b", "Conversation B"),
        ];
        seed_app_data_shards(&data_path, &data).expect("seed layout");

        let legacy_conversation_a_path = app_layout_chat_conversation_path(&data_path, "conv-a");
        let legacy_conversation_b_path = app_layout_chat_conversation_path(&data_path, "conv-b");
        assert!(!legacy_conversation_a_path.exists());
        assert!(!legacy_conversation_b_path.exists());
        let conversation_a_paths =
            message_store::message_store_paths(&data_path, "conv-a").expect("conversation a paths");
        let conversation_b_paths =
            message_store::message_store_paths(&data_path, "conv-b").expect("conversation b paths");
        assert!(message_store::chat_store_read_status(&conversation_a_paths)
            .expect("conversation a sqlite status")
            .is_some());
        assert!(message_store::chat_store_read_status(&conversation_b_paths)
            .expect("conversation b sqlite status")
            .is_some());
        let mut conversation_a = read_conversation_shard(&data_path, "conv-a").expect("read conversation a");
        conversation_a.title = "Conversation A Updated".to_string();
        assert!(write_conversation_shard(&data_path, &conversation_a).expect("write conversation a"));

        let conversation_a_meta = message_store::chat_store_read_meta(&conversation_a_paths)
            .expect("read conversation a meta")
            .expect("conversation a meta exists");
        assert_eq!(conversation_a_meta.title(), "Conversation A Updated");
        let conversation_b_meta = message_store::chat_store_read_meta(&conversation_b_paths)
            .expect("read conversation b meta")
            .expect("conversation b meta exists");
        assert_eq!(conversation_b_meta.title(), "Conversation B");
        assert!(!legacy_conversation_a_path.exists());
        assert!(!legacy_conversation_b_path.exists());
    }

    #[test]
    fn upsert_chat_index_conversation_should_replace_existing_item_without_duplicates() {
        let mut conversation_a = build_test_conversation("conv-a", "Conversation A");
        let conversation_b = build_test_conversation("conv-b", "Conversation B");
        let mut index = build_chat_index_file(&[conversation_a.clone(), conversation_b.clone()]);

        conversation_a.updated_at = "2026-04-15T12:34:56Z".to_string();
        conversation_a.status = "archived".to_string();
        conversation_a.archived_at = Some("2026-04-15T12:34:56Z".to_string());

        upsert_chat_index_conversation(&mut index, &conversation_a);

        assert_eq!(index.conversations.len(), 2);
        let updated = index
            .conversations
            .iter()
            .find(|item| item.id == "conv-a")
            .expect("find updated chat index item");
        assert_eq!(updated.updated_at, "2026-04-15T12:34:56Z");
        assert_eq!(updated.status, "archived");
        assert_eq!(
            updated.archived_at.as_deref(),
            Some("2026-04-15T12:34:56Z")
        );
    }

    #[test]
    fn remove_chat_index_conversation_should_drop_matching_item_only() {
        let conversation_a = build_test_conversation("conv-a", "Conversation A");
        let conversation_b = build_test_conversation("conv-b", "Conversation B");
        let mut index = build_chat_index_file(&[conversation_a, conversation_b]);

        remove_chat_index_conversation(&mut index, "conv-a");

        assert_eq!(index.conversations.len(), 1);
        assert!(index.conversations.iter().all(|item| item.id != "conv-a"));
        assert!(index.conversations.iter().any(|item| item.id == "conv-b"));
    }

    #[test]
    fn migration_package_version_should_allow_importing_older_data_versions() {
        let manifest = MigrationManifest {
            schema_version: MIGRATION_SCHEMA_VERSION,
            migration_version:
                DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES,
            app_version: "0.18.8".to_string(),
            exported_at: "2026-07-07T00:00:00Z".to_string(),
        };
        let mut payload = MigrationPayload {
            config: AppConfig::default(),
            runtime_data: MigrationRuntimeData::default(),
            memories: Vec::new(),
            oauth_files: Vec::new(),
            avatar_files: Vec::new(),
        };
        payload.runtime_data.data_migration_version =
            DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES;

        let version = assert_manifest_version(&manifest, &payload).expect("allow older import");
        assert_eq!(
            version,
            DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES
        );
    }

    #[test]
    fn migration_package_version_should_reject_newer_data_versions() {
        let newer_version = DATA_MIGRATION_CURRENT_VERSION + 1;
        let manifest = MigrationManifest {
            schema_version: MIGRATION_SCHEMA_VERSION,
            migration_version: newer_version,
            app_version: "0.99.0".to_string(),
            exported_at: "2026-07-07T00:00:00Z".to_string(),
        };
        let mut payload = MigrationPayload {
            config: AppConfig::default(),
            runtime_data: MigrationRuntimeData::default(),
            memories: Vec::new(),
            oauth_files: Vec::new(),
            avatar_files: Vec::new(),
        };
        payload.runtime_data.data_migration_version = newer_version;

        let err = assert_manifest_version(&manifest, &payload).expect_err("reject newer import");
        assert!(err.contains("迁移版本不兼容"));
        assert!(err.contains(&format!("V{newer_version}")));
    }

    fn build_test_conversation(id: &str, title: &str) -> Conversation {
        Conversation {
            id: id.to_string(),
            title: title.to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: "chat".to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            updated_at: "2026-04-15T00:00:00Z".to_string(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            shell_work_mode: default_shell_work_mode(),
            archived_at: None,
            messages: vec![ChatMessage {
                id: format!("{id}-message-1"),
                role: "user".to_string(),
                created_at: "2026-04-15T00:00:00Z".to_string(),
                speaker_agent_id: None,
                parts: vec![MessagePart::Text {
                    text: "hello".to_string(),
                reasoning_content: None,
            }],
                extra_text_blocks: Vec::new(),
                provider_meta: None,
                tool_call: None,
                mcp_call: None,
            meme_annotations: None,
            }],
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

    fn config_test_state() -> AppState {
        let root = std::env::temp_dir().join(format!("eca-config-test-{}", Uuid::new_v4()));
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
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_conversation_metadata: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_field_metadata_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            llm_round_logs: Arc::new(Mutex::new(RecentLlmRoundLogs::default())),
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
            remote_im_reply_delegate_runtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_reply_delegate_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
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

    fn storage_and_stt_test_state() -> AppState {
        config_test_state()
    }
