fn mcp_definition_json_schema() -> Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "P-ai MCP Definition",
        "type": "object",
        "anyOf": [
            { "required": ["mcpServers"] },
            { "required": ["command"] },
            { "required": ["url"] },
            { "minProperties": 1 }
        ],
        "properties": {
            "mcpServers": {
                "type": "object",
                "minProperties": 1,
                "additionalProperties": {
                    "type": "object",
                    "anyOf": [
                        { "required": ["command"] },
                        { "required": ["url"] }
                    ],
                    "properties": {
                        "transport": { "type": "string" },
                        "command": { "type": "string" },
                        "args": { "type": "array", "items": { "type": "string" } },
                        "env": { "type": "object", "additionalProperties": { "type": "string" } },
                        "cwd": { "type": "string" },
                        "url": { "type": "string" },
                        "bearerTokenEnvVar": { "type": "string" },
                        "httpHeaders": { "type": "object", "additionalProperties": { "type": "string" } },
                        "envHttpHeaders": { "type": "object", "additionalProperties": { "type": "string" } },
                        "enabledTools": { "type": "array", "items": { "type": "string" } },
                        "disabledTools": { "type": "array", "items": { "type": "string" } }
                    }
                }
            }
        }
    })
}

// ========== 结构化校验错误 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpValidationIssue {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    params: std::collections::HashMap<String, String>,
}

impl McpValidationIssue {
    fn new(code: &str, message: String) -> Self {
        Self {
            code: code.to_string(),
            message,
            server_name: None,
            field: None,
            params: std::collections::HashMap::new(),
        }
    }

    fn with_server(mut self, server_name: &str) -> Self {
        self.server_name = Some(server_name.to_string());
        self
    }

    fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_string());
        self
    }

    fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug, Clone)]
struct McpDefinitionValidationError {
    message: String,
    issues: Vec<McpValidationIssue>,
}

impl McpDefinitionValidationError {
    fn from_issue(issue: McpValidationIssue) -> Self {
        Self {
            message: issue.message.clone(),
            issues: vec![issue],
        }
    }
}

// ========== JSON 值读取（别名兼容） ==========

fn value_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .get(key)
        .or_else(|| value.get(&key.to_ascii_lowercase()))
        .or_else(|| {
            let snake = key
                .chars()
                .enumerate()
                .flat_map(|(idx, ch)| {
                    if ch.is_ascii_uppercase() {
                        if idx == 0 {
                            vec![ch.to_ascii_lowercase()]
                        } else {
                            vec!['_', ch.to_ascii_lowercase()]
                        }
                    } else {
                        vec![ch]
                    }
                })
                .collect::<String>();
            value.get(&snake)
        })
}

fn value_get_string(value: &Value, key: &str) -> Option<String> {
    value_get(value, key)
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 读取字符串映射，兼容两种值形态：
/// - 字符串: "KEY": "value"
/// - 对象:   "KEY": { "value": "xxx", "secret": true }（取 value 字段）
fn value_get_map_string_string(
    value: &Value,
    key: &str,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::<String, String>::new();
    let Some(map) = value_get(value, key).and_then(Value::as_object) else {
        return out;
    };
    for (k, v) in map {
        let name = k.trim();
        if name.is_empty() {
            continue;
        }
        let text = match v {
            Value::String(s) => s.trim().to_string(),
            Value::Object(obj) => obj
                .get("value")
                .and_then(Value::as_str)
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            _ => String::new(),
        };
        if !text.is_empty() {
            out.insert(name.to_string(), text);
        }
    }
    out
}

fn value_get_string_array(value: &Value, key: &str) -> Vec<String> {
    value_get(value, key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

// ========== 多格式展开 ==========

#[derive(Debug, Clone)]
struct ParsedMcpDefinition {
    servers: Vec<(String, Value)>,
    issues: Vec<McpValidationIssue>,
}

/// 判定某字段是否为"单 server 直接字段"（出现时根对象不再视为命名 server 集合）
fn is_server_direct_field(key: &str) -> bool {
    matches!(
        key,
        "command"
            | "args"
            | "env"
            | "cwd"
            | "url"
            | "transport"
            | "type"
            | "headers"
            | "httpHeaders"
            | "envHttpHeaders"
            | "bearerTokenEnvVar"
            | "enabledTools"
            | "disabledTools"
    )
}

fn issue_server_missing_name(index: usize) -> McpValidationIssue {
    McpValidationIssue::new(
        "server_missing_name",
        format!("server at index {index} is missing required 'name'"),
    )
    .with_param("index", &index.to_string())
}

fn parse_server_array(
    items: &[Value],
    field_path: &str,
) -> Result<ParsedMcpDefinition, McpDefinitionValidationError> {
    let mut out = Vec::<(String, Value)>::new();
    let mut issues = Vec::<McpValidationIssue>::new();
    for (idx, item) in items.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            issues.push(
                McpValidationIssue::new(
                    "server_item_not_object",
                    format!("{field_path}[{idx}] must be an object"),
                )
                .with_param("index", &idx.to_string()),
            );
            continue;
        };
        let value_obj = Value::Object(obj.clone());
        let name = value_get_string(&value_obj, "name");
        let Some(name) = name else {
            issues.push(issue_server_missing_name(idx));
            continue;
        };
        // 同名成员不跳过：同名覆盖由用户自行安排顺序，系统不去重
        out.push((name, value_obj));
    }
    Ok(ParsedMcpDefinition {
        servers: out,
        issues,
    })
}

fn parse_named_server_map(
    map: &serde_json::Map<String, Value>,
    field_path: &str,
) -> Result<ParsedMcpDefinition, McpDefinitionValidationError> {
    if map.is_empty() {
        return Err(McpDefinitionValidationError::from_issue(
            McpValidationIssue::new("empty_servers", format!("{field_path} is empty")),
        ));
    }
    let mut out = Vec::<(String, Value)>::new();
    let mut issues = Vec::<McpValidationIssue>::new();
    for (name, node) in map {
        let Some(obj) = node.as_object() else {
            issues.push(
                McpValidationIssue::new(
                    "server_not_object",
                    format!("{field_path}.{name} must be an object"),
                )
                .with_server(name),
            );
            continue;
        };
        out.push((name.clone(), Value::Object(obj.clone())));
    }
    Ok(ParsedMcpDefinition {
        servers: out,
        issues,
    })
}

/// 将 definitionJson 展开为 0..N 个 (server_name, server_obj)，结构问题聚合在 issues。
/// 支持的嵌套格式：
/// 1. { "mcpServers": { name: {...}, ... } }
/// 2. { name: {...}, ... }（根级平铺命名对象）
/// 3. { "mcpServers": [ {name, ...}, ... ] }
/// 4. [ {name, ...}, ... ]（根级数组）
/// 5. 单 server 直接字段（向后兼容）
///
/// Err 只用于整体无法解析（JSON 语法错、根类型错、空容器、mcpServers 类型错）。
fn normalize_mcp_member_name(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut has_legal_identifier_char = false;
    let mut previous_was_separator = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            has_legal_identifier_char = true;
            out.push(ch);
            previous_was_separator = false;
        } else if ch.is_ascii_whitespace() || matches!(ch, '.' | ':' | '/' | '\\') {
            if !previous_was_separator && !out.is_empty() {
                out.push('_');
                previous_was_separator = true;
            }
        }
    }
    if !has_legal_identifier_char {
        return None;
    }
    Some(out.trim_matches('_').to_string())
}

fn normalized_mcp_member_name_or_original(raw: &str) -> String {
    normalize_mcp_member_name(raw).unwrap_or_else(|| raw.trim().to_string())
}

fn normalized_unique_mcp_member_name(raw: &str, _used_names: &mut std::collections::HashSet<String>) -> String {
    // 成员名不去重：同名覆盖由用户自行安排顺序，规范化后原名保留
    normalized_mcp_member_name_or_original(raw)
}

fn normalize_mcp_member_name_in_array(items: &mut [Value]) {
    let mut used_names = std::collections::HashSet::<String>::new();
    for item in items {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            continue;
        };
        object.insert(
            "name".to_string(),
            Value::String(normalized_unique_mcp_member_name(name, &mut used_names)),
        );
    }
}

fn normalize_mcp_member_name_in_map(map: &mut serde_json::Map<String, Value>) {
    let original = std::mem::take(map);
    let mut used_names = std::collections::HashSet::<String>::new();
    for (name, value) in original {
        map.insert(normalized_unique_mcp_member_name(&name, &mut used_names), value);
    }
}

fn normalize_mcp_definition_member_names(definition_json: &str) -> Result<String, String> {
    let mut definition: Value = serde_json::from_str(definition_json)
        .map_err(|err| format!("Parse MCP definition JSON failed: {err}"))?;
    if let Some(items) = definition.as_array_mut() {
        normalize_mcp_member_name_in_array(items);
    } else if let Some(root) = definition.as_object_mut() {
        if let Some(servers) = root.get_mut("mcpServers") {
            if let Some(map) = servers.as_object_mut() {
                normalize_mcp_member_name_in_map(map);
            } else if let Some(items) = servers.as_array_mut() {
                normalize_mcp_member_name_in_array(items);
            }
        } else {
            let is_named_server_map = !root
                .keys()
                .any(|key| is_server_direct_field(key) || key.eq_ignore_ascii_case("name"))
                && root.values().all(Value::is_object);
            if is_named_server_map {
                normalize_mcp_member_name_in_map(root);
            } else if let Some(name) = root.get("name").and_then(Value::as_str) {
                root.insert(
                    "name".to_string(),
                    Value::String(normalized_mcp_member_name_or_original(name)),
                );
            }
        }
    }
    serde_json::to_string_pretty(&definition)
        .map_err(|err| format!("Serialize normalized MCP definition JSON failed: {err}"))
}

fn parse_mcp_definition_servers(
    definition_json: &str,
) -> Result<ParsedMcpDefinition, McpDefinitionValidationError> {
    let parsed: Value = serde_json::from_str(definition_json).map_err(|err| {
        McpDefinitionValidationError::from_issue(McpValidationIssue::new(
            "invalid_json",
            format!("MCP definition JSON parse failed: {err}"),
        ))
    })?;

    // 格式 4：根级数组
    if let Some(items) = parsed.as_array() {
        if items.is_empty() {
            return Err(McpDefinitionValidationError::from_issue(
                McpValidationIssue::new("empty_servers", "root array is empty".to_string()),
            ));
        }
        return parse_server_array(items, "root");
    }

    let root = parsed.as_object().ok_or_else(|| {
        McpDefinitionValidationError::from_issue(McpValidationIssue::new(
            "invalid_root",
            "MCP definition must be a JSON object or array".to_string(),
        ))
    })?;

    // mcpServers 键
    if let Some(servers_value) = root.get("mcpServers") {
        if let Some(servers) = servers_value.as_object() {
            return parse_named_server_map(servers, "mcpServers");
        }
        if let Some(items) = servers_value.as_array() {
            if items.is_empty() {
                return Err(McpDefinitionValidationError::from_issue(
                    McpValidationIssue::new("empty_servers", "mcpServers is empty".to_string()),
                ));
            }
            return parse_server_array(items, "mcpServers");
        }
        return Err(McpDefinitionValidationError::from_issue(
            McpValidationIssue::new(
                "mcp_servers_type_error",
                "mcpServers must be an object or an array".to_string(),
            ),
        ));
    }

    // 格式 2：根级平铺命名对象（无 server 直接字段，且所有值都是对象）
    let has_direct_field = root
        .keys()
        .any(|k| is_server_direct_field(k) || k.eq_ignore_ascii_case("name"));
    if !has_direct_field && root.values().all(Value::is_object) {
        return parse_named_server_map(root, "root");
    }

    // 格式 5：单 server 直接字段（向后兼容）
    let name = value_get_string(&parsed, "name").unwrap_or_else(|| "mcp-server".to_string());
    Ok(ParsedMcpDefinition {
        servers: vec![(name, parsed)],
        issues: Vec::new(),
    })
}

/// 展开 + 逐 server 字段校验，返回 (可用 servers, 全部 issues)。
/// 结构问题（缺 name、内部重名、类型错）与字段问题（缺 command/url、args/env 类型错）都聚合在 issues。
fn validate_mcp_definition_servers(
    definition_json: &str,
) -> (Vec<(String, Value)>, Vec<McpValidationIssue>) {
    match parse_mcp_definition_servers(definition_json) {
        Ok(parsed) => {
            let mut issues = parsed.issues;
            for (name, obj) in &parsed.servers {
                if let Some(obj_map) = obj.as_object() {
                    validate_single_server_obj(obj_map, name, &mut issues);
                }
            }
            (parsed.servers, issues)
        }
        Err(err) => (Vec::new(), err.issues),
    }
}

// ========== 单 server 解析（兼容既有调用方） ==========

/// 校验单个 server 对象，错误以 issues 返回
fn validate_single_server_obj(
    server_obj: &serde_json::Map<String, Value>,
    server_name: &str,
    issues: &mut Vec<McpValidationIssue>,
) {
    let has_command = server_obj
        .get("command")
        .and_then(Value::as_str)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_url = server_obj
        .get("url")
        .and_then(Value::as_str)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_command && !has_url {
        issues.push(
            McpValidationIssue::new(
                "server_missing_transport",
                format!("server '{server_name}' must include either non-empty command or url"),
            )
            .with_server(server_name),
        );
    }
    if let Some(args) = server_obj.get("args") {
        if !args.is_array() {
            issues.push(
                McpValidationIssue::new(
                    "args_type_error",
                    format!("server '{server_name}' args must be an array"),
                )
                .with_server(server_name)
                .with_field("args"),
            );
        } else if args
            .as_array()
            .map(|items| items.iter().any(|v| !v.is_string()))
            .unwrap_or(false)
        {
            issues.push(
                McpValidationIssue::new(
                    "args_item_type_error",
                    format!("server '{server_name}' args must be string array"),
                )
                .with_server(server_name)
                .with_field("args"),
            );
        }
    }
    for map_key in ["env", "headers", "httpHeaders", "envHttpHeaders"] {
        let Some(map_value) = server_obj.get(map_key) else {
            continue;
        };
        if !map_value.is_object() {
            issues.push(
                McpValidationIssue::new(
                    "map_type_error",
                    format!("server '{server_name}' {map_key} must be an object"),
                )
                .with_server(server_name)
                .with_field(map_key),
            );
            continue;
        }
        let map = map_value.as_object().expect("checked is_object above");
        for (k, v) in map {
            let value_ok = match v {
                Value::String(_) => true,
                Value::Object(obj) => obj
                    .get("value")
                    .map(Value::is_string)
                    .unwrap_or(false),
                _ => false,
            };
            if !value_ok {
                issues.push(
                    McpValidationIssue::new(
                        "map_value_type_error",
                        format!(
                            "server '{server_name}' {map_key}.{k} must be a string or {{value}} object"
                        ),
                    )
                    .with_server(server_name)
                    .with_field(&format!("{map_key}.{k}")),
                );
            }
        }
    }
}

fn parse_mcp_server_definition_from_value(
    _server_name: &str,
    root: &Value,
) -> Result<ParsedMcpServerDefinition, String> {
    let transport_text = value_get_string(root, "transport")
        .or_else(|| value_get_string(root, "type"))
        .unwrap_or_default()
        .to_ascii_lowercase();

    let command = value_get_string(root, "command");
    let url = value_get_string(root, "url");

    let transport = if transport_text == "sse" {
        McpTransportKind::Sse
    } else if matches!(
        transport_text.as_str(),
        "streamable_http" | "streamable-http" | "http" | "https" | "remote"
    ) {
        McpTransportKind::StreamableHttp
    } else if transport_text == "stdio" || transport_text == "local" {
        McpTransportKind::Stdio
    } else if command.is_some() {
        McpTransportKind::Stdio
    } else if url.is_some() {
        McpTransportKind::StreamableHttp
    } else {
        return Err("MCP definition must include either command(stdio) or url(streamable HTTP/sse)".to_string());
    };

    let args = value_get_string_array(root, "args");
    let env = value_get_map_string_string(root, "env");
    let cwd = value_get_string(root, "cwd");
    let bearer_token_env_var = value_get_string(root, "bearerTokenEnvVar")
        .or_else(|| value_get_string(root, "bearer_token_env_var"));
    // headers 作为 httpHeaders 的别名，两者都存在时合并
    let mut http_headers = value_get_map_string_string(root, "headers");
    for (k, v) in value_get_map_string_string(root, "httpHeaders") {
        http_headers.insert(k, v);
    }
    let env_http_headers = value_get_map_string_string(root, "envHttpHeaders");

    match transport {
        McpTransportKind::Stdio => {
            if command.as_deref().unwrap_or_default().trim().is_empty() {
                return Err("stdio MCP definition requires command".to_string());
            }
        }
        McpTransportKind::StreamableHttp => {
            if url.as_deref().unwrap_or_default().trim().is_empty() {
                return Err("streamable HTTP MCP definition requires url".to_string());
            }
        }
        McpTransportKind::Sse => {
            if url.as_deref().unwrap_or_default().trim().is_empty() {
                return Err("SSE MCP definition requires url".to_string());
            }
        }
    }

    Ok(ParsedMcpServerDefinition {
        transport,
        command,
        args,
        env,
        cwd,
        url,
        bearer_token_env_var,
        http_headers,
        env_http_headers,
    })
}

fn parse_mcp_server_definition(definition_json: &str) -> Result<(String, ParsedMcpServerDefinition), String> {
    let parsed = parse_mcp_definition_servers(definition_json)
        .map_err(|err| format!("{}", err.message))?;
    let (server_name, root) = parsed
        .servers
        .into_iter()
        .next()
        .ok_or_else(|| "MCP definition contains no servers".to_string())?;
    let parsed = parse_mcp_server_definition_from_value(&server_name, &root)?;
    Ok((server_name, parsed))
}

