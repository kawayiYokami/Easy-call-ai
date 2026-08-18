#[cfg(target_os = "windows")]
fn exec_windows_process_compatible_path(path: &std::path::Path) -> std::path::PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        return std::path::PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(target_os = "windows")]
fn exec_windows_wrap_command_for_shell(
    shell: &TerminalShellProfile,
    command: &str,
) -> String {
    if matches!(shell.kind.as_str(), "powershell7" | "powershell5") {
        return format!(
            "$ErrorActionPreference='Continue'; try {{ [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false); [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); $OutputEncoding = [Console]::OutputEncoding; chcp.com 65001 > $null; $env:PYTHONUTF8='1'; $env:PYTHONIOENCODING='utf-8'; {command} }} catch {{ Write-Error $_; $global:LASTEXITCODE = 1 }}; exit $(if ($null -eq $LASTEXITCODE) {{ 0 }} else {{ $LASTEXITCODE }})"
        );
    }
    if shell.kind == "git-bash" {
        return format!("chcp.com 65001 > /dev/null 2>&1; export LANG=en_US.UTF-8; export LC_ALL=en_US.UTF-8; export PYTHONUTF8=1; export PYTHONIOENCODING=utf-8; {command}");
    }
    command.to_string()
}

/// 唤醒 CREATE_SUSPENDED 创建的子进程主线程。
/// 进程入 job 前处于挂起状态，杜绝「先执行再入 job」的竞态；入 job 成功后恢复执行。
#[cfg(target_os = "windows")]
fn exec_windows_resume_primary_thread(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
        THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("CreateToolhelp32Snapshot failed".to_string());
    }
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
    let mut thread_id = None;
    if unsafe { Thread32First(snapshot, &mut entry) } != 0 {
        loop {
            if entry.th32OwnerProcessID == pid {
                thread_id = Some(entry.th32ThreadID);
                break;
            }
            if unsafe { Thread32Next(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    unsafe {
        CloseHandle(snapshot);
    }
    let Some(thread_id) = thread_id else {
        return Err(format!("找不到 pid={pid} 的主线程"));
    };
    let thread_handle = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, thread_id) };
    if thread_handle.is_null() {
        return Err(format!("OpenThread 失败 pid={pid} tid={thread_id}"));
    }
    let resume_count = unsafe { ResumeThread(thread_handle) };
    unsafe {
        CloseHandle(thread_handle);
    }
    if resume_count == u32::MAX {
        return Err(format!("ResumeThread 失败 pid={pid} tid={thread_id}"));
    }
    Ok(())
}

/// 有界等待 Windows reader 线程结束并取回输出；读取失败、join 失败都转成终端执行错误。
/// 带 deadline：调用方已尝试关闭管道后仍不结束（后代进程持有句柄）时，超时返回错误，
/// 避免超时路径自身永久挂起。
#[cfg(target_os = "windows")]
fn join_windows_reader(
    handle: std::thread::JoinHandle<Result<Vec<u8>, std::io::Error>>,
    name: &str,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, String> {
    while !handle.is_finished() {
        if std::time::Instant::now() >= deadline {
            // reader 线程仍阻塞在 read_to_end（后代进程持有管道写端），无法强制取消。
            // 不能直接 drop JoinHandle 让线程脱离：把句柄交给收割线程去 join，
            // 收割线程与 reader 生命周期绑定——调用方已 kill 进程树，管道写端终会关闭，
            // reader EOF 后收割线程退出，不会跨调用累积。
            let reaper = std::thread::spawn(move || {
                let _ = handle.join();
            });
            let _ = reaper;
            return Err(format!("Join {name} reader thread timed out"));
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    match handle.join() {
        Ok(Ok(buf)) => Ok(buf),
        Ok(Err(err)) => Err(format!("terminal_exec read {name} failed: {err}")),
        Err(_) => Err(format!("Join {name} reader thread failed")),
    }
}

#[cfg(target_os = "windows")]
fn exec_run_with_windows_job_backend_blocking(
    shell: &TerminalShellProfile,
    request: &ExecutionRequest,
) -> Result<ExecutionResult, String> {
    use std::io::Read as _;
    use std::os::windows::io::AsRawHandle as _;
    use std::os::windows::process::CommandExt as _;

    use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};

    let mut command_builder = std::process::Command::new(&shell.path);
    let cwd = exec_windows_process_compatible_path(&request.cwd);
    let wrapped_command = exec_windows_wrap_command_for_shell(shell, &request.command);
    command_builder.current_dir(&cwd);
    command_builder.stdout(std::process::Stdio::piped());
    command_builder.stderr(std::process::Stdio::piped());
    command_builder.stdin(std::process::Stdio::null());
    // 挂起创建：进程入 job 前不执行任何代码，从根上消除「先派生后代再入 job」的竞态
    command_builder.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    terminal_apply_windows_utf8_env(&mut command_builder);
    for arg in &shell.args_prefix {
        command_builder.arg(arg);
    }
    command_builder.arg(&wrapped_command);

    let mut child = command_builder
        .spawn()
        .map_err(|err| format!("terminal_exec windows command backend spawn failed: {err}"))?;

    // 进程尚处挂起，先入 job 再恢复：保证整棵进程树都受 kill-on-close 管辖
    // Keep process tree cleanup on timeout/exit, but do not cap child process count:
    // Git Bash bootstrap may spawn helper processes during startup.
    let job = WindowsJobGuard::create_kill_on_close()?;
    if let Err(err) = job.assign_raw_process_handle(
        child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
    ) {
        let _ = child.kill();
        return Err(format!("{}: pid={}", err, child.id()));
    }
    if let Err(err) = exec_windows_resume_primary_thread(child.id()) {
        let _ = child.kill();
        return Err(err);
    }

    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "Capture child stdout failed.".to_string())?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "Capture child stderr failed.".to_string())?;

    // 读取失败必须传播，不能静默忽略：调用方需要知道输出不完整
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = Vec::<u8>::new();
        stdout_pipe.read_to_end(&mut buf).map(|_| buf)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = Vec::<u8>::new();
        stderr_pipe.read_to_end(&mut buf).map(|_| buf)
    });

    let timeout_ms = request.timeout_ms.max(1);
    let started = std::time::Instant::now();
    loop {
        if let Some(_status) = child
            .try_wait()
            .map_err(|err| format!("terminal_exec try_wait failed: {err}"))?
        {
            break;
        }
        if started.elapsed().as_millis() >= timeout_ms as u128 {
            drop(job);
            let _ = child.kill();
            let cleanup_started = std::time::Instant::now();
            while cleanup_started.elapsed().as_millis() < 2_000 {
                if child
                    .try_wait()
                    .map_err(|err| format!("terminal_exec cleanup try_wait failed: {err}"))?
                    .is_some()
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            // kill 后管道写端关闭，reader 线程应读到 EOF 结束；仍有后代持有句柄
            // 时线程不结束，用有界等待兜底并显式收集错误
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_millis(2_000);
            let mut read_errors = Vec::<String>::new();
            if let Err(err) = join_windows_reader(stdout_reader, "stdout", deadline) {
                read_errors.push(err);
            }
            if let Err(err) = join_windows_reader(stderr_reader, "stderr", deadline) {
                read_errors.push(err);
            }
            let mut message = format!("terminal_exec timed out after {}ms", timeout_ms);
            if !read_errors.is_empty() {
                message.push_str(&format!("（{}）", read_errors.join("; ")));
            }
            return Err(message);
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    let status = child
        .wait()
        .map_err(|err| format!("terminal_exec wait failed: {err}"))?;
    // Important: close the job as soon as the root process exits so descendant
    // processes do not keep inherited stdout/stderr handles alive forever.
    drop(job);
    // 正常路径同样有界回收：保证在调用方视角 read 已完成，出错时给出明确错误
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_millis(2_000);
    let stdout = join_windows_reader(stdout_reader, "stdout", deadline)?;
    let stderr = join_windows_reader(stderr_reader, "stderr", deadline)?;
    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

    Ok(ExecutionResult {
        ok: status.success(),
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
        shell_kind: shell.kind.clone(),
        shell_path: shell.path.clone(),
    })
}

#[cfg(target_os = "windows")]
async fn exec_run_with_windows_job_backend(
    shell: &TerminalShellProfile,
    request: &ExecutionRequest,
) -> Result<ExecutionResult, String> {
    let shell = shell.clone();
    let request = request.clone();
    tokio::task::spawn_blocking(move || {
        exec_run_with_windows_job_backend_blocking(&shell, &request)
    })
    .await
    .map_err(|err| format!("Join windows command backend worker failed: {err}"))?
}

#[cfg(all(test, target_os = "windows"))]
mod exec_windows_backend_tests {
    use super::*;

    #[test]
    fn powershell_wrapper_should_enable_utf8_before_user_command() {
        let shell = TerminalShellProfile {
            kind: "powershell7".to_string(),
            path: "pwsh.exe".to_string(),
            args_prefix: vec!["-NoProfile".to_string(), "-Command".to_string()],
        };
        let wrapped = exec_windows_wrap_command_for_shell(&shell, "Write-Output 'hi'");
        assert!(wrapped.contains("InputEncoding"));
        assert!(wrapped.contains("OutputEncoding"));
        assert!(wrapped.contains("chcp.com 65001"));
        assert!(wrapped.contains("PYTHONUTF8"));
        assert!(wrapped.contains("PYTHONIOENCODING"));
        assert!(wrapped.contains("Write-Output 'hi'"));
    }

    #[test]
    fn git_bash_wrapper_should_export_utf8_locale() {
        let shell = TerminalShellProfile {
            kind: "git-bash".to_string(),
            path: "bash.exe".to_string(),
            args_prefix: vec!["-lc".to_string()],
        };
        let wrapped = exec_windows_wrap_command_for_shell(&shell, "echo hi");
        assert!(wrapped.contains("chcp.com 65001"));
        assert!(wrapped.contains("LANG=en_US.UTF-8"));
        assert!(wrapped.contains("LC_ALL=en_US.UTF-8"));
        assert!(wrapped.contains("PYTHONUTF8=1"));
        assert!(wrapped.contains("PYTHONIOENCODING=utf-8"));
        assert!(wrapped.ends_with("echo hi"));
    }
}