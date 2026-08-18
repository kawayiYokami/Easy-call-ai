#[derive(Debug, Clone, Copy)]
enum ExecutionBackendKind {
    #[cfg(target_os = "windows")]
    WindowsJobBackend,
    #[cfg(not(target_os = "windows"))]
    ProcessBackend,
}

#[derive(Debug, Clone, Copy)]
struct ExecutionManager {
    backend: ExecutionBackendKind,
}

impl ExecutionManager {
    fn from_state(_state: &AppState) -> Self {
        #[cfg(target_os = "windows")]
        {
            return Self {
                backend: ExecutionBackendKind::WindowsJobBackend,
            };
        }

        #[cfg(not(target_os = "windows"))]
        Self {
            backend: ExecutionBackendKind::ProcessBackend,
        }
    }

    async fn run(
        &self,
        state: &AppState,
        request: ExecutionRequest,
    ) -> Result<ExecutionResult, String> {
        // Defense in depth: backend entrance re-checks cwd policy unless the
        // request is explicitly exempt. The exempt path is for trusted callers
        // that already resolved cwd through their own workspace checks.
        if !request.cwd_policy_exempt {
            assert_cwd_allowed(state.clone(), request.session_id.clone(), request.cwd.clone())
                .await?;
        }
        let runtime_shell = terminal_shell_for_state(state);
        match self.backend {
            #[cfg(target_os = "windows")]
            ExecutionBackendKind::WindowsJobBackend => {
                exec_run_with_windows_job_backend(&runtime_shell, &request).await
            }
            #[cfg(not(target_os = "windows"))]
            ExecutionBackendKind::ProcessBackend => {
                exec_run_with_process_backend(&runtime_shell, &request).await
            }
        }
    }
}

async fn run_command_in_workspace(
    state: &AppState,
    session_id: &str,
    command: &str,
    cwd: &std::path::Path,
    timeout_ms: u64,
    cwd_policy_exempt: bool,
) -> Result<ExecutionResult, String> {
    let manager = ExecutionManager::from_state(state);
    let request = ExecutionRequest {
        session_id: session_id.to_string(),
        command: command.to_string(),
        cwd: cwd.to_path_buf(),
        timeout_ms,
        cwd_policy_exempt,
    };
    manager.run(state, request).await
}