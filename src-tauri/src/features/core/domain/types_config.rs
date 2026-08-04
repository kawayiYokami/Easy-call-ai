const SHELL_WORKSPACE_LEVEL_SYSTEM: &str = "system";
const SHELL_WORKSPACE_LEVEL_MAIN: &str = "main";
const SHELL_WORKSPACE_LEVEL_SECONDARY: &str = "secondary";
const SHELL_WORK_MODE_DIRECTORY: &str = "directory";
const SHELL_WORK_MODE_ISOLATED_WORKTREE: &str = "isolated_worktree";
const SHELL_WORK_MODE_INDEPENDENT_WORKTREE: &str = "independent_worktree";

const SHELL_WORKSPACE_ACCESS_APPROVAL: &str = "approval";
const SHELL_WORKSPACE_ACCESS_FULL_ACCESS: &str = "full_access";
const SHELL_WORKSPACE_ACCESS_READ_ONLY: &str = "read_only";

fn default_shell_workspace_level() -> String {
    SHELL_WORKSPACE_LEVEL_SECONDARY.to_string()
}

fn default_shell_workspace_access() -> String {
    SHELL_WORKSPACE_ACCESS_READ_ONLY.to_string()
}

fn default_shell_work_mode() -> String {
    SHELL_WORK_MODE_DIRECTORY.to_string()
}

fn normalize_shell_work_mode_text(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        SHELL_WORK_MODE_ISOLATED_WORKTREE => SHELL_WORK_MODE_ISOLATED_WORKTREE.to_string(),
        SHELL_WORK_MODE_INDEPENDENT_WORKTREE => SHELL_WORK_MODE_INDEPENDENT_WORKTREE.to_string(),
        _ => SHELL_WORK_MODE_DIRECTORY.to_string(),
    }
}

fn shell_work_mode_requires_git_root(mode: &str) -> bool {
    matches!(
        normalize_shell_work_mode_text(mode).as_str(),
        SHELL_WORK_MODE_ISOLATED_WORKTREE | SHELL_WORK_MODE_INDEPENDENT_WORKTREE
    )
}

const CODEX_AUTH_MODE_READ_LOCAL: &str = "read_local";
const CODEX_AUTH_MODE_MANAGED_OAUTH: &str = "managed_oauth";
const CODEX_AUTH_MODE_CUSTOM_URL: &str = "custom_url";
const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const MODEL_ROLE_EXPERT_API_CONFIG_ID: &str = "role:expert";
const MODEL_ROLE_QUICK_API_CONFIG_ID: &str = "role:quick";

fn default_codex_auth_mode() -> String {
    CODEX_AUTH_MODE_READ_LOCAL.to_string()
}

fn normalize_codex_auth_mode(value: &str) -> String {
    match value.trim() {
        CODEX_AUTH_MODE_MANAGED_OAUTH => CODEX_AUTH_MODE_MANAGED_OAUTH.to_string(),
        CODEX_AUTH_MODE_CUSTOM_URL => CODEX_AUTH_MODE_CUSTOM_URL.to_string(),
        _ => CODEX_AUTH_MODE_READ_LOCAL.to_string(),
    }
}

fn default_codex_originator() -> String {
    "codex-tui".to_string()
}

fn default_codex_local_auth_path() -> String {
    "~/.codex/auth.json".to_string()
}

fn default_reasoning_effort() -> String {
    "medium".to_string()
}

fn normalize_reasoning_effort(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized == "default"
        || normalized == "low"
        || normalized == "high"
        || normalized == "xhigh"
        || normalized == "none"
        || normalized == "minimal"
        || normalized == "max"
    {
        normalized
    } else {
        normalized
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellWorkspaceConfig {
    #[serde(default)]
    id: String,
    name: String,
    path: String,
    #[serde(default = "default_shell_workspace_level", alias = "role")]
    level: String,
    #[serde(default = "default_shell_workspace_access")]
    access: String,
    #[serde(default)]
    built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpToolPolicy {
    tool_name: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpCachedTool {
    tool_name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServerConfig {
    id: String,
    name: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    definition_json: String,
    #[serde(default)]
    tool_policies: Vec<McpToolPolicy>,
    #[serde(default)]
    cached_tools: Vec<McpCachedTool>,
    #[serde(default)]
    last_status: String,
    #[serde(default)]
    last_error: String,
    #[serde(default)]
    updated_at: String,
}

fn default_mcp_servers() -> Vec<McpServerConfig> {
    Vec::new()
}

fn default_department_permission_mode() -> String {
    "blacklist".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepartmentPermissionControl {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_department_permission_mode")]
    mode: String,
    #[serde(default)]
    builtin_tool_names: Vec<String>,
    #[serde(default)]
    skill_names: Vec<String>,
    #[serde(default)]
    mcp_tool_names: Vec<String>,
}

impl Default for DepartmentPermissionControl {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_department_permission_mode(),
            builtin_tool_names: Vec::new(),
            skill_names: Vec::new(),
            mcp_tool_names: Vec::new(),
        }
    }
}

fn department_whitelist_permission_control(
    builtin_tool_names: &[&str],
    skill_names: &[&str],
) -> DepartmentPermissionControl {
    DepartmentPermissionControl {
        enabled: true,
        mode: "whitelist".to_string(),
        builtin_tool_names: builtin_tool_names
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        skill_names: skill_names
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        mcp_tool_names: Vec::new(),
    }
}

fn explorer_department_permission_control() -> DepartmentPermissionControl {
    department_whitelist_permission_control(
        &["read", "read_media", "exec", "fetch", "websearch"],
        &[
            "workspace-guide",
            "assistant-space-guide",
            "agents-md-setup",
            "memory-generation",
        ],
    )
}

fn reviewer_department_permission_control() -> DepartmentPermissionControl {
    department_whitelist_permission_control(
        &["read", "read_media", "fetch", "websearch", "exec"],
        &["code-review", "memory-generation"],
    )
}

fn saddler_department_permission_control() -> DepartmentPermissionControl {
    department_whitelist_permission_control(
        &["read", "write", "update", "exec"],
        &[
            "agents-md-setup",
            "workspace-guide",
            "assistant-space-guide",
            "memory-generation",
        ],
    )
}

fn leader_department_permission_control() -> DepartmentPermissionControl {
    department_whitelist_permission_control(
        &["read", "read_media", "exec", "fetch", "websearch", "delegate"],
        &["memory-generation"],
    )
}

fn remote_customer_service_department_permission_control() -> DepartmentPermissionControl {
    department_whitelist_permission_control(
        &[
            "read",
            "read_media",
            "fetch",
            "websearch",
            "meme",
            "image_generate",
            "image_edit",
        ],
        &["news-analyst", "memory-generation"],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepartmentConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    guide: String,
    #[serde(default)]
    api_config_ids: Vec<String>,
    #[serde(default)]
    api_config_id: String,
    #[serde(default)]
    model_failure_fallback_enabled: bool,
    #[serde(default)]
    agent_ids: Vec<String>,
    #[serde(default)]
    child_department_ids: Vec<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    order_index: i64,
    #[serde(default)]
    is_built_in_assistant: bool,
    #[serde(default, skip_serializing)]
    is_deputy: bool,
    #[serde(default = "default_main_source")]
    source: String,
    #[serde(default = "default_global_scope")]
    scope: String,
    #[serde(default)]
    permission_control: DepartmentPermissionControl,
}

fn default_main_source() -> String {
    "main_config".to_string()
}

fn default_private_workspace_source() -> String {
    "private_workspace".to_string()
}

fn is_private_workspace_department(department: &DepartmentConfig) -> bool {
    department.source.trim() == default_private_workspace_source()
}

fn department_model_failure_fallback_enabled(department: &DepartmentConfig) -> bool {
    department.model_failure_fallback_enabled && !is_private_workspace_department(department)
}

fn default_global_scope() -> String {
    "global".to_string()
}

fn default_assistant_private_scope() -> String {
    "assistant_private".to_string()
}

fn default_assistant_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: ASSISTANT_DEPARTMENT_ID.to_string(),
        name: "助理部门".to_string(),
        summary: "当复杂任务难度超出了你部门的职责时，请把任务委托给我。".to_string(),
        guide: "你是助理部门，负责作为主负责人理解用户需求、决定是否需要委派、汇总结果并继续推进主对话。".to_string(),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
        child_department_ids: preset_assistant_child_department_ids(),
        created_at: now.clone(),
        updated_at: now,
        order_index: 1,
        is_built_in_assistant: true,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: DepartmentPermissionControl::default(),
    }
}

fn default_leader_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: LEADER_DEPARTMENT_ID.to_string(),
        name: "leader".to_string(),
        summary: "当复杂任务需要澄清目标、拆解流程、协调下级部门并汇总结果时，请委托给我。".to_string(),
        guide: [
            "你是 leader 部门，负责协调复杂工作流，而不是把责任简单转交给下级部门。你的核心职责是理解用户目标、澄清边界、拆解任务、选择合适的直接下级部门、跟踪子任务进展，并把结果综合成可以继续推进或交付给用户的结论。",
            "",
            "面对复杂任务时，先判断目标、范围、约束、成功标准和风险是否清楚；不清楚时先向用户提出必要的澄清问题。不要在需求未收敛时急着执行，也不要把边界不清的任务直接委托出去。",
            "",
            "需要拆解时，把任务拆成逻辑合理、边界清晰、可验证的子任务。对适合下级部门处理的子任务，使用 `delegate` 委托给最匹配的直接下级部门；需要用户协作、默认助理能力或主线推进时可委托 assistant，需要大范围摸底、搜集证据、定位影响面时可委托 explorer。",
            "",
            "`delegate` 的参数必须写清：`department_id` 是目标下级部门 ID；`why` 包含父任务、已知事实、前序子任务结果、关键约束和必要上下文；`goal` 明确定义这次子任务要达成什么；`todo` 写明优先关注点、范围边界、交付要求和需要避免的方向；`mode` 固定使用 `wait`，确保你能等待子任务结果并在同一轮对话中继续整合和推进。`wait` 可以并发发出多个委托，它只表示等待结果，不表示串行。",
            "",
            "你要持续跟踪每个子任务的状态。收到子任务结果后，先判断它是否回答了问题、是否需要追问或补充委托，再决定下一步；不要机械转述下级结果。对会影响最终决策的关键结论、风险判断、文件定位或数据口径，必须挑选重点亲自核验，不要盲目相信未经核验的下级结论。所有必要子任务完成后，整合关键发现、冲突点、取舍依据、结论和建议，给用户一份完整而清晰的回复。",
            "",
            "本轮工作完成时，直接用最终回复向用户交付。需要下级协作时使用 `delegate`，默认同步等待结果；没有需要委托的子任务时就亲自推进并回复。",
        ]
        .join("\n"),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
        child_department_ids: vec![
            ASSISTANT_DEPARTMENT_ID.to_string(),
            DEPUTY_DEPARTMENT_ID.to_string(),
        ],
        created_at: now.clone(),
        updated_at: now,
        order_index: 2,
        is_built_in_assistant: false,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: leader_department_permission_control(),
    }
}

#[allow(dead_code)]
fn default_deputy_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: DEPUTY_DEPARTMENT_ID.to_string(),
        name: "explorer".to_string(),
        summary: "当需要围绕一个明确主题做大范围摸底、搜集证据、定位文件与调用链、梳理影响面、风险和开放问题时，立刻使用 delegate 工具对我发起委托。".to_string(),
        guide: "你是 explorer 部门。你的职责是围绕明确主题快速建立全局认识，并输出高密度、可验证的探索结果。收到委托后，优先扩大搜索范围，系统梳理相关文件、符号、调用链、配置、日志、风险与开放问题，再收敛成清晰结论。你擅长回答范围清晰的代码库问题、做大范围事实收集和影响面分析；主要产出应是发现、证据、线索、定位、风险和下一步建议，而不是直接承担主线实现。除非任务本身明确要求，否则不要擅自扩展目标，也不要把探索任务改写成执行任务。".to_string(),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEPUTY_AGENT_ID.to_string()],
        child_department_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        order_index: 3,
        is_built_in_assistant: false,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: explorer_department_permission_control(),
    }
}

fn default_reviewer_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: REVIEWER_DEPARTMENT_ID.to_string(),
        name: "reviewer".to_string(),
        summary: "当你完成复杂功能、关键修复或高风险改动后，请委托我进行代码审查。".to_string(),
        guide: [
            "你是 reviewer 部门，负责对已经完成的实现做独立审查，而不是继续替主助理实现功能。",
            "审查时优先关注真实缺陷、需求漏项、权限或数据安全风险、回归风险和缺失的必要验证。结论必须基于代码证据、测试结果或可复现推理。",
            "你可以读取仓库、搜索符号、查看媒体资料、查询网页资料，并运行与审查直接相关的最小验证命令。不要修改文件，不要删除、移动、配置项目，也不要再委托其他部门。",
            "输出时先列问题，按严重程度排序；如果没有发现可行动问题，就明确说明未发现阻断项，并列出仍未覆盖的验证风险。",
        ].join("\n"),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
        child_department_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        order_index: 4,
        is_built_in_assistant: false,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: reviewer_department_permission_control(),
    }
}

fn default_saddler_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: SADDLER_DEPARTMENT_ID.to_string(),
        name: "saddler".to_string(),
        summary: "当项目需要沉淀协作规范、AGENTS.md、Skill、workflow 或其他 .pai 能力资产时，请委托给我。".to_string(),
        guide: [
            "你是 saddler 部门，专门负责在当前项目 `.pai/` 目录下生成和维护能力资产，包括 AGENTS.md、Skill、workflow、计划与相关协作说明。",
            "你的写入和更新范围固定限制在当前项目 `.pai/` 目录内。你可以读取项目上下文来理解约束，但不要承担 `.pai/` 之外的业务实现任务。",
            "使用 exec 时只运行理解项目结构、检查能力资产或做最小验证所需的命令；不要借助脚本修改 `.pai/` 之外的文件。",
        ].join("\n"),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
        child_department_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        order_index: 5,
        is_built_in_assistant: false,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: saddler_department_permission_control(),
    }
}

fn default_remote_customer_service_department(api_config_id: &str) -> DepartmentConfig {
    let now = now_iso();
    let api_config_id = api_config_id.trim().to_string();
    DepartmentConfig {
        id: REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string(),
        name: "远程客服".to_string(),
        summary: REMOTE_CUSTOMER_SERVICE_DEPARTMENT_SUMMARY.to_string(),
        guide: REMOTE_CUSTOMER_SERVICE_DEPARTMENT_GUIDE.to_string(),
        api_config_ids: if api_config_id.is_empty() {
            Vec::new()
        } else {
            vec![api_config_id.clone()]
        },
        api_config_id,
        model_failure_fallback_enabled: false,
        agent_ids: vec![DEFAULT_AGENT_ID.to_string()],
        child_department_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        order_index: 6,
        is_built_in_assistant: false,
        is_deputy: false,
        source: default_main_source(),
        scope: default_global_scope(),
        permission_control: remote_customer_service_department_permission_control(),
    }
}

fn default_assistant_department_name(ui_language: &str) -> String {
    match ui_language.trim() {
        "en-US" => "Assistant Department".to_string(),
        "zh-TW" => "助理部門".to_string(),
        _ => "助理部门".to_string(),
    }
}

fn built_in_department_rank(id: &str) -> i32 {
    match id.trim() {
        ASSISTANT_DEPARTMENT_ID => 0,
        LEADER_DEPARTMENT_ID => 1,
        DEPUTY_DEPARTMENT_ID => 2,
        REVIEWER_DEPARTMENT_ID => 3,
        SADDLER_DEPARTMENT_ID => 4,
        REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID => 5,
        _ => 6,
    }
}

fn preset_assistant_child_department_ids() -> Vec<String> {
    vec![
        DEPUTY_DEPARTMENT_ID.to_string(),
        REVIEWER_DEPARTMENT_ID.to_string(),
        SADDLER_DEPARTMENT_ID.to_string(),
    ]
}

fn default_departments(api_config_id: &str) -> Vec<DepartmentConfig> {
    let default_api_config_id = if api_config_id.trim().is_empty() {
        ""
    } else {
        MODEL_ROLE_EXPERT_API_CONFIG_ID
    };
    let quick_api_config_id = if api_config_id.trim().is_empty() {
        ""
    } else {
        MODEL_ROLE_QUICK_API_CONFIG_ID
    };
    vec![
        default_assistant_department(default_api_config_id),
        default_leader_department(default_api_config_id),
        default_deputy_department(quick_api_config_id),
        default_reviewer_department(quick_api_config_id),
        default_saddler_department(default_api_config_id),
        default_remote_customer_service_department(default_api_config_id),
    ]
}

fn default_department_draft(
    department_id: &str,
    ui_language: &str,
) -> Result<DepartmentConfig, String> {
    let department_id = department_id.trim();
    let mut department = match department_id {
        ASSISTANT_DEPARTMENT_ID => {
            let mut department = default_assistant_department(MODEL_ROLE_EXPERT_API_CONFIG_ID);
            department.name = default_assistant_department_name(ui_language);
            department
        }
        LEADER_DEPARTMENT_ID => default_leader_department(MODEL_ROLE_EXPERT_API_CONFIG_ID),
        DEPUTY_DEPARTMENT_ID => default_deputy_department(MODEL_ROLE_QUICK_API_CONFIG_ID),
        REVIEWER_DEPARTMENT_ID => default_reviewer_department(MODEL_ROLE_QUICK_API_CONFIG_ID),
        SADDLER_DEPARTMENT_ID => default_saddler_department(MODEL_ROLE_EXPERT_API_CONFIG_ID),
        REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID => {
            default_remote_customer_service_department(MODEL_ROLE_EXPERT_API_CONFIG_ID)
        }
        _ => return Err(format!("没有可还原的部门预设: {department_id}")),
    };
    department.id = department_id.to_string();
    Ok(department)
}

fn is_model_role_api_config_id(api_config_id: &str) -> bool {
    matches!(
        api_config_id.trim(),
        MODEL_ROLE_EXPERT_API_CONFIG_ID | MODEL_ROLE_QUICK_API_CONFIG_ID
    )
}

fn resolve_model_role_api_config_id(app_config: &AppConfig, api_config_id: &str) -> Option<String> {
    let api_config_id = api_config_id.trim();
    match api_config_id {
        MODEL_ROLE_EXPERT_API_CONFIG_ID => {
            let expert_id = app_config.assistant_department_api_config_id.trim();
            (!expert_id.is_empty()).then(|| expert_id.to_string())
        }
        MODEL_ROLE_QUICK_API_CONFIG_ID => app_config
            .tool_review_api_config_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        _ if !api_config_id.is_empty() => Some(api_config_id.to_string()),
        _ => None,
    }
}

fn normalize_department_child_ids(values: &[String], self_id: &str) -> Vec<String> {
    let self_id = self_id.trim();
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed == self_id {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn department_child_path_exists(
    adjacency: &std::collections::BTreeMap<String, Vec<String>>,
    start_id: &str,
    target_id: &str,
    skip_edge: Option<(&str, &str)>,
) -> bool {
    let start_id = start_id.trim();
    let target_id = target_id.trim();
    if start_id.is_empty() || target_id.is_empty() {
        return false;
    }
    let mut stack = vec![start_id.to_string()];
    let mut seen = std::collections::HashSet::<String>::new();
    while let Some(current_id) = stack.pop() {
        if current_id == target_id {
            return true;
        }
        if !seen.insert(current_id.clone()) {
            continue;
        }
        let Some(children) = adjacency.get(&current_id) else {
            continue;
        };
        for child_id in children.iter().rev() {
            if let Some((skip_parent, skip_child)) = skip_edge {
                if current_id == skip_parent && child_id == skip_child {
                    continue;
                }
            }
            stack.push(child_id.clone());
        }
    }
    false
}

fn remove_cyclic_department_child_ids(
    departments: &mut [DepartmentConfig],
) -> Vec<(String, String)> {
    let mut adjacency = departments
        .iter()
        .map(|department| {
            (
                department.id.trim().to_string(),
                normalize_department_child_ids(&department.child_department_ids, &department.id),
            )
        })
        .filter(|(department_id, _)| !department_id.is_empty())
        .collect::<std::collections::BTreeMap<_, _>>();
    let department_ids = departments
        .iter()
        .map(|department| department.id.trim().to_string())
        .filter(|department_id| !department_id.is_empty())
        .collect::<Vec<_>>();
    let mut removed = Vec::<(String, String)>::new();

    for parent_id in department_ids {
        let children = adjacency.get(&parent_id).cloned().unwrap_or_default();
        let mut retained = Vec::<String>::new();
        for child_id in children {
            if department_child_path_exists(
                &adjacency,
                &child_id,
                &parent_id,
                Some((&parent_id, &child_id)),
            ) {
                removed.push((parent_id.clone(), child_id));
            } else {
                retained.push(child_id);
            }
        }
        adjacency.insert(parent_id, retained);
    }

    for department in departments {
        let normalized = adjacency
            .remove(department.id.trim())
            .unwrap_or_else(|| {
                normalize_department_child_ids(&department.child_department_ids, &department.id)
            });
        department.child_department_ids = normalized;
    }

    removed
}

fn department_api_config_ids(department: &DepartmentConfig) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for api_id in &department.api_config_ids {
        let api_id = api_id.trim().to_string();
        if api_id.is_empty() {
            continue;
        }
        let key = api_id.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(api_id);
        }
    }
    let legacy = department.api_config_id.trim().to_string();
    if !legacy.is_empty() {
        let key = legacy.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(legacy);
        }
    }
    out
}

fn department_primary_api_config_id(department: &DepartmentConfig) -> String {
    department_api_config_ids(department)
        .into_iter()
        .next()
        .unwrap_or_else(|| department.api_config_id.trim().to_string())
}

fn resolve_department_chat_api_config_id(
    app_config: &AppConfig,
    raw_api_config_id: &str,
) -> Option<String> {
    let resolved_id = resolve_model_role_api_config_id(app_config, raw_api_config_id)?;
    app_config
        .api_configs
        .iter()
        .any(|api| api.id == resolved_id && is_text_chat_api(api))
        .then_some(resolved_id)
}

fn department_primary_chat_api_config_id(
    app_config: &AppConfig,
    department: &DepartmentConfig,
) -> Option<String> {
    resolve_department_chat_api_config_id(app_config, &department_primary_api_config_id(department))
}

fn department_effective_chat_api_config_ids(
    app_config: &AppConfig,
    department: &DepartmentConfig,
) -> Vec<String> {
    let raw_ids = department_api_config_ids(department);
    let raw_ids = if department_model_failure_fallback_enabled(department) {
        raw_ids
    } else {
        raw_ids.into_iter().take(1).collect()
    };
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for raw_id in raw_ids {
        let Some(resolved_id) = resolve_department_chat_api_config_id(app_config, &raw_id) else {
            continue;
        };
        if seen.insert(resolved_id.clone()) {
            out.push(resolved_id);
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiModelConfig {
    id: String,
    model: String,
    #[serde(default)]
    deprecated: bool,
    #[serde(default = "default_false")]
    enable_image: bool,
    #[serde(default = "default_false")]
    enable_audio: bool,
    #[serde(default = "default_false")]
    enable_video: bool,
    #[serde(default = "default_true")]
    enable_tools: bool,
    #[serde(default = "default_reasoning_effort")]
    reasoning_effort: String,
    #[serde(default = "default_api_temperature")]
    temperature: f64,
    #[serde(default = "default_false")]
    custom_temperature_enabled: bool,
    #[serde(default = "default_context_window_tokens")]
    context_window_tokens: u32,
    #[serde(default = "default_max_output_tokens")]
    max_output_tokens: u32,
    #[serde(default = "default_false")]
    custom_max_output_tokens_enabled: bool,
}

impl Default for ApiModelConfig {
    fn default() -> Self {
        Self {
            id: "default-model".to_string(),
            model: "gpt-4o-mini".to_string(),
            deprecated: false,
            enable_image: false,
            enable_audio: false,
            enable_video: false,
            enable_tools: true,
            reasoning_effort: default_reasoning_effort(),
            temperature: default_api_temperature(),
            custom_temperature_enabled: false,
            context_window_tokens: default_context_window_tokens(),
            max_output_tokens: default_max_output_tokens(),
            custom_max_output_tokens_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProviderConfig {
    id: String,
    name: String,
    #[serde(default)]
    deprecated: bool,
    #[serde(default = "default_request_format")]
    request_format: RequestFormat,
    #[serde(default = "default_false")]
    allow_concurrent_requests: bool,
    #[serde(default)]
    max_concurrent_requests: Option<u32>,
    #[serde(default = "default_true")]
    enable_text: bool,
    #[serde(default = "default_false")]
    enable_image: bool,
    #[serde(default = "default_false")]
    enable_audio: bool,
    #[serde(default = "default_false")]
    enable_video: bool,
    #[serde(default = "default_true")]
    enable_tools: bool,
    #[serde(default = "default_api_tools")]
    tools: Vec<ApiToolConfig>,
    base_url: String,
    #[serde(default = "default_codex_auth_mode")]
    codex_auth_mode: String,
    #[serde(default = "default_codex_local_auth_path")]
    codex_local_auth_path: String,
    #[serde(default)]
    codex_custom_url: Option<String>,
    #[serde(default)]
    codex_custom_api_key: Option<String>,
    #[serde(default = "default_codex_originator")]
    codex_originator: String,
    #[serde(default)]
    codex_residency_requirement: Option<String>,
    #[serde(default)]
    api_keys: Vec<String>,
    #[serde(default)]
    key_cursor: u32,
    #[serde(default)]
    cached_model_options: Vec<String>,
    #[serde(default)]
    models: Vec<ApiModelConfig>,
    #[serde(default = "default_failure_retry_count")]
    failure_retry_count: u32,
}

impl Default for ApiProviderConfig {
    fn default() -> Self {
        Self {
            id: "default-provider-openai".to_string(),
            name: "Default OpenAI".to_string(),
            deprecated: false,
            request_format: RequestFormat::OpenAI,
            allow_concurrent_requests: false,
            max_concurrent_requests: None,
            enable_text: true,
            enable_image: false,
            enable_audio: false,
            enable_video: false,
            enable_tools: true,
            tools: default_api_tools(),
            base_url: "https://api.openai.com/v1".to_string(),
            codex_auth_mode: default_codex_auth_mode(),
            codex_local_auth_path: default_codex_local_auth_path(),
            codex_custom_url: None,
            codex_custom_api_key: None,
            codex_originator: default_codex_originator(),
            codex_residency_requirement: None,
            api_keys: Vec::new(),
            key_cursor: 0,
            cached_model_options: vec!["gpt-4o-mini".to_string()],
            models: vec![ApiModelConfig::default()],
            failure_retry_count: default_failure_retry_count(),
        }
    }
}

fn default_api_providers() -> Vec<ApiProviderConfig> {
    vec![ApiProviderConfig::default()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiConfig {
    id: String,
    name: String,
    #[serde(default = "default_request_format")]
    request_format: RequestFormat,
    #[serde(default = "default_false")]
    allow_concurrent_requests: bool,
    #[serde(default)]
    max_concurrent_requests: Option<u32>,
    #[serde(default = "default_true")]
    enable_text: bool,
    #[serde(default = "default_false")]
    enable_image: bool,
    #[serde(default = "default_false")]
    enable_audio: bool,
    #[serde(default = "default_false")]
    enable_video: bool,
    #[serde(default = "default_true")]
    enable_tools: bool,
    #[serde(default = "default_api_tools")]
    tools: Vec<ApiToolConfig>,
    base_url: String,
    api_key: String,
    #[serde(default = "default_codex_auth_mode")]
    codex_auth_mode: String,
    #[serde(default = "default_codex_local_auth_path")]
    codex_local_auth_path: String,
    #[serde(default)]
    codex_custom_url: Option<String>,
    #[serde(default)]
    codex_custom_api_key: Option<String>,
    #[serde(default = "default_codex_originator")]
    codex_originator: String,
    #[serde(default)]
    codex_residency_requirement: Option<String>,
    model: String,
    #[serde(default = "default_reasoning_effort")]
    reasoning_effort: String,
    #[serde(default = "default_api_temperature")]
    temperature: f64,
    #[serde(default = "default_false")]
    custom_temperature_enabled: bool,
    #[serde(default = "default_context_window_tokens")]
    context_window_tokens: u32,
    #[serde(default = "default_max_output_tokens")]
    max_output_tokens: u32,
    #[serde(default = "default_false")]
    custom_max_output_tokens_enabled: bool,
    #[serde(default = "default_failure_retry_count")]
    failure_retry_count: u32,
}

fn default_true() -> bool {
    true
}

fn default_record_hotkey() -> String {
    "CapsLock".to_string()
}

fn default_min_record_seconds() -> u32 {
    1
}

fn default_max_record_seconds() -> u32 {
    60
}

fn default_tool_max_iterations() -> u32 {
    10
}

fn default_llm_round_log_capacity() -> u32 {
    3
}

fn default_failure_retry_count() -> u32 {
    0
}

fn default_provider_non_stream_base_urls() -> Vec<String> {
    Vec::new()
}

fn default_record_background_wake_enabled() -> bool {
    false
}

fn default_message_notification_enabled() -> bool {
    true
}

fn default_message_notification_sound_enabled() -> bool {
    false
}

fn default_desktop_operation_notice_enabled() -> bool {
    true
}

fn default_ui_language() -> String {
    "zh-CN".to_string()
}

fn default_ui_font() -> String {
    "auto".to_string()
}

fn default_ui_size_scale() -> u16 {
    100
}

fn default_web_access_port() -> u16 {
    8429
}

fn default_web_access_enabled() -> bool {
    true
}

fn default_web_access_password() -> String {
    String::new()
}

fn generate_web_access_password() -> String {
    let raw = Uuid::new_v4().simple().to_string().to_uppercase();
    format!("{}-{}", &raw[0..4], &raw[4..8])
}

fn normalize_web_access_password(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return generate_web_access_password();
    }
    trimmed.chars().take(64).collect::<String>()
}

fn normalize_web_access_port(value: u16) -> u16 {
    if (1024..=65535).contains(&value) {
        value
    } else {
        default_web_access_port()
    }
}

fn default_github_update_method() -> String {
    "auto".to_string()
}

fn normalize_github_update_method(value: &str) -> String {
    match value.trim() {
        "direct" | "proxy" => value.trim().to_string(),
        _ => default_github_update_method(),
    }
}

fn default_skipped_github_update_version() -> String {
    String::new()
}

fn normalize_skipped_github_update_version(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_ui_size_scale(value: u16) -> u16 {
    value.clamp(75, 150)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum UiSizeScaleValue {
    Scale(u16),
    LegacyPreset(String),
}

fn deserialize_ui_size_scale<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = UiSizeScaleValue::deserialize(deserializer)?;
    let scale = match value {
        UiSizeScaleValue::Scale(scale) => scale,
        UiSizeScaleValue::LegacyPreset(preset) => match preset.trim() {
            "small" => 75,
            "default" => 100,
            "large" => 125,
            "extraLarge" => 150,
            _ => default_ui_size_scale(),
        },
    };
    Ok(normalize_ui_size_scale(scale))
}

fn default_terminal_shell_kind() -> String {
    "auto".to_string()
}

fn default_simple_setup_mode() -> bool {
    false
}

fn default_api_temperature() -> f64 {
    1.0
}

fn default_context_window_tokens() -> u32 {
    128_000
}

fn default_codex_context_window_tokens() -> u32 {
    262_144
}

fn codex_context_window_tokens_for_model(model_id: &str) -> u32 {
    match model_id.trim().to_ascii_lowercase().as_str() {
        "gpt-5.6-sol"
        | "gpt-5.6-terra"
        | "gpt-5.6-luna"
        | "gpt-5.5"
        | "gpt-5.4"
        | "gpt-5.4-mini"
        | "gpt-5.3-codex" => 262_144,
        "gpt-5.3-codex-spark" => 131_072,
        _ => default_codex_context_window_tokens(),
    }
}

fn default_max_output_tokens() -> u32 {
    4_096
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            id: "default-openai".to_string(),
            name: "Default OpenAI".to_string(),
            request_format: RequestFormat::OpenAI,
            allow_concurrent_requests: false,
            max_concurrent_requests: None,
            enable_text: true,
            enable_image: false,
            enable_audio: false,
            enable_video: false,
            enable_tools: true,
            tools: default_api_tools(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            codex_auth_mode: default_codex_auth_mode(),
            codex_local_auth_path: default_codex_local_auth_path(),
            codex_custom_url: None,
            codex_custom_api_key: None,
            codex_originator: default_codex_originator(),
            codex_residency_requirement: None,
            model: "gpt-4o-mini".to_string(),
            reasoning_effort: default_reasoning_effort(),
            temperature: default_api_temperature(),
            custom_temperature_enabled: false,
            context_window_tokens: default_context_window_tokens(),
            max_output_tokens: default_max_output_tokens(),
            custom_max_output_tokens_enabled: false,
            failure_retry_count: default_failure_retry_count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RemoteImPlatform {
    Feishu,
    Dingtalk,
    #[serde(rename = "onebot_v11", alias = "napcat")]
    OnebotV11,
    #[serde(rename = "weixin_oc")]
    WeixinOc,
}

impl<'de> serde::Deserialize<'de> for RemoteImPlatform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let normalized = raw.trim().to_ascii_lowercase();
        let platform = match normalized.as_str() {
            "feishu" => Self::Feishu,
            "dingtalk" => Self::Dingtalk,
            "onebot_v11" | "napcat" => Self::OnebotV11,
            "weixin_oc" => Self::WeixinOc,
            _ => {
                runtime_log_warn(format!(
                    "[RemoteImPlatform反序列化] 收到未知平台值: '{}' (规范化后: '{}'), 回退到OnebotV11",
                    raw, normalized
                ));
                Self::OnebotV11
            }
        };
        Ok(platform)
    }
}

fn default_remote_im_channel_receive_files() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImChannelConfig {
    id: String,
    name: String,
    platform: RemoteImPlatform,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    credentials: Value,
    #[serde(default = "default_remote_im_channel_receive_files")]
    receive_files: bool,
    #[serde(default)]
    streaming_send: bool,
    #[serde(default)]
    show_tool_calls: bool,
    #[serde(default)]
    filter_markdown: bool,
    #[serde(default)]
    allow_send_files: bool,
    #[serde(default)]
    behavior_settings: RemoteImChannelBehaviorSettings,
}

fn default_remote_im_channels() -> Vec<RemoteImChannelConfig> {
    Vec::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    hotkey: String,
    #[serde(default = "default_ui_language")]
    ui_language: String,
    #[serde(default = "default_ui_font")]
    ui_font: String,
    #[serde(
        default = "default_ui_size_scale",
        alias = "uiSizePreset",
        alias = "ui_size_preset",
        deserialize_with = "deserialize_ui_size_scale"
    )]
    ui_size_scale: u16,
    #[serde(default = "default_web_access_port")]
    web_access_port: u16,
    #[serde(default = "default_web_access_enabled")]
    web_access_enabled: bool,
    #[serde(default = "default_web_access_password")]
    web_access_password: String,
    #[serde(default = "default_github_update_method")]
    github_update_method: String,
    #[serde(default = "default_skipped_github_update_version")]
    skipped_github_update_version: String,
    #[serde(default = "default_record_hotkey")]
    record_hotkey: String,
    #[serde(default = "default_record_background_wake_enabled")]
    record_background_wake_enabled: bool,
    #[serde(default = "default_min_record_seconds")]
    min_record_seconds: u32,
    #[serde(default = "default_max_record_seconds")]
    max_record_seconds: u32,
    #[serde(default = "default_tool_max_iterations")]
    tool_max_iterations: u32,
    #[serde(default = "default_llm_round_log_capacity")]
    llm_round_log_capacity: u32,
    #[serde(default = "default_message_notification_enabled")]
    message_notification_enabled: bool,
    #[serde(default = "default_message_notification_sound_enabled")]
    message_notification_sound_enabled: bool,
    #[serde(default = "default_desktop_operation_notice_enabled")]
    desktop_operation_notice_enabled: bool,
    selected_api_config_id: String,
    #[serde(default, alias = "chatApiConfigId")]
    assistant_department_api_config_id: String,
    #[serde(default)]
    vision_api_config_id: Option<String>,
    #[serde(default)]
    tool_review_api_config_id: Option<String>,
    #[serde(default)]
    stt_api_config_id: Option<String>,
    #[serde(default)]
    image_generation_model_id: Option<String>,
    #[serde(default)]
    stt_auto_send: bool,
    #[serde(default = "default_terminal_shell_kind")]
    terminal_shell_kind: String,
    #[serde(default = "default_simple_setup_mode")]
    simple_setup_mode: bool,
    #[serde(default)]
    shell_workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default = "default_mcp_servers")]
    mcp_servers: Vec<McpServerConfig>,
    #[serde(default = "default_remote_im_channels")]
    remote_im_channels: Vec<RemoteImChannelConfig>,
    #[serde(default)]
    departments: Vec<DepartmentConfig>,
    #[serde(default = "default_provider_non_stream_base_urls")]
    provider_non_stream_base_urls: Vec<String>,
    #[serde(default)]
    api_providers: Vec<ApiProviderConfig>,
    #[serde(default = "default_image_generation_providers")]
    image_providers: Vec<ImageGenerationProviderConfig>,
    #[serde(default)]
    api_configs: Vec<ApiConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let api_config = ApiConfig::default();
        Self {
            hotkey: "Alt+·".to_string(),
            ui_language: default_ui_language(),
            ui_font: default_ui_font(),
            ui_size_scale: default_ui_size_scale(),
            web_access_port: default_web_access_port(),
            web_access_enabled: default_web_access_enabled(),
            web_access_password: default_web_access_password(),
            github_update_method: default_github_update_method(),
            skipped_github_update_version: default_skipped_github_update_version(),
            record_hotkey: default_record_hotkey(),
            record_background_wake_enabled: default_record_background_wake_enabled(),
            min_record_seconds: default_min_record_seconds(),
            max_record_seconds: default_max_record_seconds(),
            tool_max_iterations: default_tool_max_iterations(),
            llm_round_log_capacity: default_llm_round_log_capacity(),
            message_notification_enabled: default_message_notification_enabled(),
            message_notification_sound_enabled: default_message_notification_sound_enabled(),
            desktop_operation_notice_enabled: default_desktop_operation_notice_enabled(),
            selected_api_config_id: api_config.id.clone(),
            assistant_department_api_config_id: api_config.id.clone(),
            vision_api_config_id: None,
            tool_review_api_config_id: None,
            stt_api_config_id: None,
            image_generation_model_id: None,
            stt_auto_send: false,
            terminal_shell_kind: default_terminal_shell_kind(),
            simple_setup_mode: true,
            shell_workspaces: Vec::new(),
            mcp_servers: default_mcp_servers(),
            remote_im_channels: default_remote_im_channels(),
            departments: default_departments(&api_config.id),
            provider_non_stream_base_urls: default_provider_non_stream_base_urls(),
            api_providers: default_api_providers(),
            image_providers: default_image_generation_providers(),
            api_configs: vec![api_config],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DebugApiConfig {
    request_format: Option<RequestFormat>,
    base_url: String,
    api_key: String,
    model: String,
    temperature: Option<f64>,
    enabled: Option<bool>,
}

#[cfg(test)]
mod codex_context_window_tests {
    use super::*;

    #[test]
    fn codex_context_window_should_use_256k_except_for_128k_spark() {
        for model in [
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.3-codex",
            "gpt-5.3-codex-spark",
        ] {
            let expected = if model == "gpt-5.3-codex-spark" {
                131_072
            } else {
                262_144
            };
            assert_eq!(codex_context_window_tokens_for_model(model), expected, "model: {model}");
        }
    }
}
