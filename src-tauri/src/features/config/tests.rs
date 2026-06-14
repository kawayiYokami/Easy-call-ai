    #[test]
    fn image_text_cache_upsert_and_find_should_work() {
        let mut data = AppData::default();
        upsert_image_text_cache(&mut data, "h1", "vision-a", "text-a");
        assert_eq!(
            find_image_text_cache(&data, "h1", "vision-a"),
            Some("text-a".to_string())
        );

        upsert_image_text_cache(&mut data, "h1", "vision-a", "text-b");
        assert_eq!(
            find_image_text_cache(&data, "h1", "vision-a"),
            Some("text-b".to_string())
        );
        assert_eq!(find_image_text_cache(&data, "h1", "vision-b"), None);
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
    fn startup_window_label_should_open_quick_setup_without_usable_text_llm() {
        let mut cfg = AppConfig::default();
        normalize_app_config(&mut cfg);
        assert_eq!(startup_window_label_for_config(&cfg), "quick-setup");

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
        assert_eq!(startup_window_label_for_config(&cfg), "quick-setup");
    }

    #[test]
    fn normalize_app_config_should_fix_invalid_record_and_stt_fields() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            webview_zoom_percent: default_webview_zoom_percent(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            record_hotkey: "".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 0,
            max_record_seconds: 0,
            tool_max_iterations: 0,
            llm_round_log_capacity: 9,
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            selected_api_config_id: "a1".to_string(),
            assistant_department_api_config_id: "a1".to_string(),
            vision_api_config_id: None,
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
            webview_zoom_percent: default_webview_zoom_percent(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            selected_api_config_id: "edit-b".to_string(),
            assistant_department_api_config_id: "chat-a".to_string(),
            vision_api_config_id: None,
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
    fn normalize_app_config_should_disable_audio_capability_globally() {
        let mut cfg = AppConfig {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            webview_zoom_percent: default_webview_zoom_percent(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            selected_api_config_id: "tts-a".to_string(),
            assistant_department_api_config_id: "tts-a".to_string(),
            vision_api_config_id: Some("tts-a".to_string()),
            stt_api_config_id: Some("tts-a".to_string()),
            stt_auto_send: true,
            provider_non_stream_base_urls: Vec::new(),
            terminal_shell_kind: default_terminal_shell_kind(),
            shell_workspaces: Vec::new(),
            mcp_servers: Vec::new(),
            remote_im_channels: Vec::new(),
            departments: Vec::new(),
            api_configs: vec![ApiConfig {
                id: "tts-a".to_string(),
                name: "tts-a".to_string(),
                request_format: RequestFormat::OpenAITts,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                enable_text: true,
                enable_image: false,
                enable_audio: true,
                enable_video: false,
                enable_tools: true,
                tools: vec![],
                base_url: "https://api.siliconflow.cn/v1/audio/transcriptions".to_string(),
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
            }],
            api_providers: Vec::new(),
            tool_review_api_config_id: None,
        };
        normalize_app_config(&mut cfg);
        let api = &cfg.api_configs[0];
        assert!(api.enable_text);
        assert!(!api.enable_image);
        assert!(!api.enable_audio);
        assert!(api.enable_tools);
        assert_eq!(cfg.vision_api_config_id, None);
        assert!(cfg.stt_auto_send);
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
    fn normalize_app_config_should_restore_deputy_department_and_attach_to_assistant() {
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
        assert!(
            assistant
                .child_department_ids
                .iter()
                .any(|id| id == DEPUTY_DEPARTMENT_ID)
        );
    }

    #[test]
    fn normalize_app_config_should_restore_leader_department_with_default_children() {
        let mut cfg = AppConfig::default();
        cfg.departments
            .retain(|item| item.id != LEADER_DEPARTMENT_ID);

        normalize_app_config(&mut cfg);

        let leader = cfg
            .departments
            .iter()
            .find(|item| item.id == LEADER_DEPARTMENT_ID)
            .expect("leader department");
        assert_eq!(leader.name, "leader");
        assert_eq!(leader.agent_ids, vec![DEFAULT_AGENT_ID.to_string()]);
        assert!(leader.summary.contains("协调下级部门"));
        assert!(leader.guide.contains("`delegate`"));
        assert!(leader.guide.contains("`mode` 固定使用 `wait`"));
        assert!(leader.guide.contains("`wait` 可以并发发出多个委托"));
        assert!(leader.guide.contains("不要盲目相信未经核验的下级结论"));
        assert!(!leader.guide.contains("async"));
        assert!(!leader.guide.contains("`plan`"));
        assert!(!leader.guide.contains("`todo`"));
        assert!(!leader.guide.contains("`task`"));
        assert!(!leader.guide.contains("task(action"));
        assert!(!leader.guide.contains("new_task"));
        assert!(!leader.guide.contains("attempt_completion"));
        assert!(!leader.guide.contains("完成工具"));
        assert_eq!(
            leader.child_department_ids,
            vec![
                ASSISTANT_DEPARTMENT_ID.to_string(),
                DEPUTY_DEPARTMENT_ID.to_string(),
            ]
        );
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
            webview_zoom_percent: default_webview_zoom_percent(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            selected_api_config_id: "embed-a".to_string(),
            assistant_department_api_config_id: "chat-a".to_string(),
            vision_api_config_id: None,
            stt_api_config_id: None,
            stt_auto_send: false,
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
            webview_zoom_percent: default_webview_zoom_percent(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            record_hotkey: "Alt".to_string(),
            record_background_wake_enabled: false,
            min_record_seconds: 1,
            max_record_seconds: 60,
            tool_max_iterations: 10,
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            selected_api_config_id: "chat-a".to_string(),
            assistant_department_api_config_id: String::new(),
            vision_api_config_id: None,
            stt_api_config_id: None,
            stt_auto_send: false,
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
                        enable_image: false,
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
                        enable_image: false,
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
                    enable_image: false,
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
            .any(|(key, value)| key == "session_id" && !value.trim().is_empty()));
    }

    #[test]
    fn write_app_data_should_only_flush_changed_shards() {
        let root = std::env::temp_dir().join(format!("eca-app-data-layout-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut data = AppData::default();
        data.conversations = vec![
            build_test_conversation("conv-a", "Conversation A"),
            build_test_conversation("conv-b", "Conversation B"),
        ];

        let first = write_app_data_with_stats(&data_path, &data).expect("write first layout");
        assert!(first.agents_written);
        assert!(first.runtime_written);
        assert_eq!(first.conversation_writes, 3);
        assert_eq!(first.conversation_deletes, 0);
        assert!(!app_layout_chat_index_path(&data_path).exists());
        let system_notification = read_conversation_shard(
            &data_path,
            SYSTEM_NOTIFICATION_CONVERSATION_ID,
        )
        .expect("system notification conversation should be materialized");
        assert_eq!(
            system_notification.conversation_kind,
            CONVERSATION_KIND_SYSTEM_NOTIFICATION
        );

        let second = write_app_data_with_stats(&data_path, &data).expect("write same layout");
        assert!(!second.agents_written);
        assert!(!second.runtime_written);
        assert_eq!(second.conversation_writes, 0);
        assert_eq!(second.conversation_deletes, 0);

        let mut runtime_only = data.clone();
        runtime_only.assistant_department_agent_id = "agent-runtime-only".to_string();
        let runtime_stats =
            write_app_data_with_stats(&data_path, &runtime_only).expect("write runtime-only diff");
        assert!(!runtime_stats.agents_written);
        assert!(runtime_stats.runtime_written);
        assert_eq!(runtime_stats.conversation_writes, 0);
        assert_eq!(runtime_stats.conversation_deletes, 0);
    }

    #[test]
    fn write_agents_and_runtime_shards_should_not_touch_other_files() {
        let root = std::env::temp_dir().join(format!("eca-app-data-shards-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut data = AppData::default();
        data.assistant_department_agent_id = "assistant-before".to_string();
        data.conversations = vec![build_test_conversation("conv-a", "Conversation A")];
        write_app_data_with_stats(&data_path, &data).expect("seed layout");

        let agents_path = app_layout_agents_path(&data_path);
        let runtime_path = app_layout_runtime_state_path(&data_path);
        let conversation_paths =
            message_store::message_store_paths(&data_path, "conv-a").expect("conversation paths");

        let runtime_before = std::fs::read(&runtime_path).expect("read runtime before");
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
        assert_eq!(std::fs::read(&runtime_path).expect("read runtime after agents"), runtime_before);
        assert_eq!(
            message_store::message_store_shard_write_signature(&conversation_paths),
            conversation_before
        );

        let mut runtime = read_runtime_state_shard(&data_path).expect("read runtime shard");
        runtime.assistant_department_agent_id = "assistant-after".to_string();
        assert!(write_runtime_state_shard(&data_path, &runtime).expect("write runtime shard"));
        assert_ne!(
            std::fs::read(&runtime_path).expect("read runtime after runtime write"),
            runtime_before
        );
        assert_eq!(
            message_store::message_store_shard_write_signature(&conversation_paths),
            conversation_before
        );
        assert!(!std::fs::read(&agents_path).expect("read agents after runtime").is_empty());
    }

    #[test]
    fn runtime_state_should_read_legacy_message_store_migration_version_separately() {
        let root = std::env::temp_dir().join(format!("eca-runtime-legacy-migration-version-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("state")).expect("create temp state dir");
        let data_path = root.join("config").join("app_data.json");
        std::fs::write(
            app_layout_runtime_state_path(&data_path),
            r#"{
  "version": 1,
  "messageStoreMigrationVersion": 1,
  "assistantDepartmentAgentId": "default-agent",
  "userAlias": "用户",
  "responseStyleId": "concise",
  "pdfReadMode": "image",
  "backgroundVoiceScreenshotKeywords": "",
  "backgroundVoiceScreenshotMode": "focused_window",
  "mainConversationId": "system-notification-conversation"
}"#,
        )
        .expect("write legacy runtime state");

        let runtime = read_runtime_state_shard(&data_path).expect("read runtime shard");

        assert_eq!(runtime.data_migration_version, 0);
        assert_eq!(
            runtime.message_store_migration_version,
            DATA_MIGRATION_VERSION_V1_BASELINE
        );
    }

    #[test]
    fn write_app_data_should_not_downgrade_data_migration_version() {
        let root = std::env::temp_dir().join(format!("eca-app-data-migration-version-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");
        let data = AppData::default();
        write_app_data_with_stats(&data_path, &data).expect("seed layout");

        let mut runtime = read_runtime_state_shard(&data_path).expect("read runtime shard");
        runtime.data_migration_version = DATA_MIGRATION_VERSION_V1_BASELINE;
        assert!(write_runtime_state_shard(&data_path, &runtime).expect("write baseline migration version"));

        let stats = write_app_data_with_stats(&data_path, &data).expect("write app data again");
        let restored = read_runtime_state_shard(&data_path).expect("read restored runtime shard");

        assert_eq!(restored.data_migration_version, DATA_MIGRATION_VERSION_V1_BASELINE);
        assert_eq!(restored.message_store_migration_version, 0);
        assert!(!stats.runtime_written);
    }

    #[test]
    fn runtime_volatile_normalization_should_not_require_rewriting_after_baseline_recorded() {
        let root = std::env::temp_dir().join(format!("eca-read-baseline-migration-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");
        let mut data = AppData::default();
        data.data_migration_version = DATA_MIGRATION_VERSION_V1_BASELINE;
        data.conversations = vec![build_test_conversation("conv-baseline", "Baseline")];
        write_app_data_with_stats(&data_path, &data).expect("seed layout");
        let paths = message_store::message_store_paths(&data_path, "conv-baseline")
            .expect("conversation paths");
        let before = message_store::message_store_shard_write_signature(&paths);

        let mut restored = read_app_data(&data_path).expect("read app data");
        let after = message_store::message_store_shard_write_signature(&paths);

        assert_eq!(restored.data_migration_version, DATA_MIGRATION_VERSION_V1_BASELINE);
        assert_eq!(after, before);
        assert!(restored.conversations[0].messages[0].speaker_agent_id.is_none());
        normalize_app_data_runtime_volatile_fields(&mut restored);
        assert_eq!(
            restored.conversations[0].messages[0].speaker_agent_id.as_deref(),
            Some(USER_PERSONA_ID)
        );
    }

    #[test]
    fn read_app_data_should_record_baseline_without_rewriting_conversations_for_volatile_fields() {
        let root = std::env::temp_dir().join(format!("eca-read-record-baseline-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");
        let mut data = AppData::default();
        data.data_migration_version = 0;
        data.conversations = vec![build_test_conversation("conv-baseline", "Baseline")];
        write_app_data_with_stats(&data_path, &data).expect("seed layout");
        let paths = message_store::message_store_paths(&data_path, "conv-baseline")
            .expect("conversation paths");
        let before = message_store::message_store_shard_write_signature(&paths);

        let restored = read_app_data(&data_path).expect("read app data");
        let after = message_store::message_store_shard_write_signature(&paths);
        let runtime = read_runtime_state_shard(&data_path).expect("read runtime shard");

        assert_eq!(restored.data_migration_version, DATA_MIGRATION_CURRENT_VERSION);
        assert_eq!(runtime.data_migration_version, DATA_MIGRATION_CURRENT_VERSION);
        assert_eq!(restored.message_store_migration_version, 0);
        assert_eq!(runtime.message_store_migration_version, 0);
        assert_eq!(after, before);
        assert!(restored.conversations[0].messages[0].speaker_agent_id.is_none());
        let mut runtime_view = restored.clone();
        normalize_app_data_runtime_volatile_fields(&mut runtime_view);
        assert_eq!(
            runtime_view.conversations[0].messages[0].speaker_agent_id.as_deref(),
            Some(USER_PERSONA_ID)
        );
    }

    #[test]
    fn read_runtime_state_shard_should_materialize_fixed_system_notification_conversation() {
        let root = std::env::temp_dir().join(format!("eca-system-notification-shard-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let old_main = build_test_conversation("conversation-main", "主会话");
        write_conversation_shard(&data_path, &old_main).expect("write old main conversation");
        let mut runtime = RuntimeStateFile::default();
        runtime.main_conversation_id = Some("conversation-main".to_string());
        write_json_file_atomic(
            &app_layout_runtime_state_path(&data_path),
            &runtime,
            "runtime state file",
        )
        .expect("write runtime state directly");
        assert!(read_conversation_shard(&data_path, SYSTEM_NOTIFICATION_CONVERSATION_ID).is_err());

        let restored = read_runtime_state_shard(&data_path).expect("read runtime shard");

        assert_eq!(
            restored.main_conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );
        let system_notification = read_conversation_shard(
            &data_path,
            SYSTEM_NOTIFICATION_CONVERSATION_ID,
        )
        .expect("read system notification conversation");
        assert_eq!(system_notification.id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(system_notification.title, "P-ai系统");
        assert_eq!(
            system_notification.conversation_kind,
            CONVERSATION_KIND_SYSTEM_NOTIFICATION
        );
        assert!(system_notification.messages.is_empty());
        let old_main_after =
            read_conversation_shard(&data_path, "conversation-main").expect("read old main");
        assert_eq!(old_main_after.id, "conversation-main");
        assert_eq!(old_main_after.title, "主会话");
        assert_eq!(old_main_after.conversation_kind, CONVERSATION_KIND_CHAT);
    }

    #[test]
    fn runtime_state_shard_should_sync_remote_im_contact_communication_flags() {
        let root = std::env::temp_dir().join(format!("eca-app-data-contact-sync-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut runtime = RuntimeStateFile::default();
        runtime.remote_im_contacts.push(RemoteImContact {
            id: "contact-a".to_string(),
            channel_id: "channel-a".to_string(),
            platform: RemoteImPlatform::OnebotV11,
            remote_contact_type: "private".to_string(),
            remote_contact_id: "remote-a".to_string(),
            remote_contact_name: "张三".to_string(),
            avatar_url: String::new(),
            remark_name: String::new(),
            allow_send: false,
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
        });

        assert!(write_runtime_state_shard(&data_path, &runtime).expect("write runtime shard"));
        let restored = read_runtime_state_shard(&data_path).expect("read runtime shard");
        let contact = restored.remote_im_contacts.first().expect("contact exists");
        assert!(contact.allow_send);
        assert!(contact.allow_receive);
        let system_notification = read_conversation_shard(
            &data_path,
            SYSTEM_NOTIFICATION_CONVERSATION_ID,
        )
        .expect("read system notification conversation");
        assert_eq!(
            system_notification.conversation_kind,
            CONVERSATION_KIND_SYSTEM_NOTIFICATION
        );
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
        write_app_data_with_stats(&data_path, &data).expect("seed layout");

        let legacy_conversation_a_path = app_layout_chat_conversation_path(&data_path, "conv-a");
        let legacy_conversation_b_path = app_layout_chat_conversation_path(&data_path, "conv-b");
        assert!(!legacy_conversation_a_path.exists());
        assert!(!legacy_conversation_b_path.exists());
        let conversation_a_paths =
            message_store::message_store_paths(&data_path, "conv-a").expect("conversation a paths");
        let conversation_b_paths =
            message_store::message_store_paths(&data_path, "conv-b").expect("conversation b paths");
        assert!(message_store::should_write_jsonl_snapshot_directory_shard(&conversation_a_paths)
            .expect("conversation a manifest ready"));
        assert!(message_store::should_write_jsonl_snapshot_directory_shard(&conversation_b_paths)
            .expect("conversation b manifest ready"));
        let conversation_a_before =
            message_store::message_store_shard_write_signature(&conversation_a_paths);
        let conversation_b_before =
            message_store::message_store_shard_write_signature(&conversation_b_paths);

        let mut conversation_a = read_conversation_shard(&data_path, "conv-a").expect("read conversation a");
        conversation_a.title = "Conversation A Updated".to_string();
        assert!(write_conversation_shard(&data_path, &conversation_a).expect("write conversation a"));

        assert_ne!(
            message_store::message_store_shard_write_signature(&conversation_a_paths),
            conversation_a_before
        );
        assert_eq!(
            message_store::message_store_shard_write_signature(&conversation_b_paths),
            conversation_b_before
        );
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
        conversation_a.summary = "updated summary".to_string();
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
        assert_eq!(updated.summary, "updated summary");
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
    fn runtime_state_shard_should_preserve_pdf_caches() {
        let root = std::env::temp_dir().join(format!("eca-runtime-pdf-cache-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create temp config dir");
        let data_path = root.join("config").join("app_data.json");

        let mut data = AppData::default();
        data.pdf_text_cache.push(PdfTextCacheEntry {
            file_hash: "file-hash-a".to_string(),
            file_path: "C:/tmp/a.pdf".to_string(),
            file_name: "a.pdf".to_string(),
            extracted_text: "pdf text".to_string(),
            total_pages: 8,
            extracted_pages: 3,
            is_truncated: true,
            conversation_ids: vec!["conv-a".to_string()],
            created_at: "2026-04-15T00:00:00Z".to_string(),
            updated_at: "2026-04-15T00:00:00Z".to_string(),
        });
        data.pdf_image_cache.push(PdfImageCacheEntry {
            file_hash: "file-hash-b".to_string(),
            file_path: "C:/tmp/b.pdf".to_string(),
            file_name: "b.pdf".to_string(),
            total_pages: 4,
            rendered_pages: 2,
            dpi: 144,
            images: vec![PdfRenderedImage {
                page_index: 0,
                width: 100,
                height: 200,
                bytes_base64: "Zm9v".to_string(),
                mime: "image/png".to_string(),
            }],
            conversation_ids: vec!["conv-b".to_string()],
            created_at: "2026-04-15T00:00:00Z".to_string(),
            updated_at: "2026-04-15T00:00:00Z".to_string(),
        });

        let runtime = build_runtime_state_file(&data);
        assert!(write_runtime_state_shard(&data_path, &runtime).expect("write runtime shard"));

        let restored = read_runtime_state_shard(&data_path).expect("read runtime shard");
        assert_eq!(restored.pdf_text_cache.len(), 1);
        assert_eq!(restored.pdf_text_cache[0].file_name, "a.pdf");
        assert_eq!(restored.pdf_image_cache.len(), 1);
        assert_eq!(restored.pdf_image_cache[0].file_name, "b.pdf");
        assert_eq!(restored.pdf_image_cache[0].images.len(), 1);
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
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
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
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }
