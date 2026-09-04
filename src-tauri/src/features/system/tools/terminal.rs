use std::path::Path;

const TERMINAL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
const TERMINAL_DEFAULT_TIMEOUT_MS: u64 = 300_000;
// 前台 exec 与后台 shell 共用采集上限，防止日志把内存撑爆。
const DEFAULT_OUTPUT_BYTES_CAP: usize = 1024 * 1024;

include!("terminal/runtime.rs");

include!("terminal/output.rs");

include!("terminal/workspace.rs");

include!("terminal/matcher.rs");

include!("terminal/analyzer.rs");

include!("terminal/approval.rs");

include!("terminal/guards.rs");

include!("terminal/exec.rs");
