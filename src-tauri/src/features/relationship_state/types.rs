#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipStateRoot {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub by_agent: std::collections::HashMap<String, AgentRelationshipState>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRelationshipState {
    #[serde(default)]
    pub dimensions: RelationshipDimensions,
    #[serde(default)]
    pub last_event: Option<InteractionEvent>,
    #[serde(default)]
    pub recent_events: Vec<InteractionEvent>,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub turn_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDimensions {
    #[serde(default = "default_affection")]
    pub affection: i32,
    #[serde(default = "default_trust")]
    pub trust: i32,
    #[serde(default)]
    pub tension: i32,
    #[serde(default)]
    pub sadness: i32,
    #[serde(default = "default_playfulness")]
    pub playfulness: i32,
    #[serde(default = "default_attachment")]
    pub attachment: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionEvent {
    #[serde(alias = "event_type")]
    pub event_type: String,
    #[serde(default = "default_intensity")]
    pub intensity: f64,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub valence: f64,
    #[serde(default)]
    pub reason: String,
    #[serde(default, alias = "suggested_delta")]
    pub suggested_delta: StateDelta,
    #[serde(default, alias = "applied_delta")]
    pub applied_delta: StateDelta,
    #[serde(default, alias = "created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDelta {
    #[serde(default)]
    pub affection: i32,
    #[serde(default)]
    pub trust: i32,
    #[serde(default)]
    pub tension: i32,
    #[serde(default)]
    pub sadness: i32,
    #[serde(default)]
    pub playfulness: i32,
    #[serde(default)]
    pub attachment: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipRules {
    #[serde(default = "default_recent_event_limit")]
    pub recent_event_limit: usize,
    #[serde(default)]
    pub floor: RelationshipDimensions,
    #[serde(default = "default_ceiling")]
    pub ceiling: RelationshipDimensions,
    #[serde(default)]
    pub decay_per_turn: StateDelta,
    #[serde(default)]
    pub event_impacts: std::collections::HashMap<String, StateDelta>,
    #[serde(default = "default_display_order")]
    pub display_order: Vec<String>,
    #[serde(default = "default_true")]
    pub analyzer_enabled: bool,
    #[serde(default)]
    pub developer_mode: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipPanelSnapshot {
    pub agent_id: String,
    pub dimensions: RelationshipDimensions,
    pub last_event: Option<InteractionEvent>,
    pub recent_events: Vec<InteractionEvent>,
    pub relationship_block: String,
    pub raw_json: serde_json::Value,
    pub rules: RelationshipRules,
}

fn default_version() -> u32 { 1 }
fn default_affection() -> i32 { 60 }
fn default_trust() -> i32 { 50 }
fn default_playfulness() -> i32 { 30 }
fn default_attachment() -> i32 { 40 }
fn default_intensity() -> f64 { 1.0 }
fn default_confidence() -> f64 { 1.0 }
fn default_recent_event_limit() -> usize { 8 }
fn default_true() -> bool { true }
fn default_display_order() -> Vec<String> {
    vec![
        "affection".to_string(),
        "trust".to_string(),
        "tension".to_string(),
        "sadness".to_string(),
        "playfulness".to_string(),
        "attachment".to_string(),
    ]
}

impl Default for RelationshipDimensions {
    fn default() -> Self {
        Self {
            affection: default_affection(),
            trust: default_trust(),
            tension: 0,
            sadness: 0,
            playfulness: default_playfulness(),
            attachment: default_attachment(),
        }
    }
}

impl Default for AgentRelationshipState {
    fn default() -> Self {
        Self {
            dimensions: RelationshipDimensions::default(),
            last_event: None,
            recent_events: Vec::new(),
            updated_at: String::new(),
            turn_count: 0,
        }
    }
}

impl Default for RelationshipStateRoot {
    fn default() -> Self {
        Self {
            version: default_version(),
            by_agent: std::collections::HashMap::new(),
        }
    }
}

impl Default for RelationshipRules {
    fn default() -> Self {
        let mut event_impacts = std::collections::HashMap::new();
        event_impacts.insert("gratitude".to_string(), StateDelta { affection: 4, trust: 2, tension: -2, sadness: 0, playfulness: 1, attachment: 1 });
        event_impacts.insert("praise".to_string(), StateDelta { affection: 3, trust: 1, tension: -1, sadness: 0, playfulness: 2, attachment: 1 });
        event_impacts.insert("insult".to_string(), StateDelta { affection: -6, trust: -5, tension: 8, sadness: 4, playfulness: -3, attachment: -2 });
        event_impacts.insert("apology".to_string(), StateDelta { affection: 2, trust: 3, tension: -6, sadness: -3, playfulness: 0, attachment: 1 });
        event_impacts.insert("rejection".to_string(), StateDelta { affection: -3, trust: -1, tension: 2, sadness: 5, playfulness: -2, attachment: -1 });
        event_impacts.insert("repair".to_string(), StateDelta { affection: 3, trust: 4, tension: -7, sadness: -4, playfulness: 1, attachment: 2 });
        event_impacts.insert("neutral".to_string(), StateDelta::default());
        Self {
            recent_event_limit: default_recent_event_limit(),
            floor: RelationshipDimensions { affection: 0, trust: 0, tension: 0, sadness: 0, playfulness: 0, attachment: 0 },
            ceiling: default_ceiling(),
            decay_per_turn: StateDelta { affection: 0, trust: 0, tension: -1, sadness: -1, playfulness: -1, attachment: 0 },
            event_impacts,
            display_order: default_display_order(),
            analyzer_enabled: true,
            developer_mode: false,
        }
    }
}

fn default_ceiling() -> RelationshipDimensions {
    RelationshipDimensions { affection: 100, trust: 100, tension: 100, sadness: 100, playfulness: 100, attachment: 100 }
}
