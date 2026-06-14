const APP_DATA_SCHEMA_VERSION: u32 = 1;

// ========== 数据迁移版本门禁 ==========
//
// 版本语义：
//   - DATA_MIGRATION_VERSION_V1_BASELINE：历史启动期兼容迁移的合集。
//     这些迁移在「显式版本号」机制引入之前就已存在，因此 v1 不拆成单步，
//     统一由 read_app_data() 里的 run_v1_baseline_migrations 门禁触发。
//   - DATA_MIGRATION_CURRENT_VERSION：当前数据迁移版本，启动期写回 runtime_state。
//
// 新增迁移（v2+）的接入流程：
//   1. 在 app_data_layout.rs 的 data_migration_steps() 注册一个 DataMigrationStep；
//   2. 在此处新增 DATA_MIGRATION_VERSION_V2 常量，并把 CURRENT_VERSION 提到它；
//   3. 不要继续往 v1 baseline 门禁块里堆叠。
// 当前尚无 v2+ 迁移，data_migration_steps() 为空，registry 未接入执行路径。
const DATA_MIGRATION_VERSION_V1_BASELINE: u32 = 1;
const DATA_MIGRATION_CURRENT_VERSION: u32 = DATA_MIGRATION_VERSION_V1_BASELINE;
const MAX_MULTIMODAL_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_AGENT_ID: &str = "default-agent";
const DEPUTY_AGENT_ID: &str = "deputy-agent";
const USER_PERSONA_ID: &str = "user-persona";
const SYSTEM_PERSONA_ID: &str = "system-persona";
const ASSISTANT_DEPARTMENT_ID: &str = "assistant-department";
const LEADER_DEPARTMENT_ID: &str = "leader-department";
const DEPUTY_DEPARTMENT_ID: &str = "deputy-department";
const REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID: &str = "remote-customer-service-department";
const DELEGATE_TOOL_KIND_DELEGATE: &str = "delegate";
const DELEGATE_TOOL_KIND_USER_MENTION: &str = "user_async_delegate";
const SYSTEM_NOTIFICATION_CONVERSATION_ID: &str = "system-notification-conversation";
const CONVERSATION_KIND_CHAT: &str = "chat";
const CONVERSATION_KIND_SYSTEM_NOTIFICATION: &str = "system_notification";
const CONVERSATION_KIND_DELEGATE: &str = "delegate";
const CONVERSATION_KIND_REMOTE_IM_CONTACT: &str = "remote_im_contact";
const DEFAULT_RESPONSE_STYLE_ID: &str = "concise";
const DEFAULT_PDF_READ_MODE: &str = "image";
const DEFAULT_BACKGROUND_VOICE_SCREENSHOT_MODE: &str = "focused_window";
const CHAT_ABORTED_BY_USER_ERROR: &str = "CHAT_ABORTED_BY_USER";
const CHAT_DISPATCH_RESTART_AFTER_COMPACTION: &str = "CHAT_DISPATCH_RESTART_AFTER_COMPACTION";
const APP_HTTP_ORIGINATOR: &str = "p_ai_desktop";
