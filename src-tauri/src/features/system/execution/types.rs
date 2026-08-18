/// reader 清理的共享超时（毫秒）：进程被终止后等待输出管道 EOF 的上限。
/// Windows 与 Unix 后端共用，防止后代进程持有管道写端时 reader 永久挂起。
const READER_CLEANUP_TIMEOUT_MS: u64 = 2_000;

#[derive(Debug, Clone)]
struct ExecutionRequest {
    session_id: String,
    command: String,
    cwd: std::path::PathBuf,
    timeout_ms: u64,
    cwd_policy_exempt: bool,
}

#[derive(Debug, Clone)]
struct ExecutionResult {
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