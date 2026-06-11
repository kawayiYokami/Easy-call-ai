fn delegate_store_db_path(data_path: &PathBuf) -> PathBuf {
    app_root_from_data_path(data_path)
        .join("delegate")
        .join(DELEGATE_DB_FILE_NAME)
}

fn delegate_store_open(data_path: &PathBuf) -> Result<Connection, String> {
    let path = delegate_store_db_path(data_path);
    let parent = path
        .parent()
        .ok_or_else(|| "委托数据库路径缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|err| format!("创建委托目录失败: {err}"))?;
    let conn = Connection::open(&path)
        .map_err(|err| format!("打开委托数据库失败 ({}): {err}", path.display()))?;
    delegate_store_init(&conn)?;
    Ok(conn)
}

fn delegate_store_init(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS delegate_record (
            delegate_id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            parent_delegate_id TEXT,
            source_department_id TEXT NOT NULL,
            target_department_id TEXT NOT NULL,
            source_agent_id TEXT NOT NULL,
            target_agent_id TEXT NOT NULL,
            title TEXT NOT NULL,
            why TEXT NOT NULL,
            goal TEXT NOT NULL,
            todo TEXT NOT NULL,
            notify_assistant_when_done INTEGER NOT NULL,
            call_stack_json TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            delivered_at TEXT,
            completed_at TEXT
        );
        COMMIT;",
    )
    .map_err(|err| format!("初始化委托数据库失败: {err}"))?;
    delegate_store_migrate_why_goal_todo(conn)?;
    Ok(())
}

fn delegate_store_table_columns(conn: &Connection) -> Result<std::collections::HashSet<String>, String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(delegate_record)")
        .map_err(|err| format!("读取委托表结构失败: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("读取委托表字段失败: {err}"))?;
    let mut columns = std::collections::HashSet::new();
    for row in rows {
        columns.insert(row.map_err(|err| format!("读取委托表字段失败: {err}"))?);
    }
    Ok(columns)
}

fn delegate_store_sql_expr_for_column(
    columns: &std::collections::HashSet<String>,
    preferred: &str,
    legacy: &[&str],
) -> String {
    let mut exprs = Vec::<String>::new();
    if columns.contains(preferred) {
        exprs.push(format!("NULLIF({preferred}, '')"));
    }
    for column in legacy {
        if columns.contains(*column) {
            exprs.push(format!("NULLIF({column}, '')"));
        }
    }
    if exprs.is_empty() {
        "''".to_string()
    } else {
        format!("COALESCE({}, '')", exprs.join(", "))
    }
}

fn delegate_store_migrate_why_goal_todo(conn: &Connection) -> Result<(), String> {
    let columns = delegate_store_table_columns(conn)?;
    let has_new_columns =
        columns.contains("why") && columns.contains("goal") && columns.contains("todo");
    let has_legacy_columns = columns.contains("background")
        || columns.contains("instruction")
        || columns.contains("question")
        || columns.contains("specific_goal")
        || columns.contains("deliverable_requirement")
        || columns.contains("focus");
    if has_new_columns && !has_legacy_columns {
        return Ok(());
    }
    let why_expr = delegate_store_sql_expr_for_column(&columns, "why", &["background"]);
    let goal_expr = delegate_store_sql_expr_for_column(&columns, "goal", &["question", "instruction"]);
    let todo_expr =
        delegate_store_sql_expr_for_column(&columns, "todo", &["focus", "specific_goal", "deliverable_requirement"]);
    let sql = format!(
        "BEGIN;
        CREATE TABLE delegate_record_next (
            delegate_id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            parent_delegate_id TEXT,
            source_department_id TEXT NOT NULL,
            target_department_id TEXT NOT NULL,
            source_agent_id TEXT NOT NULL,
            target_agent_id TEXT NOT NULL,
            title TEXT NOT NULL,
            why TEXT NOT NULL,
            goal TEXT NOT NULL,
            todo TEXT NOT NULL,
            notify_assistant_when_done INTEGER NOT NULL,
            call_stack_json TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            delivered_at TEXT,
            completed_at TEXT
        );
        INSERT INTO delegate_record_next (
            delegate_id, kind, conversation_id, parent_delegate_id,
            source_department_id, target_department_id, source_agent_id, target_agent_id,
            title, why, goal, todo,
            notify_assistant_when_done, call_stack_json, status, created_at, updated_at, delivered_at, completed_at
        )
        SELECT
            delegate_id, kind, conversation_id, parent_delegate_id,
            source_department_id, target_department_id, source_agent_id, target_agent_id,
            title, {why_expr}, {goal_expr}, {todo_expr},
            notify_assistant_when_done, call_stack_json, status, created_at, updated_at, delivered_at, completed_at
        FROM delegate_record;
        DROP TABLE delegate_record;
        ALTER TABLE delegate_record_next RENAME TO delegate_record;
        COMMIT;"
    );
    conn.execute_batch(&sql)
        .map_err(|err| format!("迁移委托字段 why/goal/todo 失败: {err}"))?;
    eprintln!("[委托] 完成，任务=迁移字段why_goal_todo");
    Ok(())
}

fn delegate_call_stack_to_json(items: &[String]) -> Result<String, String> {
    serde_json::to_string(items).map_err(|err| format!("序列化委托调用栈失败: {err}"))
}

fn delegate_call_stack_from_json(raw: &str) -> Vec<String> {
    match serde_json::from_str(raw) {
        Ok(items) => items,
        Err(err) => {
            eprintln!(
                "[委托] 解析调用栈失败，raw={}, error={}",
                raw,
                err
            );
            Vec::new()
        }
    }
}

fn delegate_row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<DelegateEntry> {
    Ok(DelegateEntry {
        delegate_id: row.get("delegate_id")?,
        kind: row.get("kind")?,
        conversation_id: row.get("conversation_id")?,
        parent_delegate_id: row.get("parent_delegate_id")?,
        source_department_id: row.get("source_department_id")?,
        target_department_id: row.get("target_department_id")?,
        source_agent_id: row.get("source_agent_id")?,
        target_agent_id: row.get("target_agent_id")?,
        title: row.get("title")?,
        why: row.get("why")?,
        goal: row.get("goal")?,
        todo: row.get("todo")?,
        notify_assistant_when_done: row.get::<_, i64>("notify_assistant_when_done")? != 0,
        call_stack: delegate_call_stack_from_json(&row.get::<_, String>("call_stack_json")?),
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        delivered_at: row.get("delivered_at")?,
        completed_at: row.get("completed_at")?,
    })
}

fn delegate_store_create_delegate(
    data_path: &PathBuf,
    input: &DelegateCreateInput,
) -> Result<DelegateEntry, String> {
    if input.conversation_id.trim().is_empty() {
        return Err("delegate.conversationId 不能为空".to_string());
    }
    if input.source_department_id.trim().is_empty() || input.target_department_id.trim().is_empty() {
        return Err("委托 source/target department 不能为空".to_string());
    }
    if input.source_agent_id.trim().is_empty() || input.target_agent_id.trim().is_empty() {
        return Err("委托 source/target agent 不能为空".to_string());
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err("delegate.title 不能为空".to_string());
    }
    let goal = input.goal.trim();
    if goal.is_empty() {
        return Err("delegate.goal 不能为空".to_string());
    }
    let conn = delegate_store_open(data_path)?;
    let delegate_id = format!("delegate-{}", Uuid::new_v4());
    let now = now_iso();
    conn.execute(
        "INSERT INTO delegate_record (
            delegate_id, kind, conversation_id, parent_delegate_id,
            source_department_id, target_department_id, source_agent_id, target_agent_id,
            title, why, goal, todo,
            notify_assistant_when_done, call_stack_json, status, created_at, updated_at, delivered_at, completed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, NULL)",
        params![
            delegate_id,
            input.kind.trim(),
            input.conversation_id.trim(),
            input.parent_delegate_id.as_deref(),
            input.source_department_id.trim(),
            input.target_department_id.trim(),
            input.source_agent_id.trim(),
            input.target_agent_id.trim(),
            title,
            input.why.trim(),
            goal,
            input.todo.trim(),
            if input.notify_assistant_when_done { 1 } else { 0 },
            delegate_call_stack_to_json(&input.call_stack)?,
            DELEGATE_STATUS_DELIVERED,
            now,
            now,
            now,
        ],
    )
    .map_err(|err| format!("创建委托记录失败: {err}"))?;
    conn.query_row(
        "SELECT * FROM delegate_record WHERE delegate_id = ?1",
        params![delegate_id],
        delegate_row_to_entry,
    )
    .map_err(|err| format!("读取委托记录失败: {err}"))
}

fn delegate_store_get_delegate(data_path: &PathBuf, delegate_id: &str) -> Result<DelegateEntry, String> {
    let conn = delegate_store_open(data_path)?;
    conn.query_row(
        "SELECT * FROM delegate_record WHERE delegate_id = ?1",
        params![delegate_id.trim()],
        delegate_row_to_entry,
    )
    .map_err(|err| format!("读取委托记录失败: {err}"))
}

fn delegate_store_update_status(
    data_path: &PathBuf,
    delegate_id: &str,
    status: &str,
) -> Result<DelegateEntry, String> {
    let conn = delegate_store_open(data_path)?;
    let now = now_iso();
    let completed_at = if status == DELEGATE_STATUS_COMPLETED || status == DELEGATE_STATUS_FAILED {
        Some(now.clone())
    } else {
        None
    };
    let affected = conn.execute(
        "UPDATE delegate_record
         SET status = ?2, updated_at = ?3, completed_at = COALESCE(?4, completed_at)
         WHERE delegate_id = ?1",
        params![delegate_id.trim(), status.trim(), now, completed_at],
    )
    .map_err(|err| format!("更新委托状态失败: {err}"))?;
    if affected == 0 {
        return Err(format!(
            "更新委托状态失败：未找到委托 {}",
            delegate_id.trim()
        ));
    }
    eprintln!(
        "[委托] 更新状态成功，delegate_id={}, status={}, completed_at={}",
        delegate_id.trim(),
        status.trim(),
        completed_at.as_deref().unwrap_or("-")
    );
    delegate_store_get_delegate(data_path, delegate_id)
}

#[cfg(test)]
mod delegate_store_tests {
    use super::*;

    #[test]
    fn delegate_store_migrate_why_goal_todo_should_prefer_new_fields_and_drop_legacy_columns() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE delegate_record (
                delegate_id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                parent_delegate_id TEXT,
                source_department_id TEXT NOT NULL,
                target_department_id TEXT NOT NULL,
                source_agent_id TEXT NOT NULL,
                target_agent_id TEXT NOT NULL,
                title TEXT NOT NULL,
                why TEXT NOT NULL,
                goal TEXT NOT NULL,
                todo TEXT NOT NULL,
                background TEXT NOT NULL,
                question TEXT NOT NULL,
                focus TEXT NOT NULL,
                notify_assistant_when_done INTEGER NOT NULL,
                call_stack_json TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                delivered_at TEXT,
                completed_at TEXT
            );
            INSERT INTO delegate_record (
                delegate_id, kind, conversation_id, parent_delegate_id,
                source_department_id, target_department_id, source_agent_id, target_agent_id,
                title, why, goal, todo, background, question, focus,
                notify_assistant_when_done, call_stack_json, status, created_at, updated_at, delivered_at, completed_at
            ) VALUES (
                'delegate-a', 'delegate', 'conversation-a', NULL,
                'source-dept', 'target-dept', 'source-agent', 'target-agent',
                'title-a', 'new why', 'new goal', '', 'old why', 'old goal', 'old todo',
                0, '[]', 'delivered', '2026-06-11T00:00:00Z', '2026-06-11T00:00:00Z', NULL, NULL
            );",
        )
        .expect("seed legacy delegate table");

        delegate_store_migrate_why_goal_todo(&conn).expect("migrate delegate fields");

        let columns = delegate_store_table_columns(&conn).expect("read migrated columns");
        assert!(columns.contains("why"));
        assert!(columns.contains("goal"));
        assert!(columns.contains("todo"));
        assert!(!columns.contains("background"));
        assert!(!columns.contains("question"));
        assert!(!columns.contains("focus"));

        let (why, goal, todo): (String, String, String) = conn
            .query_row(
                "SELECT why, goal, todo FROM delegate_record WHERE delegate_id = 'delegate-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read migrated row");
        assert_eq!(why, "new why");
        assert_eq!(goal, "new goal");
        assert_eq!(todo, "old todo");
    }
}
