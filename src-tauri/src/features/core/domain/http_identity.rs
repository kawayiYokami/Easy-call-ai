
fn app_http_user_agent() -> String {
    format!(
        "{}/{} ({}; tauri)",
        APP_HTTP_ORIGINATOR,
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    )
}

fn app_identity_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::HeaderName::from_static("originator"),
        reqwest::header::HeaderValue::from_static(APP_HTTP_ORIGINATOR),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_str(&app_http_user_agent())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static(APP_HTTP_ORIGINATOR)),
    );
    headers
}

fn app_identity_genai_headers() -> genai::Headers {
    genai::Headers::from([
        ("originator".to_string(), APP_HTTP_ORIGINATOR.to_string()),
        ("user-agent".to_string(), app_http_user_agent()),
    ])
}

const CODEX_TUI_COMPAT_VERSION: &str = "0.139.0";
const CODEX_BETA_FEATURES: &str = "terminal_resize_reflow";

fn codex_cli_sandbox_label() -> &'static str {
    "none"
}

fn codex_cli_user_agent(originator: &str) -> String {
    let version = CODEX_TUI_COMPAT_VERSION;
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "windows") {
        let os_info = os_info::get();
        return format!(
            "{originator}/{version} (Windows {}; {arch}) WindowsTerminal ({originator}; {version})",
            os_info.version()
        );
    }

    let os_info = os_info::get();
    format!(
        "{originator}/{version} ({} {}; {arch}) {originator} ({originator}; {version})",
        os_info.os_type(),
        os_info.version()
    )
}

fn codex_genai_headers(
    originator: &str,
    session_id: Option<&str>,
    thread_id: Option<&str>,
    residency_requirement: Option<&str>,
) -> genai::Headers {
    let user_agent = codex_cli_user_agent(originator);

    let mut pairs = vec![
        ("Originator".to_string(), originator.to_string()),
        ("User-Agent".to_string(), user_agent),
        ("X-Codex-Beta-Features".to_string(), CODEX_BETA_FEATURES.to_string()),
    ];

    if let Some(id) = session_id.map(str::trim).filter(|value| !value.is_empty()) {
        let thread_id = thread_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(id);
        let window_id = format!("{id}:0");
        let turn_metadata = serde_json::json!({
            "session_id": id,
            "thread_id": thread_id,
            "thread_source": "user",
            "turn_id": uuid::Uuid::new_v4().to_string(),
            "workspaces": {},
            "sandbox": codex_cli_sandbox_label(),
            "turn_started_at_unix_ms": chrono::Utc::now().timestamp_millis(),
            "request_kind": "turn",
            "window_id": window_id,
        });
        pairs.push(("Session-Id".to_string(), id.to_string()));
        pairs.push(("Thread-Id".to_string(), thread_id.to_string()));
        pairs.push(("X-Client-Request-Id".to_string(), id.to_string()));
        pairs.push(("X-Codex-Window-Id".to_string(), format!("{id}:0")));
        pairs.push(("X-Codex-Turn-Metadata".to_string(), turn_metadata.to_string()));
    }

    if let Some(requirement) = residency_requirement.map(str::trim).filter(|value| !value.is_empty()) {
        pairs.push((
            "x-openai-internal-codex-residency".to_string(),
            requirement.to_string(),
        ));
    }

    genai::Headers::from(pairs)
}
