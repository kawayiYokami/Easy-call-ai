static LAST_SNAPSHOT: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, RelationshipPanelSnapshot>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

static RULES_CACHE: std::sync::LazyLock<std::sync::Mutex<Option<RelationshipRules>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

pub fn relationship_data_dir(data_path: Option<&str>) -> Result<String, String> {
    let Some(path) = data_path.and_then(|p| {
        let trimmed = p.trim();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }) else {
        return Err("relationship data path is required".to_string());
    };
    Ok(format!("{}/relationship_state", path.trim_end_matches('/').trim_end_matches('\\')))
}

fn relationship_rules_path(data_path: &str) -> std::path::PathBuf {
    [data_path, "relationship_rules.json"].iter().collect()
}

pub async fn load_relationship_rules(data_path: &str) -> Result<RelationshipRules, String> {
    {
        let cache = RULES_CACHE.lock().map_err(|e| format!("锁失败: {}", e))?;
        if let Some(rules) = cache.as_ref() {
            return Ok(rules.clone());
        }
    }

    let path = relationship_rules_path(data_path);
    if tokio::fs::metadata(&path).await.is_err() {
        write_default_relationship_rules(data_path).await?;
    }
    let text = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("读取 relationship_rules.json 失败: {}", e))?;
    let rules: RelationshipRules = serde_json::from_str(&text)
        .map_err(|e| format!("解析 relationship_rules.json 失败: {}", e))?;
    let mut cache = RULES_CACHE.lock().map_err(|e| format!("锁失败: {}", e))?;
    *cache = Some(rules.clone());
    Ok(rules)
}

async fn write_default_relationship_rules(data_path: &str) -> Result<(), String> {
    let path = relationship_rules_path(data_path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| format!("创建 relationship_state 目录失败: {}", e))?;
    }
    let text = serde_json::to_string_pretty(&RelationshipRules::default())
        .map_err(|e| format!("序列化默认 relationship_rules.json 失败: {}", e))?;
    tokio::fs::write(&path, text).await.map_err(|e| format!("写入 relationship_rules.json 失败: {}", e))
}

pub fn invalidate_relationship_rules_cache() {
    if let Ok(mut cache) = RULES_CACHE.lock() {
        *cache = None;
    }
}

pub fn build_relationship_state_block_for_agent(
    relationship_state_value: Option<&serde_json::Value>,
    agent_id: &str,
) -> String {
    let root = read_relationship_from_value(relationship_state_value);
    let state = agent_state(&root, agent_id);
    build_relationship_state_block(&state)
}

pub fn apply_interaction_event_with_rules(
    relationship_state: &mut Option<serde_json::Value>,
    conversation_id: &str,
    agent_id: &str,
    event: InteractionEvent,
    rules: &RelationshipRules,
) {
    let mut root = read_relationship_from_value(relationship_state.as_ref());
    let state = agent_state_mut(&mut root, agent_id);
    reduce_relationship_state(state, event, rules);
    let snapshot = build_panel_snapshot(agent_id, state, rules);
    if let Ok(mut cache) = LAST_SNAPSHOT.lock() {
        cache.insert(snapshot_key(conversation_id, agent_id), snapshot);
    }
    *relationship_state = Some(write_relationship_to_value(&root));
}

fn build_panel_snapshot(
    agent_id: &str,
    state: &AgentRelationshipState,
    rules: &RelationshipRules,
) -> RelationshipPanelSnapshot {
    RelationshipPanelSnapshot {
        agent_id: agent_id.to_string(),
        dimensions: state.dimensions.clone(),
        last_event: state.last_event.clone(),
        recent_events: state.recent_events.clone(),
        relationship_block: build_relationship_state_block(state),
        raw_json: serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({})),
        rules: rules.clone(),
    }
}

fn snapshot_key(conversation_id: &str, agent_id: &str) -> String {
    format!("{}::{}", conversation_id.trim(), agent_id.trim())
}

#[tauri::command]
pub async fn get_relationship_panel_snapshot(
    conversation_id: String,
    agent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<RelationshipPanelSnapshot, String> {
    let agent_id = agent_id.unwrap_or_else(|| "default_agent".to_string());
    if let Ok(cache) = LAST_SNAPSHOT.lock() {
        if let Some(snapshot) = cache.get(&snapshot_key(&conversation_id, &agent_id)) {
            return Ok(snapshot.clone());
        }
    }
    let rules = load_relationship_rules(&relationship_data_dir(Some(&state.data_path.to_string_lossy()))?).await?;
    Ok(build_panel_snapshot(&agent_id, &AgentRelationshipState::default(), &rules))
}

#[tauri::command]
pub async fn preview_relationship_block(agent_id: Option<String>, state: State<'_, AppState>) -> Result<String, String> {
    let agent_id = agent_id.unwrap_or_else(|| "default_agent".to_string());
    let rules = load_relationship_rules(&relationship_data_dir(Some(&state.data_path.to_string_lossy()))?).await?;
    Ok(build_panel_snapshot(&agent_id, &AgentRelationshipState::default(), &rules).relationship_block)
}

#[tauri::command]
pub async fn refresh_relationship_rules(state: State<'_, AppState>) -> Result<RelationshipRules, String> {
    invalidate_relationship_rules_cache();
    load_relationship_rules(&relationship_data_dir(Some(&state.data_path.to_string_lossy()))?).await
}

#[tauri::command]
pub async fn reset_relationship_state(
    conversation_id: String,
    agent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<RelationshipPanelSnapshot, String> {
    let agent_id = agent_id.unwrap_or_else(|| "default_agent".to_string());
    let rules = load_relationship_rules(&relationship_data_dir(Some(&state.data_path.to_string_lossy()))?).await?;
    let snapshot = build_panel_snapshot(&agent_id, &AgentRelationshipState::default(), &rules);
    if let Ok(mut cache) = LAST_SNAPSHOT.lock() {
        cache.insert(snapshot_key(&conversation_id, &agent_id), snapshot.clone());
    }
    Ok(snapshot)
}

#[tauri::command]
pub async fn simulate_relationship_event(
    conversation_id: String,
    agent_id: Option<String>,
    event_type: String,
    intensity: f64,
    state: State<'_, AppState>,
) -> Result<RelationshipPanelSnapshot, String> {
    let agent_id = agent_id.unwrap_or_else(|| "default_agent".to_string());
    let rules = load_relationship_rules(&relationship_data_dir(Some(&state.data_path.to_string_lossy()))?).await?;
    let mut relationship = if let Ok(cache) = LAST_SNAPSHOT.lock() {
        cache
            .get(&snapshot_key(&conversation_id, &agent_id))
            .and_then(|snapshot| serde_json::from_value::<AgentRelationshipState>(snapshot.raw_json.clone()).ok())
            .unwrap_or_default()
    } else {
        AgentRelationshipState::default()
    };
    let event = InteractionEvent {
        event_type,
        intensity,
        confidence: 1.0,
        valence: 0.0,
        reason: "developer simulated event".to_string(),
        suggested_delta: StateDelta::default(),
        applied_delta: StateDelta::default(),
        created_at: String::new(),
    };
    reduce_relationship_state(&mut relationship, event, &rules);
    let snapshot = build_panel_snapshot(&agent_id, &relationship, &rules);
    if let Ok(mut cache) = LAST_SNAPSHOT.lock() {
        cache.insert(snapshot_key(&conversation_id, &agent_id), snapshot.clone());
    }
    Ok(snapshot)
}

#[cfg(test)]
mod relationship_rules_tests {
    use super::*;

    #[tokio::test]
    async fn load_relationship_rules_should_create_default_file() {
        let root = std::env::temp_dir().join(format!(
            "relationship-rules-{}",
            Uuid::new_v4()
        ));
        let data_dir = root.to_string_lossy().to_string();
        let rules = load_relationship_rules(&data_dir).await.expect("load rules");
        assert!(relationship_rules_path(&data_dir).exists());
        assert!(rules.event_impacts.contains_key("gratitude"));
        let _ = std::fs::remove_dir_all(root);
    }
}
