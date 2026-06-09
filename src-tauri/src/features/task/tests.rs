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

    fn task_prompt_test_record(task_id: &str, goal: &str, why: &str, todo: &str) -> TaskRecordStored {
        TaskRecordStored {
            task_id: task_id.to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: TASK_TARGET_SCOPE_DESKTOP.to_string(),
            order_index: 1,
            title: goal.to_string(),
            cause: why.to_string(),
            goal: goal.to_string(),
            flow: String::new(),
            todos: task_legacy_todos_from_todo(todo),
            status_summary: task_legacy_status_summary_from_todo(todo),
            completion_state: TASK_STATE_ACTIVE.to_string(),
            completion_conclusion: String::new(),
            progress_notes: Vec::new(),
            stage_key: String::new(),
            stage_updated_at_utc: None,
            trigger: TaskTriggerStored {
                run_at_utc: Some("2026-04-10T02:00:00Z".to_string()),
                cron_expression: Some("0 * * * *".to_string()),
                legacy_every_minutes: None,
                end_at_utc: Some("2026-04-11T02:00:00Z".to_string()),
                next_run_at_utc: Some("2026-04-10T02:00:00Z".to_string()),
            },
            created_at_utc: "2026-04-10T01:00:00Z".to_string(),
            updated_at_utc: "2026-04-10T01:00:00Z".to_string(),
            last_triggered_at_utc: None,
            completed_at_utc: None,
        }
    }

    fn task_optimize_input(title: &str, content: &str) -> TaskOptimizeDraftInput {
        TaskOptimizeDraftInput {
            title: title.to_string(),
            content: content.to_string(),
            schedule_mode: "once".to_string(),
            run_at: "2026-06-10T17:00:00+08:00".to_string(),
            repeat_every: "1".to_string(),
            repeat_unit: "hours".to_string(),
            end_at: String::new(),
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
    fn task_trigger_prompt_should_use_goal_format_when_why_is_empty() {
        let task = task_prompt_test_record("task-goal", "整理发布清单", "", "检查剩余风险");

        let prompt = build_task_trigger_hidden_prompt(&task);

        assert_eq!(
            prompt,
            "<task_remind>\n背景：用户希望你能独立完成任务达成目标\n目标：整理发布清单\n要求：一直持续工作，直到达成目标，最后在当前会话进行工作汇报。\n</task_remind>"
        );
        assert!(!prompt.contains("task_id:"));
        assert!(!prompt.contains("run_at:"));
        assert!(!prompt.contains("cron_expression:"));
        assert!(!prompt.contains("task complete"));
    }

    #[test]
    fn task_trigger_prompt_should_use_llm_task_format_when_why_exists() {
        let task = task_prompt_test_record("task-llm", "跟进模型刷新", "用户稍后需要结果", "检查缓存并汇报");

        let prompt = build_task_trigger_hidden_prompt(&task);

        assert_eq!(
            prompt,
            "<task_remind>\n背景：用户稍后需要结果\n目标：跟进模型刷新\n要求：检查缓存并汇报\n</task_remind>"
        );
        assert!(!prompt.contains("run_at:"));
        assert!(!prompt.contains("{\"action\":\"complete\""));
        assert!(!prompt.contains("task complete"));
    }

    #[test]
    fn task_optimize_draft_prompt_should_require_content() {
        let err = task_optimize_draft_prompt(&task_optimize_input("", "   "))
        .expect_err("empty content should fail");

        assert!(err.contains("任务内容不能为空"));
    }

    #[test]
    fn task_optimize_draft_output_should_parse_title_and_content() {
        let parsed = task_optimize_draft_output_from_value(
            &serde_json::json!({
                "title": "提醒提交周报",
                "content": "到点后提醒用户整理并提交本周周报。",
                "scheduleMode": "interval",
                "runAt": "2026-06-12T10:00:00+08:00",
                "repeatEvery": "1",
                "repeatUnit": "weeks",
                "endAt": "2026-07-12T10:00:00+08:00"
            }),
            &task_optimize_input("", "每周五提醒我交周报"),
        )
        .expect("parse optimize draft output");

        assert_eq!(parsed.title, "提醒提交周报");
        assert_eq!(parsed.content, "到点后提醒用户整理并提交本周周报。");
        assert_eq!(parsed.schedule_mode, "interval");
        assert!(parse_rfc3339_time(&parsed.run_at).is_some());
        assert_eq!(parsed.repeat_every, "1");
        assert_eq!(parsed.repeat_unit, "weeks");
        assert!(parse_rfc3339_time(&parsed.end_at).is_some());
    }

    #[test]
    fn task_optimize_draft_output_should_fallback_title_but_require_content() {
        let parsed = task_optimize_draft_output_from_value(
            &serde_json::json!({
                "content": "提醒用户检查会议纪要并同步下一步。"
            }),
            &task_optimize_input("会议纪要跟进", "检查会议纪要"),
        )
        .expect("parse output with fallback title");
        assert_eq!(parsed.title, "会议纪要跟进");
        assert_eq!(parsed.schedule_mode, "once");
        assert!(parse_rfc3339_time(&parsed.run_at).is_some());

        let err = task_optimize_draft_output_from_value(
            &serde_json::json!({
                "title": "缺内容"
            }),
            &task_optimize_input("", "原始内容"),
        )
        .expect_err("missing content should fail");
        assert!(err.contains("有效任务内容"));
    }

    #[test]
    fn task_optimize_draft_output_should_normalize_invalid_repeat_months() {
        let parsed = task_optimize_draft_output_from_value(
            &serde_json::json!({
                "title": "月度提醒",
                "content": "提醒用户做月度复盘。",
                "scheduleMode": "interval",
                "runAt": "2026-06-10T09:00:00+08:00",
                "repeatEvery": "5",
                "repeatUnit": "months"
            }),
            &task_optimize_input("", "每5个月提醒我复盘"),
        )
        .expect("parse normalized monthly interval");

        assert_eq!(parsed.schedule_mode, "interval");
        assert_eq!(parsed.repeat_every, "1");
        assert_eq!(parsed.repeat_unit, "months");
        assert!(parsed.end_at.is_empty());
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
    fn task_store_should_normalize_empty_conversation_id_to_system() {
        let data_path = test_task_data_path("normalize_empty_conversation_id");
        let input = TaskCreateInput {
            goal: "系统任务".to_string(),
            conversation_id: None,
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "全局入口创建".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: None,
                legacy_every_minutes: None,
            },
        };

        let created = task_store_create_task(&data_path, &input).expect("create task");
        assert_eq!(
            created.conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );

        let fetched = task_store_get_task(&data_path, &created.task_id).expect("get task");
        assert_eq!(
            fetched.conversation_id.as_deref(),
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        );

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
    fn task_store_complete_one_time_dispatch_should_finish_task_without_conclusion() {
        let data_path = test_task_data_path("complete_one_time_dispatch");
        let input = TaskCreateInput {
            goal: "一次性调度".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "发起一次就结束".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: None,
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");

        let changed = task_store_complete_one_time_dispatch(
            &data_path,
            &created.task_id,
        )
        .expect("complete one-time dispatch");

        assert!(changed);
        let completed = task_store_get_task_record(&data_path, &created.task_id)
            .expect("get completed task");
        assert_eq!(completed.completion_state, TASK_STATE_COMPLETED);
        assert!(completed.completion_conclusion.is_empty());
        assert!(completed.last_triggered_at_utc.is_some());
        assert!(completed.completed_at_utc.is_some());

        let err = task_store_complete_task(&data_path, &TaskCompleteInput {
            task_id: created.task_id.clone(),
            completion_state: TASK_STATE_FAILED_COMPLETED.to_string(),
            completion_conclusion: "委托返回的最终结论".to_string(),
        })
        .expect_err("completed dispatch task should not accept hidden conclusion");
        assert!(err.contains("already completed"));

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_migration_should_complete_triggered_one_time_tasks() {
        let data_path = test_task_data_path("migrate_triggered_once_completed");
        let input = TaskCreateInput {
            goal: "历史一次性任务".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "已经触发过".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: None,
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");
        let conn = task_store_open(&data_path).expect("open task db");
        conn.execute(
            "UPDATE task_record
             SET last_triggered_at_utc = '2026-04-10T02:00:00Z',
                 completion_state = ?2
             WHERE task_id = ?1",
            params![created.task_id.as_str(), TASK_STATE_ACTIVE],
        )
        .expect("seed triggered once task");

        task_store_apply_migrations(&conn).expect("apply task migrations");

        let migrated = task_store_get_task_record(&data_path, &created.task_id)
            .expect("read migrated task");
        assert_eq!(migrated.completion_state, TASK_STATE_COMPLETED);
        assert!(migrated.completion_conclusion.is_empty());
        assert_eq!(
            migrated.completed_at_utc.as_deref(),
            Some("2026-04-10T02:00:00Z")
        );

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_migration_should_keep_future_one_time_tasks_active() {
        let data_path = test_task_data_path("migrate_future_once_active");
        let input = TaskCreateInput {
            goal: "未来一次性任务".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "不要误完成".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2099-04-10T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: None,
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");
        let conn = task_store_open(&data_path).expect("open task db");
        conn.execute(
            "UPDATE task_record
             SET last_triggered_at_utc = '2026-04-10T02:00:00Z',
                 completion_state = ?2
             WHERE task_id = ?1",
            params![created.task_id.as_str(), TASK_STATE_ACTIVE],
        )
        .expect("seed future once task with stale trigger");

        task_store_apply_migrations(&conn).expect("apply task migrations");

        let migrated = task_store_get_task_record(&data_path, &created.task_id)
            .expect("read migrated task");
        assert_eq!(migrated.completion_state, TASK_STATE_ACTIVE);
        assert!(migrated.completed_at_utc.is_none());

        let _ = fs::remove_dir_all(app_root_from_data_path(&data_path));
    }

    #[test]
    fn task_store_update_should_reject_recurring_to_one_time() {
        let data_path = test_task_data_path("reject_recurring_to_once");
        let input = TaskCreateInput {
            goal: "定时任务".to_string(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            why: String::new(),
            todo: "保持定时".to_string(),
            trigger: TaskTriggerInputLocal {
                run_at: Some("2026-04-10T10:00:00+08:00".to_string()),
                cron_expression: Some("0 10 * * *".to_string()),
                end_at: None,
                legacy_every_minutes: None,
            },
        };
        let created = task_store_create_task(&data_path, &input).expect("create task");

        let err = task_store_update_task(&data_path, &TaskUpdateInput {
            task_id: created.task_id.clone(),
            conversation_id: Some("conversation-a".to_string()),
            target_scope: Some(TASK_TARGET_SCOPE_DESKTOP.to_string()),
            goal: Some("定时任务".to_string()),
            why: Some(String::new()),
            todo: Some("保持定时".to_string()),
            trigger: Some(TaskTriggerInputLocal {
                run_at: Some("2026-04-11T10:00:00+08:00".to_string()),
                cron_expression: None,
                end_at: None,
                legacy_every_minutes: None,
            }),
        })
        .expect_err("recurring task cannot switch to one-time");
        assert!(err.contains("定时任务不能切换为一次性任务"));

        let fetched = task_store_get_task_record(&data_path, &created.task_id).expect("read task");
        assert_eq!(fetched.completion_state, TASK_STATE_ACTIVE);
        assert!(fetched.trigger.cron_expression.is_some());

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
    fn task_dispatch_conversation_should_prefer_bound_and_not_fallback_missing() {
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
            Some(side.id.as_str()),
        )
        .expect("resolve bound conversation")
        .expect("preferred conversation");
        assert_eq!(preferred.conversation_id, side.id);
        assert_eq!(preferred.target_scope, TASK_TARGET_SCOPE_DESKTOP);
        assert!(!preferred.system_task);

        let missing = task_resolve_dispatch_conversation(
            &state,
            Some("missing-conversation"),
        )
        .expect("resolve missing conversation");
        assert!(missing.is_none());

        let system = task_resolve_dispatch_conversation(
            &state,
            Some(SYSTEM_NOTIFICATION_CONVERSATION_ID),
        )
        .expect("resolve system task")
        .expect("system task");
        assert_eq!(system.conversation_id, SYSTEM_NOTIFICATION_CONVERSATION_ID);
        assert_eq!(system.target_scope, TASK_TARGET_SCOPE_DESKTOP);
        assert!(system.system_task);

        let _ = fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }

    #[test]
    fn task_dispatch_conversation_should_not_resolve_missing_contact_conversation() {
        let state = task_test_state("dispatch_missing_contact");
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
            Some("missing-contact-conversation"),
        )
        .expect("resolve missing contact conversation");

        assert!(resolved.is_none());

        let _ = fs::remove_dir_all(app_root_from_data_path(&state.data_path));
    }
