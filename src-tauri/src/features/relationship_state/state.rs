pub fn read_relationship_from_value(value: Option<&serde_json::Value>) -> RelationshipStateRoot {
    value
        .and_then(|v| serde_json::from_value::<RelationshipStateRoot>(v.clone()).ok())
        .unwrap_or_default()
}

pub fn write_relationship_to_value(state: &RelationshipStateRoot) -> serde_json::Value {
    serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({ "version": 1, "byAgent": {} }))
}

pub fn agent_state_mut<'a>(
    root: &'a mut RelationshipStateRoot,
    agent_id: &str,
) -> &'a mut AgentRelationshipState {
    let key = normalized_agent_id(agent_id);
    root.by_agent.entry(key).or_default()
}

pub fn agent_state<'a>(
    root: &'a RelationshipStateRoot,
    agent_id: &str,
) -> AgentRelationshipState {
    root.by_agent
        .get(&normalized_agent_id(agent_id))
        .cloned()
        .unwrap_or_default()
}

fn normalized_agent_id(agent_id: &str) -> String {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() {
        "default_agent".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let secs = dur.as_secs();
        let days = secs / 86400;
        let rest = secs % 86400;
        let h = rest / 3600;
        let m = (rest % 3600) / 60;
        let s = rest % 60;
        format!("unix_day_{}:{:02}:{:02}:{:02}Z", days, h, m, s)
    } else {
        "unknown".to_string()
    }
}
