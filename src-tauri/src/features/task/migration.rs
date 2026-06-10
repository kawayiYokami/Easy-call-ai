fn task_store_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Prepare table info failed: {table}, {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("Read table info failed: {table}, {err}"))?;
    for row in rows {
        let name = row.map_err(|err| format!("Read table info row failed: {table}, {err}"))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn task_store_rename_column_if_needed(
    conn: &Connection,
    table: &str,
    legacy_column: &str,
    next_column: &str,
) -> Result<(), String> {
    if !task_store_has_column(conn, table, legacy_column)? || task_store_has_column(conn, table, next_column)? {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} RENAME COLUMN {legacy_column} TO {next_column}"),
        [],
    )
    .map_err(|err| {
        format!(
            "Rename task column failed: table={table}, from={legacy_column}, to={next_column}, {err}"
        )
    })?;
    Ok(())
}

fn task_store_add_column_if_missing(
    conn: &Connection,
    table: &str,
    definition: &str,
    column: &str,
) -> Result<(), String> {
    if task_store_has_column(conn, table, column)? {
        return Ok(());
    }
    conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])
        .map_err(|err| format!("Add task column failed: table={table}, column={column}, {err}"))?;
    Ok(())
}

fn task_store_apply_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|err| format!("Begin task migration transaction failed: {err}"))?;

    let migration_result = (|| -> Result<(), String> {
        task_store_add_column_if_missing(conn, "task_record", "conversation_id TEXT", "conversation_id")?;
        task_store_add_column_if_missing(conn, "task_record", "department_id TEXT", "department_id")?;
        task_store_add_column_if_missing(conn, "task_record", "agent_id TEXT", "agent_id")?;
        task_store_add_column_if_missing(
            conn,
            "task_record",
            "target_scope TEXT NOT NULL DEFAULT 'desktop'",
            "target_scope",
        )?;
        task_store_add_column_if_missing(conn, "task_record", "cron_expression TEXT", "cron_expression")?;
        task_store_rename_column_if_needed(conn, "task_record", "stage_updated_at", "stage_updated_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "run_at", "run_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "end_at", "end_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "created_at", "created_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "updated_at", "updated_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "last_triggered_at", "last_triggered_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_record", "completed_at", "completed_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_runtime_state", "updated_at", "updated_at_utc")?;
        task_store_rename_column_if_needed(conn, "task_run_log", "triggered_at", "triggered_at_utc")?;
        task_store_migrate_empty_conversation_id_to_system(conn)?;
        task_store_migrate_legacy_task_triggers(conn)?;
        task_store_migrate_triggered_one_time_tasks_to_completed(conn)?;
        Ok(())
    })();

    match migration_result {
        Ok(()) => conn
            .execute_batch("COMMIT;")
            .map_err(|err| format!("Commit task migration transaction failed: {err}"))?,
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(err);
        }
    }

    Ok(())
}

fn task_store_migrate_empty_conversation_id_to_system(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE task_record
         SET conversation_id = ?1
         WHERE conversation_id IS NULL OR TRIM(conversation_id) = ''",
        params![SYSTEM_NOTIFICATION_CONVERSATION_ID],
    )
    .map_err(|err| format!("Migrate empty task conversation_id failed: {err}"))?;
    Ok(())
}

fn task_store_migrate_triggered_one_time_tasks_to_completed(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE task_record
         SET completion_state = ?1,
             completed_at_utc = COALESCE(
                 NULLIF(TRIM(completed_at_utc), ''),
                 NULLIF(TRIM(last_triggered_at_utc), ''),
                 updated_at_utc,
                 created_at_utc
             ),
             updated_at_utc = COALESCE(
                 NULLIF(TRIM(last_triggered_at_utc), ''),
                 updated_at_utc,
                 created_at_utc
             )
         WHERE completion_state = ?2
           AND last_triggered_at_utc IS NOT NULL
           AND TRIM(last_triggered_at_utc) <> ''
           AND (
               run_at_utc IS NULL
               OR TRIM(run_at_utc) = ''
               OR TRIM(run_at_utc) <= TRIM(last_triggered_at_utc)
           )
           AND (cron_expression IS NULL OR TRIM(cron_expression) = '')
           AND every_minutes IS NULL",
        params![TASK_STATE_COMPLETED, TASK_STATE_ACTIVE],
    )
    .map_err(|err| format!("Migrate triggered one-time tasks failed: {err}"))?;
    Ok(())
}

fn task_store_migrate_legacy_task_triggers(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT task_id, run_at_utc, cron_expression, every_minutes, end_at_utc, last_triggered_at_utc, created_at_utc, updated_at_utc
             FROM task_record",
        )
        .map_err(|err| format!("Prepare legacy task trigger migration failed: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(|err| format!("Read legacy task trigger rows failed: {err}"))?;
    for row in rows {
        let (
            task_id,
            run_at_utc,
            cron_expression,
            every_minutes,
            end_at_utc,
            last_triggered_at_utc,
            created_at_utc,
            updated_at_utc,
        ) = row.map_err(|err| format!("Read legacy task trigger row failed: {err}"))?;
        let normalized_run_at_utc = run_at_utc
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or(last_triggered_at_utc.clone())
            .or(Some(created_at_utc.clone()))
            .or(Some(updated_at_utc.clone()));
        let normalized_cron_expression = cron_expression
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(task_normalize_cron_expression)
            .transpose()?
            .or_else(|| {
                every_minutes.and_then(|value| {
                    task_exact_cron_expression_from_legacy_every_minutes(
                        normalized_run_at_utc.as_deref(),
                        value,
                    )
                })
            });
        let next_trigger_kind = task_trigger_kind_from_fields(
            normalized_run_at_utc.as_deref(),
            normalized_cron_expression.as_deref(),
            every_minutes
                .and_then(task_legacy_every_minutes_normalized)
                .filter(|_| normalized_cron_expression.is_none()),
        );
        let should_clear_legacy_every_minutes =
            every_minutes.is_some() && normalized_cron_expression.is_some();
        let changed = normalized_run_at_utc.as_deref() != run_at_utc.as_deref()
            || normalized_cron_expression.as_deref() != cron_expression.as_deref()
            || should_clear_legacy_every_minutes;
        if !changed {
            continue;
        }
        conn.execute(
            "UPDATE task_record
             SET run_at_utc = ?2, cron_expression = ?3, trigger_kind = ?4, every_minutes = ?5, end_at_utc = ?6
             WHERE task_id = ?1",
            params![
                task_id,
                normalized_run_at_utc.as_deref(),
                normalized_cron_expression.as_deref(),
                next_trigger_kind,
                if should_clear_legacy_every_minutes {
                    None
                } else {
                    every_minutes.and_then(task_legacy_every_minutes_normalized)
                },
                end_at_utc.as_deref(),
            ],
        )
        .map_err(|err| format!("Migrate legacy task trigger failed: {err}"))?;
    }
    Ok(())
}
