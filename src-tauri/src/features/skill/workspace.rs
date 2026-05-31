use super::*;

fn hidden_skill_summaries_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, Vec<SkillSummaryItem>>> {
    static CACHE: OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, Vec<SkillSummaryItem>>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn hidden_skill_cache_scope_key(state: &AppState) -> String {
    state.data_path.display().to_string()
}

fn llm_workspace_skills_root_at(workspace_root: &Path) -> PathBuf {
    workspace_root.join("skills")
}

fn llm_workspace_skills_root(state: &AppState) -> Result<PathBuf, String> {
    Ok(llm_workspace_skills_root_at(&configured_workspace_root_path(state)?))
}

fn sync_skill_template_file(path: &PathBuf, content: &str) -> Result<(), String> {
    if path.exists() {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() == content.len() as u64 {
                return Ok(());
            }
        }
    }
    fs::write(path, content).map_err(|err| format!("Write file failed ({}): {err}", path.display()))
}

fn sync_workspace_preset_skill(
    skills_root: &PathBuf,
    skill_dir_name: &str,
    skill_md: &str,
) -> Result<(), String> {
    let dir = skills_root.join(skill_dir_name);
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Create preset skill dir failed ({}): {err}", dir.display()))?;
    sync_skill_template_file(&dir.join("SKILL.md"), skill_md)
}

pub(crate) fn ensure_workspace_skills_layout(state: &AppState) -> Result<(), String> {
    let workspace_root = ensure_workspace_root_ready(&configured_workspace_root_path(state)?)?;
    ensure_workspace_skills_layout_at_root(&workspace_root)
}

pub(crate) fn ensure_workspace_skills_layout_at_root(workspace_root: &Path) -> Result<(), String> {
    let skills_root = llm_workspace_skills_root_at(&workspace_root);
    fs::create_dir_all(&skills_root)
        .map_err(|err| format!("Create skills dir failed ({}): {err}", skills_root.display()))?;

    let legacy_readme = skills_root.join("README.md");
    if legacy_readme.exists() {
        let _ = fs::remove_file(&legacy_readme);
    }

    sync_workspace_preset_skill(
        &skills_root,
        "browser-automation",
        include_str!("../../../resources/preset-skills/browser-automation/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "news-analyst",
        include_str!("../../../resources/preset-skills/news-analyst/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "agent-office",
        include_str!("../../../resources/preset-skills/agent-office/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "agents-md-setup",
        include_str!("../../../resources/preset-skills/agents-md-setup/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "assistant-interaction-guide",
        include_str!("../../../resources/preset-skills/assistant-interaction-guide/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "skill-setup",
        include_str!("../../../resources/preset-skills/skill-setup/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "mcp-setup",
        include_str!("../../../resources/preset-skills/mcp-setup/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "workspace-guide",
        include_str!("../../../resources/preset-skills/workspace-guide/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "private-organization-guide",
        include_str!("../../../resources/preset-skills/private-organization-guide/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "pai-guide",
        include_str!("../../../resources/preset-skills/pai-guide/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "code-review",
        include_str!("../../../resources/preset-skills/code-review/SKILL.md"),
    )?;
    sync_workspace_preset_skill(
        &skills_root,
        "memory-generation",
        include_str!("../../../resources/preset-skills/memory-generation/SKILL.md"),
    )?;

    Ok(())
}

fn parse_skill_file(skill_md_path: &PathBuf) -> Result<(String, String, String), String> {
    let content = fs::read_to_string(skill_md_path)
        .map_err(|err| format!("Read SKILL.md failed ({}): {err}", skill_md_path.display()))?;
    let mut lines = content.lines();
    let first = lines
        .next()
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string();
    if first != "---" {
        return Err("SKILL.md must start with YAML frontmatter".to_string());
    }
    let mut name = String::new();
    let mut description = String::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        let Some((key, raw_value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let mut value = raw_value.trim().to_string();
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            let first = bytes[0];
            let last = bytes[value.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                value = value[1..value.len() - 1].to_string();
            }
        }
        if key == "name" {
            name = value;
        } else if key == "description" {
            description = value;
        }
    }
    if name.trim().is_empty() {
        name = skill_md_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|v| v.to_str())
            .unwrap_or("skill")
            .to_string();
    }
    let body = if let Some((_, rest)) = content.split_once("\n---") {
        rest.trim_start_matches(['\r', '\n']).trim().to_string()
    } else {
        String::new()
    };
    Ok((name, description, body))
}

pub(crate) fn load_workspace_skill_summaries_with_errors(
    state: &AppState,
) -> Result<(Vec<SkillSummaryItem>, Vec<WorkspaceLoadError>), String> {
    ensure_workspace_skills_layout(state)?;
    let mut skills = Vec::<SkillSummaryItem>::new();
    let mut errors = Vec::<WorkspaceLoadError>::new();
    let skills_dir = llm_workspace_skills_root(state)?;
    let mut dirs = fs::read_dir(&skills_dir)
        .map_err(|err| format!("Read skills dir failed ({}): {err}", skills_dir.display()))?
        .filter_map(|entry| entry.ok().map(|v| v.path()))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    for dir in dirs {
        let skill_md = dir.join("SKILL.md");
        if !skill_md.is_file() {
            errors.push(WorkspaceLoadError {
                item: dir.to_string_lossy().to_string(),
                error: "SKILL.md not found".to_string(),
            });
            continue;
        }
        match parse_skill_file(&skill_md) {
            Ok((name, description, content)) => {
                skills.push(SkillSummaryItem {
                    name,
                    description,
                    content,
                    path: skill_md.to_string_lossy().to_string(),
                });
            }
            Err(err) => errors.push(WorkspaceLoadError {
                item: skill_md.to_string_lossy().to_string(),
                error: err,
            }),
        }
    }
    Ok((skills, errors))
}

pub(crate) fn render_skill_summary(skills: &[SkillSummaryItem]) -> String {
    if skills.is_empty() {
        return "No skills found in current skills directory.".to_string();
    }
    let mut lines = Vec::<String>::new();
    for item in skills {
        let desc = if item.description.trim().is_empty() {
            "(no description)"
        } else {
            item.description.trim()
        };
        lines.push(format!("- {}: {}", item.name.trim(), desc));
    }
    lines.join("\n")
}

fn filter_skills_for_department(
    department: Option<&DepartmentConfig>,
    skills: &[SkillSummaryItem],
) -> Vec<SkillSummaryItem> {
    skills
        .iter()
        .filter(|item| {
            department_permission_allows_any_name(
                department,
                DepartmentPermissionCategory::Skill,
                &[item.name.as_str()],
            )
        })
        .cloned()
        .collect()
}

fn render_hidden_skill_snapshot_block(
    state: &AppState,
    skills: &[SkillSummaryItem],
    scan_error: Option<&str>,
) -> String {
    let skills_root_path = llm_workspace_skills_root(state)
        .unwrap_or_else(|_| state.llm_workspace_path.join("skills"));
    let skills_root = skills_root_path.to_string_lossy();
    if let Some(err) = scan_error {
        return prompt_xml_block(
            "skill usage",
            format!(
                "System skill directory path: {}\nscan failed: {}",
                skills_root, err
            ),
        );
    }
    let example_path = skills
        .iter()
        .find(|item| item.name.trim().eq_ignore_ascii_case("workspace-guide"))
        .map(|item| item.path.trim().to_string())
        .unwrap_or_else(|| {
            skills_root_path
                .join("workspace-guide")
                .join("SKILL.md")
                .to_string_lossy()
                .to_string()
        });
    let summary = render_skill_summary(skills);
    format!(
        "{}\n\n{}",
        prompt_xml_block(
            "skill usage",
            format!(
                "System skill directory path: {}\n\nhow to read skill:\nexample:\nworkspace-guide\nread this path: {}",
                skills_root, example_path
            ),
        ),
        prompt_xml_block("skill index", summary)
    )
}

pub(crate) fn update_hidden_skill_snapshot_cache(
    state: &AppState,
    skills: &[SkillSummaryItem],
    scan_error: Option<&str>,
) -> Result<String, String> {
    let snapshot = render_hidden_skill_snapshot_block(state, skills, scan_error);
    let mut guard = state
        .hidden_skill_snapshot_cache
        .lock()
        .map_err(|_| "Failed to lock hidden skill snapshot cache".to_string())?;
    *guard = snapshot.clone();
    drop(guard);

    let mut summaries_guard = hidden_skill_summaries_cache()
        .lock()
        .map_err(|_| "Failed to lock hidden skill summaries cache".to_string())?;
    summaries_guard.insert(hidden_skill_cache_scope_key(state), skills.to_vec());
    Ok(snapshot)
}

pub(crate) fn clear_hidden_skill_snapshot_cache(state: &AppState) -> Result<(), String> {
    let mut guard = state
        .hidden_skill_snapshot_cache
        .lock()
        .map_err(|_| "Failed to lock hidden skill snapshot cache".to_string())?;
    *guard = String::new();
    drop(guard);

    let mut summaries_guard = hidden_skill_summaries_cache()
        .lock()
        .map_err(|_| "Failed to lock hidden skill summaries cache".to_string())?;
    summaries_guard.remove(&hidden_skill_cache_scope_key(state));
    Ok(())
}

pub(crate) fn build_hidden_skill_snapshot_block(state: &AppState) -> String {
    match state.hidden_skill_snapshot_cache.lock() {
        Ok(guard) if !guard.trim().is_empty() => guard.clone(),
        _ => String::new(),
    }
}

pub(crate) fn build_hidden_skill_snapshot_block_for_department(
    state: &AppState,
    department: Option<&DepartmentConfig>,
) -> String {
    if department
        .map(|item| !item.permission_control.enabled)
        .unwrap_or(true)
    {
        return build_hidden_skill_snapshot_block(state);
    }
    let cache_key = hidden_skill_cache_scope_key(state);
    let cached_skills = hidden_skill_summaries_cache()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&cache_key).cloned());
    match cached_skills {
        Some(skills) => {
            let filtered = filter_skills_for_department(department, &skills);
            render_hidden_skill_snapshot_block(state, &filtered, None)
        }
        None => {
            runtime_log_warn(
                "[技能工作区] 隐藏技能快照未命中结构化缓存，返回现有快照文本；如需更新请显式刷新技能工作区。"
                    .to_string(),
            );
            build_hidden_skill_snapshot_block(state)
        }
    }
}

fn format_workspace_named_item(name: &str, id: &str) -> String {
    let trimmed_name = name.trim();
    let trimmed_id = id.trim();
    if trimmed_name.is_empty() {
        return trimmed_id.to_string();
    }
    if trimmed_id.is_empty() || trimmed_name == trimmed_id {
        return trimmed_name.to_string();
    }
    format!("{trimmed_name} ({trimmed_id})")
}

fn build_workspace_loaded_groups(
    servers: &[McpServerConfig],
    skills: &[SkillSummaryItem],
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    private_agent_ids: &[String],
    private_department_ids: &[String],
) -> Vec<WorkspaceLoadedGroup> {
    let mcp_items = servers
        .iter()
        .map(|server| format_workspace_named_item(&server.name, &server.id))
        .collect::<Vec<_>>();
    let skill_items = skills
        .iter()
        .map(|item| item.name.trim().to_string())
        .collect::<Vec<_>>();
    let private_agent_items = private_agent_ids
        .iter()
        .map(|id| {
            agents
                .iter()
                .find(|agent| agent.id == *id)
                .map(|agent| format_workspace_named_item(&agent.name, &agent.id))
                .unwrap_or_else(|| id.clone())
        })
        .collect::<Vec<_>>();
    let private_department_items = private_department_ids
        .iter()
        .map(|id| {
            departments
                .iter()
                .find(|department| department.id == *id)
                .map(|department| format_workspace_named_item(&department.name, &department.id))
                .unwrap_or_else(|| id.clone())
        })
        .collect::<Vec<_>>();
    vec![
        WorkspaceLoadedGroup {
            kind: "mcp".to_string(),
            label: "MCP".to_string(),
            count: mcp_items.len(),
            items: mcp_items,
        },
        WorkspaceLoadedGroup {
            kind: "skill".to_string(),
            label: "SKILL".to_string(),
            count: skill_items.len(),
            items: skill_items,
        },
        WorkspaceLoadedGroup {
            kind: "private_agent".to_string(),
            label: "私有人格".to_string(),
            count: private_agent_items.len(),
            items: private_agent_items,
        },
        WorkspaceLoadedGroup {
            kind: "private_department".to_string(),
            label: "私有部门".to_string(),
            count: private_department_items.len(),
            items: private_department_items,
        },
    ]
}

fn build_workspace_failed_groups(
    mcp_failed: &[WorkspaceLoadError],
    skills_failed: &[WorkspaceLoadError],
    private_agents_failed: &[WorkspaceLoadError],
    private_departments_failed: &[WorkspaceLoadError],
) -> Vec<WorkspaceFailedGroup> {
    vec![
        WorkspaceFailedGroup {
            kind: "mcp".to_string(),
            label: "MCP".to_string(),
            count: mcp_failed.len(),
            items: mcp_failed.to_vec(),
        },
        WorkspaceFailedGroup {
            kind: "skill".to_string(),
            label: "SKILL".to_string(),
            count: skills_failed.len(),
            items: skills_failed.to_vec(),
        },
        WorkspaceFailedGroup {
            kind: "private_agent".to_string(),
            label: "私有人格".to_string(),
            count: private_agents_failed.len(),
            items: private_agents_failed.to_vec(),
        },
        WorkspaceFailedGroup {
            kind: "private_department".to_string(),
            label: "私有部门".to_string(),
            count: private_departments_failed.len(),
            items: private_departments_failed.to_vec(),
        },
    ]
}

fn summarize_workspace_loaded_groups(groups: &[WorkspaceLoadedGroup]) -> String {
    let mut lines = Vec::<String>::new();
    for group in groups {
        let details = if group.items.is_empty() {
            "无".to_string()
        } else {
            group.items.join("、")
        };
        lines.push(format!(
            "{}：成功加载 {} 个；{}",
            group.label, group.count, details
        ));
    }
    lines.join("\n")
}

fn summarize_workspace_failed_groups(groups: &[WorkspaceFailedGroup]) -> String {
    let mut lines = Vec::<String>::new();
    for group in groups {
        if group.items.is_empty() {
            lines.push(format!("{}：0 个加载失败", group.label));
            continue;
        }
        lines.push(format!("{}：{} 个加载失败", group.label, group.count));
        for item in &group.items {
            lines.push(format!("- {} | {}", item.item, item.error));
        }
    }
    lines.join("\n")
}

fn finalize_workspace_load_result(
    mut result: RefreshMcpAndSkillsResult,
    servers: &[McpServerConfig],
    merged_agents: &[AgentProfile],
    merged_departments: &[DepartmentConfig],
) -> RefreshMcpAndSkillsResult {
    let loaded_groups = build_workspace_loaded_groups(
        servers,
        &result.skills,
        merged_agents,
        merged_departments,
        &result.private_agents_loaded,
        &result.private_departments_loaded,
    );
    let failed_groups = build_workspace_failed_groups(
        &result.mcp_failed,
        &result.skills_failed,
        &result.private_agents_failed,
        &result.private_departments_failed,
    );
    let total_loaded = loaded_groups.iter().map(|group| group.count).sum::<usize>();
    let total_failed = failed_groups.iter().map(|group| group.count).sum::<usize>();
    let loaded_summary = summarize_workspace_loaded_groups(&loaded_groups);
    let failed_summary = summarize_workspace_failed_groups(&failed_groups);
    result.loaded_groups = loaded_groups;
    result.failed_groups = failed_groups;
    result.total_loaded = total_loaded;
    result.total_failed = total_failed;
    result.loaded_summary = loaded_summary;
    result.failed_summary = failed_summary;
    result.needs_repair = total_failed > 0;
    result
}

fn collect_workspace_load_snapshot(
    state: &AppState,
) -> Result<(RefreshMcpAndSkillsResult, Vec<McpServerConfig>, Vec<AgentProfile>, Vec<DepartmentConfig>), String> {
    ensure_workspace_mcp_layout(state)?;
    ensure_workspace_skills_layout(state)?;
    ensure_workspace_private_organization_layout(state)?;
    let (servers, mcp_errors) = load_workspace_mcp_servers_with_errors(state)?;
    let (skills, skill_errors) = load_workspace_skill_summaries_with_errors(state)?;
    if let Err(err) = update_hidden_skill_snapshot_cache(state, &skills, None) {
        runtime_log_error(format!(
            "[技能工作区] 更新隐藏技能快照缓存失败: skills={}, error={}",
            skills.len(),
            err
        ));
    }
    let mut config = read_config(&state.config_path)?;
    let mut data = read_app_data(&state.data_path)?;
    let private_org = merge_private_organization_into_runtime(&state.data_path, &mut config, &mut data.agents)?;
    let mcp_loaded = servers.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
    let skills_loaded = skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    let skill_summary = render_skill_summary(&skills);
    let result = RefreshMcpAndSkillsResult {
        mcp_loaded,
        mcp_failed: mcp_errors
            .into_iter()
            .map(|v| WorkspaceLoadError {
                item: v.item,
                error: v.error,
            })
            .collect(),
        skills_loaded,
        skills_failed: skill_errors,
        skills: skills.clone(),
        skill_summary,
        private_agents_loaded: private_org.private_agents_loaded,
        private_agents_failed: private_org.private_agents_failed,
        private_departments_loaded: private_org.private_departments_loaded,
        private_departments_failed: private_org.private_departments_failed,
        loaded_groups: Vec::new(),
        failed_groups: Vec::new(),
        total_loaded: 0,
        total_failed: 0,
        loaded_summary: String::new(),
        failed_summary: String::new(),
        needs_repair: false,
    };
    Ok((result, servers, data.agents, config.departments))
}

async fn disconnect_workspace_mcp_runtime_clients(state: &AppState) {
    match load_workspace_mcp_servers(state) {
        Ok(servers) => {
            for server in servers {
                mcp_disconnect_cached_client(&server.id).await;
                mcp_runtime_state_set(&server.id, false, "stopped", "", Vec::new());
            }
        }
        Err(err) => runtime_log_warn(format!(
            "[工作区加载] reload 前清理 MCP 运行态失败，继续尝试重新加载：{}",
            err
        )),
    }
}

pub(crate) async fn load_workspace(state: &AppState) -> Result<RefreshMcpAndSkillsResult, String> {
    let (mut result, servers, merged_agents, merged_departments) =
        collect_workspace_load_snapshot(state)?;
    match mcp_start_supervisor_probe_all_from_policy(state.clone(), "workspace_load") {
        Ok(()) => {}
        Err(err) => {
            result.mcp_failed.push(WorkspaceLoadError {
                item: "mcp_supervisor_start".to_string(),
                error: err,
            });
        }
    }
    refresh_global_tool_schema_cache(state);
    mark_prompt_cache_rebuild_for_all_final_system_sources(state);
    Ok(finalize_workspace_load_result(
        result,
        &servers,
        &merged_agents,
        &merged_departments,
    ))
}

pub(crate) async fn reload_workspace(
    state: &AppState,
) -> Result<RefreshMcpAndSkillsResult, String> {
    disconnect_workspace_mcp_runtime_clients(state).await;
    clear_global_tool_schema_cache();
    if let Err(err) = clear_hidden_skill_snapshot_cache(state) {
        runtime_log_warn(format!("[工作区加载] reload 前清空技能快照缓存失败：{}", err));
    }
    load_workspace(state).await
}

pub(crate) fn log_workspace_load_result(prefix: &str, result: &RefreshMcpAndSkillsResult) {
    runtime_log_info(format!(
        "{} 状态=完成，成功加载={}，加载失败={}，需修复={}",
        prefix, result.total_loaded, result.total_failed, result.needs_repair
    ));
    if !result.loaded_summary.trim().is_empty() {
        for line in result.loaded_summary.lines() {
            runtime_log_info(format!("{} {}", prefix, line));
        }
    }
    if !result.failed_summary.trim().is_empty() {
        for line in result.failed_summary.lines() {
            runtime_log_info(format!("{} {}", prefix, line));
        }
    }
}

pub(crate) fn open_skills_workspace_dir(state: &AppState) -> Result<String, String> {
    ensure_workspace_skills_layout(state)?;
    let path = llm_workspace_skills_root(state)?;
    open_path_in_file_manager(&path)?;
    Ok(path.to_string_lossy().to_string())
}
