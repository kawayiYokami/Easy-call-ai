#[derive(Debug, Clone)]
struct SandboxRequest {
    session_id: String,
    command: String,
    cwd: std::path::PathBuf,
    timeout_ms: u64,
    cwd_pre_validated: bool,
}

#[derive(Debug, Clone)]
struct SandboxExecutionResult {
    ok: bool,
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    duration_ms: u64,
    #[allow(dead_code)]
    shell_kind: String,
    #[allow(dead_code)]
    shell_path: String,
}
