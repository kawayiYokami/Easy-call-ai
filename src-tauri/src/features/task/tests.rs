    fn test_task_data_path(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "easy_call_ai_task_test_{}_{}",
            label,
            Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn task_test_state(label: &str) -> AppState {
        let data_path = test_task_data_path(label).join("app_data.json");
        let state = AppState::new().expect("create test app state");
        AppState { data_path, ..state }
    }

    fn write_task_test_snapshot(
        state: &AppState,
        runtime: &mut RuntimeStateFile,
        conversations: &[Conversation],
    ) {
        state_write_runtime_state_cached(state, runtime).expect("write runtime state");
        for conversation in conversations {
            state_write_conversation_cached(state, conversation).expect("write conversation");
        }
    }

    #[test]
    fn task_todo_from_legacy_fields_should_dedupe_same_status_and_todos() {
        let todo = task_todo_from_legacy_fields("请自行判断", &["请自行判断".to_string()]);
        assert_eq!(todo, "请自行判断");

        let prefixed = task_todo_from_legacy_fields("待办：请自行判断", &["请自行判断".to_string()]);
        assert_eq!(prefixed, "待办：请自行判断");
    }

    #[test]
    fn task_tool_how_from_args_should_dedupe_same_status_and_todos() {
        let args = TaskToolArgsWire {
            action: "create".to_string(),
            task_id: None,
            goal: None,
            how: None,
            why: None,
            title: None,
            cause: None,
            flow: None,
            todos: Some(vec!["请自行判断".to_string()]),
            status_summary: Some("请自行判断".to_string()),
            stage_key: None,
            append_note: None,
            completion_state: None,
            completion_conclusion: None,
            trigger: None,
        };

        assert_eq!(task_tool_how_from_args(&args).as_deref(), Some("请自行判断"));
    }

    #[test]
    fn task_store_should_persist_conversation_id() {
        let data_path = test_task_data_path("persist_conversation_id");
        let input = TaskCreateInput {
            goal: "跟进并发会话".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "检查调度".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: Some("0,30 * * * *".to_string()),
                end_at: Some("2026-04-10T12:00:00+08:00".to_string()),
                legacy_every_minutes: None,
            },
        };

        let created = task_store_create_task(&data_path, &input).expect("create task");
        assert_eq!(created.conversation_id.as_deref(), Some("conversation-a"));

        let fetched = task_store_get_task(&data_path, &created.task_id).expect("get task");
        assert_eq!(fetched.conversation_id.as_deref(), Some("conversation-a"));

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_mark_skipped_should_advance_next_run_atomically() {
        let data_path = test_task_data_path("mark_skipped_advances_next_run");
        let input = TaskCreateInput {
            goal: "跳过后重试".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "等待空闲后继续".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: Some("0,30 * * * *".to_string()),
                end_at: Some("2099-04-10T12:00:00+08:00".to_string()),
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");
        let before = task_store_get_task_record(&data_path, &created.task_id).expect("get before");
        assert!(before.last_triggered_at_utc.is_none());
        let before_run = before
            .trigger
            .run_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("before run");
        let before_next = before
            .trigger
            .next_run_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("before next run");
        assert_eq!(before_next, before_run);

        task_store_mark_skipped(&data_path, &created.task_id, "skipped", "busy skip")
            .expect("mark skipped");

        let after = task_store_get_task_record(&data_path, &created.task_id).expect("get after");
        let last = after
            .last_triggered_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("last triggered");
        let next = after
            .trigger
            .next_run_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("next run");
        assert!(next > last);
        let next_local = to_local_datetime(next);
        assert!(matches!(next_local.minute(), 0 | 30));
        let logs = task_store_list_run_log_records(&data_path, Some(&created.task_id), 10)
            .expect("list logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].outcome, "skipped");

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_migration_should_convert_subminute_legacy_interval_to_minute_cron() {
        let data_path = test_task_data_path("migrate_subminute_legacy_interval");
        let input = TaskCreateInput {
            goal: "兼容旧调度".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "迁移历史 every_minutes".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: Some("2026-04-10T12:00:00+08:00".to_string()),
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");
        let conn = task_store_open(&data_path).expect("open task db");
        conn.execute(
            "UPDATE task_record
             SET cron_expression = NULL,
                 every_minutes = 0.5,
                 trigger_kind = 'legacy_immediate'
             WHERE task_id = ?1",
            params![created.task_id.as_str()],
        )
        .expect("seed legacy trigger");

        task_store_apply_migrations(&conn).expect("apply task migrations");

        let migrated = conn
            .query_row(
                "SELECT cron_expression, every_minutes, trigger_kind
                 FROM task_record
                 WHERE task_id = ?1",
                params![created.task_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("read migrated task");
        assert_eq!(migrated.0.as_deref(), Some("* * * * *"));
        assert!(migrated.1.is_none());
        assert_eq!(migrated.2, "cron");

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_migration_should_keep_unsupported_legacy_interval() {
        let data_path = test_task_data_path("migrate_keep_legacy_interval");
        let input = TaskCreateInput {
            goal: "保留旧间隔".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "不要硬转错 cron".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: Some("2099-04-10T15:00:00+08:00".to_string()),
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");
        let conn = task_store_open(&data_path).expect("open task db");
        conn.execute(
            "UPDATE task_record
             SET cron_expression = NULL,
                 every_minutes = 45,
                 trigger_kind = 'legacy_immediate'
             WHERE task_id = ?1",
            params![created.task_id.as_str()],
        )
        .expect("seed legacy trigger");

        task_store_apply_migrations(&conn).expect("apply task migrations");

        let migrated = task_store_get_task_record(&data_path, &created.task_id).expect("read migrated task");
        assert!(migrated.trigger.cron_expression.is_none());
        assert_eq!(migrated.trigger.legacy_every_minutes, Some(45.0));
        let next_run = migrated
            .trigger
            .next_run_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("next run");
        let next_run_local = to_local_datetime(next_run);
        assert_eq!(next_run_local.hour(), 10);
        assert_eq!(next_run_local.minute(), 0);

        task_store_mark_skipped(&data_path, &created.task_id, "skipped", "legacy interval")
            .expect("mark skipped");
        let after = task_store_get_task_record(&data_path, &created.task_id).expect("read after");
        let last_after_skip = after
            .last_triggered_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("last triggered after skip");
        let next_after_skip = after
            .trigger
            .next_run_at_utc
            .as_deref()
            .and_then(parse_rfc3339_time)
            .expect("next run after skip");
        assert_eq!(
            (next_after_skip - last_after_skip).whole_minutes(),
            45,
        );

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_cron_parse_field_should_treat_full_range_forms_as_unrestricted() {
        let schedule = task_parse_cron_expression("0 9 */1 * 1").expect("parse cron");
        assert!(schedule.dom_unrestricted);
        assert!(!schedule.dow_unrestricted);

        let monday = parse_rfc3339_time("2026-04-13T09:00:00+08:00").expect("parse monday");
        let tuesday = parse_rfc3339_time("2026-04-14T09:00:00+08:00").expect("parse tuesday");
        assert!(task_cron_matches_local(&schedule, monday));
        assert!(!task_cron_matches_local(&schedule, tuesday));

        let weekday_full_range = task_parse_cron_expression("0 9 * * 0-6").expect("parse full weekday range");
        assert!(weekday_full_range.dow_unrestricted);

        let dom_full_range = task_parse_cron_expression("0 9 1-31 * *").expect("parse full dom range");
        assert!(dom_full_range.dom_unrestricted);
    }

    #[test]
    fn task_dispatch_conversation_should_prefer_bound_then_fallback_to_main() {
        let state = task_test_state("dispatch_prefer_bound");
        let mut runtime = RuntimeStateFile::default();
        let api_id = "api-a";
        let agent_id = DEFAULT_AGENT_ID;

        let mut main = build_conversation_record(
            api_id,
            agent_id,
            ASSISTANT_DEPARTMENT_ID,
            "main",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        main.id = "main-conversation".to_string();
        let mut side = build_conversation_record(
            api_id,
            agent_id,
            REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID,
            "side",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        side.id = "side-conversation".to_string();
        runtime.main_conversation_id = Some(main.id.clone());
        write_task_test_snapshot(&state, &mut runtime, &[main.clone(), side.clone()]);

        let preferred = task_resolve_dispatch_conversation(
            &state,
            &mut runtime,
            api_id,
            agent_id,
            Some(side.id.as_str()),
            TASK_TARGET_SCOPE_DESKTOP,
        )
        .expect("resolve bound conversation")
        .expect("preferred conversation");
        assert_eq!(preferred.conversation_id, side.id);
        assert_eq!(preferred.target_scope, TASK_TARGET_SCOPE_DESKTOP);
        assert!(!preferred.fallback_to_main);

        let fallback = task_resolve_dispatch_conversation(
            &state,
            &mut runtime,
            api_id,
            agent_id,
            Some("missing-conversation"),
            TASK_TARGET_SCOPE_DESKTOP,
        )
        .expect("fallback to main conversation")
        .expect("fallback conversation");
        assert_eq!(fallback.conversation_id, main.id);
        assert_eq!(fallback.target_scope, TASK_TARGET_SCOPE_DESKTOP);
        assert!(fallback.fallback_to_main);

        let _ = fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn task_dispatch_conversation_should_skip_missing_contact_conversation() {
        let state = task_test_state("dispatch_skip_missing_contact");
        let mut runtime = RuntimeStateFile::default();
        let api_id = "api-a";
        let agent_id = DEFAULT_AGENT_ID;

        let mut main = build_conversation_record(
            api_id,
            agent_id,
            ASSISTANT_DEPARTMENT_ID,
            "main",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        main.id = "main-conversation".to_string();
        runtime.main_conversation_id = Some(main.id.clone());
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
            bound_conversation_id: Some("missing-contact-conversation".to_string()),
            processing_mode: "continuous".to_string(),
            response_strategy: default_remote_im_contact_response_strategy(),
            response_guidance: default_remote_im_contact_response_guidance(),
            last_activated_at: None,
            last_message_at: None,
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            shell_workspaces: Vec::new(),
        });
        write_task_test_snapshot(&state, &mut runtime, &[main]);

        let resolved = task_resolve_dispatch_conversation(
            &state,
            &mut runtime,
            api_id,
            agent_id,
            Some("missing-contact-conversation"),
            TASK_TARGET_SCOPE_CONTACT,
        )
        .expect("resolve missing contact conversation");

        assert!(resolved.is_none());

        let _ = fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }
