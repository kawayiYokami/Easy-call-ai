#[derive(Debug, Clone)]
struct RuntimeOrganizationSnapshot {
    config: AppConfig,
    agents: Vec<AgentProfile>,
    departments_by_id: std::collections::HashMap<String, DepartmentConfig>,
    department_ids_by_agent: std::collections::HashMap<String, String>,
}

fn normalize_runtime_organization_department_children(config: &mut AppConfig) {
    let valid_department_ids = config
        .departments
        .iter()
        .map(|department| department.id.trim().to_string())
        .filter(|department_id| !department_id.is_empty())
        .collect::<std::collections::HashSet<_>>();
    for department in &mut config.departments {
        department.child_department_ids = normalize_department_child_ids(
            &department.child_department_ids,
            &department.id,
        )
        .into_iter()
        .filter(|child_id| valid_department_ids.contains(child_id))
        .collect::<Vec<_>>();
    }
    let removed = remove_cyclic_department_child_ids(&mut config.departments);
    if !removed.is_empty() {
        let edges = removed
            .iter()
            .map(|(parent_id, child_id)| format!("{parent_id}->{child_id}"))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "[运行组织] 跳过成环部门关系: count={}, edges={}",
            removed.len(),
            edges
        );
    }
}

fn build_runtime_organization_snapshot_from_parts(
    data_path: &PathBuf,
    base_config: &AppConfig,
    base_agents: &[AgentProfile],
) -> Result<RuntimeOrganizationSnapshot, String> {
    let mut config = base_config.clone();
    let mut runtime_data = AppData::default();
    runtime_data.agents = base_agents.to_vec();
    merge_private_organization_into_runtime_data(data_path, &mut config, &mut runtime_data)?;
    normalize_runtime_organization_department_children(&mut config);

    let mut departments_by_id = std::collections::HashMap::<String, DepartmentConfig>::new();
    let mut department_ids_by_agent = std::collections::HashMap::<String, String>::new();
    for department in &config.departments {
        let department_id = department.id.trim();
        if department_id.is_empty() {
            continue;
        }
        departments_by_id.insert(department_id.to_string(), department.clone());
        for agent_id in &department.agent_ids {
            let agent_id = agent_id.trim();
            if agent_id.is_empty() {
                continue;
            }
            department_ids_by_agent
                .entry(agent_id.to_string())
                .or_insert_with(|| department_id.to_string());
        }
    }

    Ok(RuntimeOrganizationSnapshot {
        config,
        agents: runtime_data.agents,
        departments_by_id,
        department_ids_by_agent,
    })
}

fn load_runtime_organization_snapshot(
    state: &AppState,
) -> Result<RuntimeOrganizationSnapshot, String> {
    let config = state_read_config_cached(state)?;
    let agents = state_read_agents_cached(state)?;
    build_runtime_organization_snapshot_from_parts(&state.data_path, &config, &agents)
}

fn runtime_department_by_id<'a>(
    snapshot: &'a RuntimeOrganizationSnapshot,
    department_id: &str,
) -> Option<&'a DepartmentConfig> {
    let department_id = department_id.trim();
    if department_id.is_empty() {
        return None;
    }
    snapshot.departments_by_id.get(department_id)
}

fn runtime_department_for_agent<'a>(
    snapshot: &'a RuntimeOrganizationSnapshot,
    agent_id: &str,
) -> Option<&'a DepartmentConfig> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        return None;
    }
    snapshot
        .department_ids_by_agent
        .get(agent_id)
        .and_then(|department_id| runtime_department_by_id(snapshot, department_id))
        .or_else(|| {
            if agent_id == DEFAULT_AGENT_ID {
                runtime_department_by_id(snapshot, ASSISTANT_DEPARTMENT_ID)
            } else {
                None
            }
        })
}
