const MEMORY_RECALL_MODE_AUTO: &str = "auto";
const MEMORY_RECALL_MODE_MANUAL: &str = "manual";
const MEMORY_RECALL_MODE_OFF: &str = "off";

fn default_agent_memory_recall_mode() -> String {
    MEMORY_RECALL_MODE_AUTO.to_string()
}

fn normalize_agent_memory_recall_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        MEMORY_RECALL_MODE_MANUAL => MEMORY_RECALL_MODE_MANUAL.to_string(),
        MEMORY_RECALL_MODE_OFF => MEMORY_RECALL_MODE_OFF.to_string(),
        _ => MEMORY_RECALL_MODE_AUTO.to_string(),
    }
}

fn agent_memory_recall_mode(agent: &AgentProfile) -> String {
    normalize_agent_memory_recall_mode(&agent.memory_recall_mode)
}

fn agent_memory_rag_enabled(agent: &AgentProfile) -> bool {
    agent_memory_recall_mode(agent) == MEMORY_RECALL_MODE_AUTO
}

fn agent_memory_recall_enabled(agent: &AgentProfile) -> bool {
    agent_memory_recall_mode(agent) != MEMORY_RECALL_MODE_OFF
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentProfile {
    id: String,
    name: String,
    system_prompt: String,
    #[serde(default = "default_agent_tools")]
    tools: Vec<ApiToolConfig>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    avatar_path: Option<String>,
    #[serde(default)]
    avatar_updated_at: Option<String>,
    #[serde(default)]
    is_built_in_user: bool,
    #[serde(default)]
    is_built_in_system: bool,
    #[serde(default)]
    private_memory_enabled: bool,
    #[serde(default = "default_agent_memory_recall_mode")]
    memory_recall_mode: String,
    #[serde(default = "default_main_source")]
    source: String,
    #[serde(default = "default_global_scope")]
    scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveAgentsInput {
    agents: Vec<AgentProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum MessagePart {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
    },
    Image {
        mime: String,
        bytes_base64: String,
        name: Option<String>,
        compressed: bool,
    },
    Audio {
        mime: String,
        bytes_base64: String,
        name: Option<String>,
        compressed: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemeAnnotation {
    meme: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatMessage {
    id: String,
    role: String,
    created_at: String,
    #[serde(default)]
    speaker_agent_id: Option<String>,
    parts: Vec<MessagePart>,
    #[serde(default)]
    extra_text_blocks: Vec<String>,
    provider_meta: Option<Value>,
    tool_call: Option<Vec<Value>>,
    mcp_call: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    meme_annotations: Option<Vec<MemeAnnotation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImMessageSource {
    channel_id: String,
    platform: RemoteImPlatform,
    im_name: String,
    remote_contact_type: String,
    remote_contact_id: String,
    remote_contact_name: String,
    sender_id: String,
    sender_name: String,
    #[serde(default)]
    sender_avatar_url: Option<String>,
    #[serde(default)]
    platform_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RemoteImActivationSource {
    channel_id: String,
    platform: RemoteImPlatform,
    remote_contact_type: String,
    remote_contact_id: String,
    remote_contact_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationTodoItem {
    content: String,
    status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationCumulativeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_tokens: u64,
    #[serde(default)]
    cache_write_tokens: u64,
}

impl ConversationCumulativeUsage {
    fn is_empty(&self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.cache_read_tokens == 0
            && self.cache_write_tokens == 0
    }

    fn keep_at_least(&mut self, other: &ConversationCumulativeUsage) {
        self.input_tokens = self.input_tokens.max(other.input_tokens);
        self.output_tokens = self.output_tokens.max(other.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(other.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.max(other.cache_write_tokens);
    }
}

fn conversation_cumulative_usage_add_provider_usage(
    target: &mut ConversationCumulativeUsage,
    usage: &Value,
) -> bool {
    fn read_u64(usage: &Value, keys: &[&str]) -> u64 {
        keys.iter()
            .find_map(|key| {
                let value = usage.get(*key)?;
                value
                    .as_u64()
                    .or_else(|| value.as_i64().and_then(|item| u64::try_from(item).ok()))
            })
            .unwrap_or(0)
    }

    let delta = ConversationCumulativeUsage {
        input_tokens: read_u64(usage, &["promptTokens", "prompt_tokens"]),
        output_tokens: read_u64(usage, &["completionTokens", "completion_tokens"]),
        cache_read_tokens: read_u64(usage, &["cachedTokens", "cached_tokens"]),
        cache_write_tokens: read_u64(
            usage,
            &["cacheCreationTokens", "cache_creation_tokens"],
        )
        .saturating_add(read_u64(
            usage,
            &["cacheCreation5mTokens", "cache_creation_5m_tokens"],
        ))
        .saturating_add(read_u64(
            usage,
            &["cacheCreation1hTokens", "cache_creation_1h_tokens"],
        )),
    };
    if delta.is_empty() {
        return false;
    }
    target.input_tokens = target.input_tokens.saturating_add(delta.input_tokens);
    target.output_tokens = target.output_tokens.saturating_add(delta.output_tokens);
    target.cache_read_tokens = target
        .cache_read_tokens
        .saturating_add(delta.cache_read_tokens);
    target.cache_write_tokens = target
        .cache_write_tokens
        .saturating_add(delta.cache_write_tokens);
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationGoalState {
    goal_id: String,
    status: String,
    objective: String,
    started_at: String,
    #[serde(default)]
    ended_at: Option<String>,
    #[serde(default)]
    usage_start: ConversationCumulativeUsage,
    #[serde(default)]
    usage_end: Option<ConversationCumulativeUsage>,
}

fn conversation_goal_is_active(goal: &ConversationGoalState) -> bool {
    goal.status.trim() == "active"
}

fn conversation_cumulative_usage_weighted_tokens(
    cumulative_usage: &ConversationCumulativeUsage,
) -> u64 {
    let weighted = (cumulative_usage.output_tokens as f64 * 2.0)
        + (cumulative_usage.cache_read_tokens as f64 * 0.02)
        + cumulative_usage.cache_write_tokens as f64;
    if !weighted.is_finite() || weighted <= 0.0 {
        0
    } else if weighted >= u64::MAX as f64 {
        u64::MAX
    } else {
        weighted.round() as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Conversation {
    id: String,
    title: String,
    agent_id: String,
    #[serde(default)]
    department_id: String,
    #[serde(default)]
    bound_conversation_id: Option<String>,
    #[serde(default)]
    parent_conversation_id: Option<String>,
    #[serde(default)]
    child_conversation_ids: Vec<String>,
    #[serde(default)]
    fork_message_cursor: Option<String>,
    #[serde(default)]
    unread_count: usize,
    #[serde(default)]
    conversation_kind: String,
    #[serde(default)]
    root_conversation_id: Option<String>,
    #[serde(default)]
    delegate_id: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    last_user_at: Option<String>,
    last_assistant_at: Option<String>,
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    user_profile_snapshot: String,
    #[serde(default)]
    shell_workspace_path: Option<String>,
    #[serde(default)]
    shell_workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    shell_autonomous_mode: bool,
    #[serde(default)]
    archived_at: Option<String>,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    current_todos: Vec<ConversationTodoItem>,
    #[serde(default)]
    memory_recall_table: Vec<String>,
    #[serde(default)]
    plan_mode_enabled: bool,
    #[serde(default)]
    preferred_api_config_id: Option<String>,
    #[serde(default, alias = "usageSummary")]
    cumulative_usage: ConversationCumulativeUsage,
    #[serde(default)]
    active_goal: Option<ConversationGoalState>,
}

#[derive(Debug, Clone)]
struct RemoteImConversationAssistantContext {
    department_id: String,
    department_name: String,
    agent_id: String,
    agent_name: String,
}

#[derive(Debug, Clone)]
struct ConversationRuntimeSlot {
    state: MainSessionState,
    pending_queue: std::collections::VecDeque<ChatPendingEvent>,
    stream_cache: ConversationStreamRuntimeCache,
    active_remote_im_activation_sources: Vec<RemoteImActivationSource>,
    active_remote_im_assistant_context: Option<RemoteImConversationAssistantContext>,
    plan_mode_enabled: bool,
    last_activity_at: String,
}

#[derive(Debug, Clone, Default)]
struct ConversationStreamRuntimeCache {
    activation_id: String,
    request_id: String,
    department_id: String,
    agent_id: String,
    assistant_text: String,
    activity_reasoning_text: String,
    tool_status_text: String,
    tool_status_state: String,
    stream_blocks: Vec<AssistantStreamBlock>,
    started_at: String,
    started_at_ms: u64,
    updated_at: String,
    persisted_assistant_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantStreamToolBlock {
    tool_call_id: String,
    name: String,
    args_text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    result_text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantStreamBlock {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    reasoning: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AssistantStreamToolBlock>,
}

impl Default for ConversationRuntimeSlot {
    fn default() -> Self {
        Self {
            state: MainSessionState::Idle,
            pending_queue: std::collections::VecDeque::new(),
            stream_cache: ConversationStreamRuntimeCache::default(),
            active_remote_im_activation_sources: Vec::new(),
            active_remote_im_assistant_context: None,
            plan_mode_enabled: false,
            last_activity_at: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct DelegateRuntimeThread {
    delegate_id: String,
    root_conversation_id: String,
    target_agent_id: String,
    title: String,
    call_stack: Vec<String>,
    parent_chat_session_key: Option<String>,
    archived_at: Option<String>,
    conversation: Conversation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationArchive {
    archive_id: String,
    archived_at: String,
    reason: String,
    source_conversation: Conversation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveSummary {
    archive_id: String,
    archived_at: String,
    title: String,
    message_count: usize,
    api_config_id: String,
    agent_id: String,
}
