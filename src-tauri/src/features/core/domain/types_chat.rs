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
