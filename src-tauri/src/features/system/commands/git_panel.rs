// ==================== Git 面板 ====================
// 统一 git CLI 封装 + 白名单命令面。
// 所有命令只接收业务字段（路径、消息、hash 等），不接受任意 args；
// 参数安全在 Rust 侧硬编码约束，前端无法注入任意 git 参数。

use tokio::process::Command as AsyncCommand;

// ---------- 输入结构 ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelWorkspaceInput {
    workspace_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelPathsInput {
    workspace_path: String,
    paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCommitInput {
    workspace_path: String,
    message: String,
    amend: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelStashInput {
    workspace_path: String,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelStashRefInput {
    workspace_path: String,
    stash_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelBranchInput {
    workspace_path: String,
    name: String,
    start_point: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCheckoutInput {
    workspace_path: String,
    reference: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelShowInput {
    workspace_path: String,
    hash: String,
    /// 为空时查看整个提交的 diff
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelLogInput {
    workspace_path: String,
    limit: Option<usize>,
    skip: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCommitFilesInput {
    workspace_path: String,
    hash: String,
}

// ---------- 输出结构 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelDetectOutput {
    git_available: bool,
    repo_root: Option<String>,
    checked: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelRunOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelStatusEntry {
    path: String,
    staged_status: String,
    unstaged_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelStatusOutput {
    repo_root: String,
    branch: String,
    entries: Vec<GitPanelStatusEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelDiffOutput {
    diff: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCommitFileEntry {
    path: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCommitFilesOutput {
    entries: Vec<GitPanelCommitFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelBranchEntry {
    name: String,
    is_current: bool,
    is_remote: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelRemoteEntry {
    name: String,
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelStashEntry {
    reference: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelLogEntry {
    hash: String,
    short_hash: String,
    author: String,
    date: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelLogOutput {
    entries: Vec<GitPanelLogEntry>,
}

// ---------- 路径与参数校验 ----------

fn git_panel_validate_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim().to_string();
    if trimmed.is_empty() {
        return Err("路径不能为空".to_string());
    }
    if trimmed.contains('\0') {
        return Err("路径包含非法字符".to_string());
    }
    if trimmed.starts_with('-') {
        return Err("路径不能以 - 开头".to_string());
    }
    Ok(trimmed)
}

fn git_panel_validate_paths(paths: &[String]) -> Result<Vec<String>, String> {
    if paths.is_empty() {
        return Err("路径列表不能为空".to_string());
    }
    paths.iter().map(|item| git_panel_validate_path(item)).collect()
}

fn git_panel_validate_reference(reference: &str) -> Result<String, String> {
    let trimmed = reference.trim().to_string();
    if trimmed.is_empty() {
        return Err("引用不能为空".to_string());
    }
    if trimmed.contains('\0') {
        return Err("引用包含非法字符".to_string());
    }
    if trimmed.starts_with('-') {
        return Err("引用不能以 - 开头".to_string());
    }
    Ok(trimmed)
}

fn git_panel_validate_message(message: &str) -> Result<String, String> {
    let trimmed = message.trim().to_string();
    if trimmed.is_empty() {
        return Err("提交信息不能为空".to_string());
    }
    if trimmed.contains('\0') {
        return Err("提交信息包含非法字符".to_string());
    }
    Ok(trimmed)
}

fn git_panel_validate_hash(hash: &str) -> Result<String, String> {
    let trimmed = hash.trim().to_string();
    if trimmed.is_empty() {
        return Err("提交哈希不能为空".to_string());
    }
    if trimmed.contains('\0') || trimmed.contains(' ') {
        return Err("提交哈希格式非法".to_string());
    }
    Ok(trimmed)
}

fn git_panel_validate_branch_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("分支名不能为空".to_string());
    }
    if trimmed.contains('\0') || trimmed.contains(' ') || trimmed.starts_with('-') {
        return Err("分支名格式非法".to_string());
    }
    Ok(trimmed)
}

// ---------- git CLI 封装 ----------

fn git_panel_spawn(workdir: &str, args: &[&str]) -> AsyncCommand {
    let mut command = AsyncCommand::new("git");
    command
        .current_dir(workdir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(target_os = "windows")]
    {
        // 避免 Git 进程在 GUI 应用中创建短暂可见的控制台窗口。
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

/// 执行 git 命令，返回 stdout/stderr/exit_code（不因非零退出码报错，由调用方判断）。
async fn git_panel_run_raw(workdir: &str, args: &[&str]) -> Result<GitPanelRunOutput, String> {
    let output = git_panel_spawn(workdir, args)
        .output()
        .await
        .map_err(|err| format!("无法运行 git：{err}"))?;
    Ok(GitPanelRunOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// 执行 git 命令，成功（退出码 0）返回 stdout，失败返回可读错误。
async fn git_panel_run(workdir: &str, args: &[&str]) -> Result<String, String> {
    let result = git_panel_run_raw(workdir, args).await?;
    if result.exit_code != 0 {
        let stderr = result.stderr.trim();
        let stdout = result.stdout.trim();
        let detail = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            format!("退出码 {}", result.exit_code)
        };
        let cmd = args.first().copied().unwrap_or("git");
        return Err(format!("git {cmd} 失败：{detail}"));
    }
    Ok(result.stdout)
}

/// 执行 git 网络命令（fetch/pull/push），带超时控制；超时后终止子进程并返回可读错误。
async fn git_panel_run_network(workdir: &str, args: &[&str]) -> Result<GitPanelRunOutput, String> {
    let mut command = git_panel_spawn(workdir, args);
    // future 被取消（超时）时自动杀死子进程，避免远端无响应时进程残留。
    command.kill_on_drop(true);
    let wait = command.output();
    // 远端无响应时避免前端无限等待；60 秒足够覆盖大仓库传输。
    match tokio::time::timeout(std::time::Duration::from_secs(60), wait).await {
        Ok(result) => {
            let output = result.map_err(|err| format!("无法运行 git：{err}"))?;
            Ok(GitPanelRunOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
        Err(_) => Err("git 网络操作超时：远端无响应，已终止".to_string()),
    }
}

/// 在 workspace_path 向上探测仓库根（git rev-parse --show-toplevel）。
async fn git_panel_resolve_root(workspace_path: &str) -> Result<String, String> {
    let normalized = git_panel_validate_path(workspace_path)?;
    let result = git_panel_run_raw(&normalized, &["rev-parse", "--show-toplevel"]).await?;
    if result.exit_code != 0 {
        let stderr = result.stderr.trim();
        if stderr.contains("not a git repository") || stderr.contains("不是 git 仓库") {
            return Err("当前目录不是 Git 仓库".to_string());
        }
        return Err(format!("Git 仓库探测失败：{}", stderr));
    }
    let root = result.stdout.trim().to_string();
    if root.is_empty() {
        return Err("Git 仓库探测失败：无法解析仓库根".to_string());
    }
    Ok(root)
}

/// 获取当前分支名（detached HEAD 时返回空）。
async fn git_panel_current_branch(workdir: &str) -> String {
    git_panel_run(workdir, &["branch", "--show-current"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn git_panel_parse_status_entry(record: &str) -> Option<GitPanelStatusEntry> {
    // porcelain v1 -z 的记录格式：XY path；rename/copy 会附带第二条 old-path 记录，
    // 该记录开头不是合法的 XY 状态列（两字符 + 空格），直接跳过。
    let mut chars = record.chars();
    let staged = chars.next()?;
    let unstaged = chars.next()?;
    if chars.next()? != ' ' {
        return None;
    }
    let rest = chars.as_str();
    let path = rest.trim_start_matches(' ').to_string();
    if path.is_empty() {
        return None;
    }
    // rename/copy 时 rest 形如 "new -> old"，取 -> 前的目标路径
    let display_path = path.split(" -> ").next().unwrap_or(&path).to_string();
    Some(GitPanelStatusEntry {
        path: display_path,
        staged_status: staged.to_string(),
        unstaged_status: unstaged.to_string(),
    })
}

/// 从 porcelain v1 -z 记录中提取文件路径（忽略 rename 的旧路径记录）。
fn git_panel_parse_status_path(record: &str) -> Option<String> {
    let mut chars = record.chars();
    chars.next()?;
    chars.next()?;
    if chars.next()? != ' ' {
        return None;
    }
    let path = chars.as_str().trim_start_matches(' ').to_string();
    if path.is_empty() {
        return None;
    }
    let display_path = path.split(" -> ").next().unwrap_or(&path).to_string();
    Some(display_path)
}

// ---------- 命令：探测 ----------

#[tauri::command]
async fn git_panel_detect(input: GitPanelWorkspaceInput) -> Result<GitPanelDetectOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let version = git_panel_run_raw(&workspace_path, &["--version"]).await;
    let Ok(version_output) = version else {
        return Ok(GitPanelDetectOutput {
            git_available: false,
            repo_root: None,
            checked: false,
            error: Some("无法运行 git 命令".to_string()),
        });
    };
    if version_output.exit_code != 0 {
        return Ok(GitPanelDetectOutput {
            git_available: false,
            repo_root: None,
            checked: false,
            error: Some(version_output.stderr.trim().to_string()),
        });
    }
    match git_panel_resolve_root(&workspace_path).await {
        Ok(root) => Ok(GitPanelDetectOutput {
            git_available: true,
            repo_root: Some(root),
            checked: true,
            error: None,
        }),
        Err(err) => Ok(GitPanelDetectOutput {
            git_available: true,
            repo_root: None,
            checked: true,
            error: Some(err),
        }),
    }
}

// ---------- 命令：状态 ----------

#[tauri::command]
async fn git_panel_status(input: GitPanelWorkspaceInput) -> Result<GitPanelStatusOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let branch = git_panel_current_branch(&repo_root).await;
    let stdout = git_panel_run(
        &repo_root,
        &["status", "--porcelain=v1", "-z", "-uall"],
    )
    .await?;
    let entries: Vec<GitPanelStatusEntry> = stdout
        .split('\0')
        .filter(|record| !record.is_empty())
        .filter_map(git_panel_parse_status_entry)
        .collect();
    Ok(GitPanelStatusOutput {
        repo_root,
        branch,
        entries,
    })
}

// ---------- 命令：diff ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelDiffInput {
    workspace_path: String,
    path: String,
    /// staged=true 表示暂存区 vs HEAD（git diff --cached）；false 表示工作区 vs 暂存区
    staged: bool,
    /// 提供 hash 时查看某次提交的该文件 diff（git show hash -- path）
    hash: String,
}

#[tauri::command]
async fn git_panel_diff(input: GitPanelDiffInput) -> Result<GitPanelDiffOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let path = git_panel_validate_path(&input.path)?;

    let diff = if !input.hash.trim().is_empty() {
        let hash = git_panel_validate_hash(&input.hash)?;
        git_panel_run(&repo_root, &["show", "--format=", "--no-ext-diff", &hash, "--", &path]).await?
    } else if input.staged {
        git_panel_run(&repo_root, &["diff", "--cached", "--no-ext-diff", "--", &path]).await?
    } else {
        git_panel_run(&repo_root, &["diff", "--no-ext-diff", "--", &path]).await?
    };
    Ok(GitPanelDiffOutput { diff })
}

// ---------- 命令：暂存 / 取消暂存 ----------

#[tauri::command]
async fn git_panel_stage(input: GitPanelPathsInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let paths = git_panel_validate_paths(&input.paths)?;
    let mut args: Vec<&str> = vec!["add", "--"];
    let path_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    args.extend(path_refs);
    git_panel_run_raw(&repo_root, &args).await
}

#[tauri::command]
async fn git_panel_unstage(input: GitPanelPathsInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let paths = git_panel_validate_paths(&input.paths)?;
    let mut args: Vec<&str> = vec!["restore", "--staged", "--"];
    let path_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    args.extend(path_refs);
    git_panel_run_raw(&repo_root, &args).await
}

// ---------- 命令：提交 ----------

#[tauri::command]
async fn git_panel_commit(input: GitPanelCommitInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let message = git_panel_validate_message(&input.message)?;
    let mut args: Vec<&str> = vec!["commit"];
    if input.amend {
        args.push("--amend");
    }
    args.push("-m");
    args.push(&message);
    git_panel_run_raw(&repo_root, &args).await
}

// ---------- 命令：丢弃 ----------

#[tauri::command]
async fn git_panel_discard(input: GitPanelPathsInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let paths = git_panel_validate_paths(&input.paths)?;
    // 一次性丢弃 staged + worktree 改动，回到 HEAD（VS Code discard 语义）
    let mut args: Vec<&str> = vec!["restore", "--staged", "--worktree", "--"];
    let path_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    args.extend(path_refs);
    git_panel_run_raw(&repo_root, &args).await
}

// ---------- 命令：储藏 ----------

#[tauri::command]
async fn git_panel_stash_list(input: GitPanelWorkspaceInput) -> Result<Vec<GitPanelStashEntry>, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let stdout = git_panel_run(&repo_root, &["stash", "list"]).await?;
    let entries = stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let reference = trimmed.split(':').next().unwrap_or("").trim().to_string();
            let message = trimmed
                .splitn(2, ':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            if reference.is_empty() {
                return None;
            }
            Some(GitPanelStashEntry { reference, message })
        })
        .collect();
    Ok(entries)
}

#[tauri::command]
async fn git_panel_stash_create(input: GitPanelStashInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let message = input.message.trim().to_string();
    let mut args: Vec<&str> = vec!["stash", "push"];
    if !message.is_empty() {
        args.push("-m");
        args.push(&message);
    }
    git_panel_run_raw(&repo_root, &args).await
}

#[tauri::command]
async fn git_panel_stash_apply(input: GitPanelStashRefInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.stash_ref)?;
    git_panel_run_raw(&repo_root, &["stash", "apply", &reference]).await
}

#[tauri::command]
async fn git_panel_stash_pop(input: GitPanelStashRefInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.stash_ref)?;
    git_panel_run_raw(&repo_root, &["stash", "pop", &reference]).await
}

#[tauri::command]
async fn git_panel_stash_drop(input: GitPanelStashRefInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.stash_ref)?;
    git_panel_run_raw(&repo_root, &["stash", "drop", &reference]).await
}

#[tauri::command]
async fn git_panel_stash_files(input: GitPanelStashRefInput) -> Result<GitPanelCommitFilesOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.stash_ref)?;
    // --name-status 输出 "状态\t路径"；--no-renames 保证 rename 按 删+增 输出
    let stdout = git_panel_run(&repo_root, &["stash", "show", "--name-status", "--no-renames", &reference])
        .await?;
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(3, '\t');
        let Some(status) = parts.next() else { continue };
        let Some(path) = parts.next() else { continue };
        if status.is_empty() || path.is_empty() {
            continue;
        }
        entries.push(GitPanelCommitFileEntry {
            path: path.to_string(),
            status: status.chars().next().unwrap_or('?').to_string(),
        });
    }
    Ok(GitPanelCommitFilesOutput { entries })
}

// ---------- 命令：分支 ----------

#[tauri::command]
async fn git_panel_branch_list(input: GitPanelWorkspaceInput) -> Result<Vec<GitPanelBranchEntry>, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let current = git_panel_current_branch(&repo_root).await;
    let stdout = git_panel_run(&repo_root, &["branch", "-a", "--no-color"]).await?;
    let entries = stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (is_current, name) = if let Some(rest) = line.strip_prefix('*') {
                (true, rest.trim().to_string())
            } else {
                (false, line.trim().to_string())
            };
            if name.is_empty() {
                return None;
            }
            let is_remote = name.starts_with("remotes/");
            let display_name = if is_remote {
                name.strip_prefix("remotes/").unwrap_or(&name).to_string()
            } else {
                name
            };
            Some(GitPanelBranchEntry {
                is_current: is_current || display_name == current,
                name: display_name,
                is_remote,
            })
        })
        .collect();
    Ok(entries)
}

#[tauri::command]
async fn git_panel_branch_create(input: GitPanelBranchInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let name = git_panel_validate_branch_name(&input.name)?;
    let start_point = if input.start_point.trim().is_empty() {
        String::new()
    } else {
        git_panel_validate_reference(&input.start_point)?
    };
    let mut args: Vec<&str> = vec!["branch", &name];
    if !start_point.is_empty() {
        args.push(&start_point);
    }
    git_panel_run_raw(&repo_root, &args).await
}

#[tauri::command]
async fn git_panel_branch_delete(input: GitPanelBranchInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let name = git_panel_validate_branch_name(&input.name)?;
    // -d 仅删除已合并分支；未合并时 git 会拒绝并提示，前端可再确认用 -D
    git_panel_run_raw(&repo_root, &["branch", "-d", &name]).await
}

// ---------- 命令：签出 ----------

#[tauri::command]
async fn git_panel_checkout(input: GitPanelCheckoutInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.reference)?;
    git_panel_run_raw(&repo_root, &["checkout", &reference]).await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelCheckoutCheckOutput {
    /// 工作区未提交/未跟踪的文件路径
    dirty_paths: Vec<String>,
    /// 目标分支相对当前 HEAD 修改的文件路径
    changed_paths: Vec<String>,
    /// 交集：切换会覆盖或冲突的文件路径
    conflicting_paths: Vec<String>,
}

/// 切换分支预检：对比工作区未提交文件与目标分支改动，找出会冲突的文件
#[tauri::command]
async fn git_panel_checkout_check(input: GitPanelCheckoutInput) -> Result<GitPanelCheckoutCheckOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let reference = git_panel_validate_reference(&input.reference)?;

    // 工作区未提交/未跟踪文件（含重命名等，-z 按 NUL 分隔）
    let status_stdout = git_panel_run(
        &repo_root,
        &["status", "--porcelain=v1", "-z", "-uall"],
    )
    .await?;
    let dirty_paths: Vec<String> = status_stdout
        .split('\0')
        .filter(|record| !record.is_empty())
        .filter_map(git_panel_parse_status_path)
        .collect();

    // 目标分支相对当前 HEAD 修改的文件
    let changed_stdout = git_panel_run(
        &repo_root,
        &["diff", "--name-only", "HEAD", &reference],
    )
    .await?;
    let changed_paths: Vec<String> = changed_stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    let dirty_set: std::collections::HashSet<&String> = dirty_paths.iter().collect();
    let conflicting_paths: Vec<String> = changed_paths
        .iter()
        .filter(|path| dirty_set.contains(*path))
        .cloned()
        .collect();

    Ok(GitPanelCheckoutCheckOutput {
        dirty_paths,
        changed_paths,
        conflicting_paths,
    })
}

// ---------- 命令：远程 ----------

#[tauri::command]
async fn git_panel_remote_list(input: GitPanelWorkspaceInput) -> Result<Vec<GitPanelRemoteEntry>, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let stdout = git_panel_run(&repo_root, &["remote", "-v"]).await?;
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(url) = parts.next() else { continue };
        if seen.insert(name.to_string()) {
            entries.push(GitPanelRemoteEntry {
                name: name.to_string(),
                url: url.to_string(),
            });
        }
    }
    Ok(entries)
}

// ---------- 命令：同步 ----------

#[tauri::command]
async fn git_panel_fetch(input: GitPanelWorkspaceInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    git_panel_run_network(&repo_root, &["fetch"]).await
}

#[tauri::command]
async fn git_panel_pull(input: GitPanelWorkspaceInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    git_panel_run_network(&repo_root, &["pull"]).await
}

#[tauri::command]
async fn git_panel_push(input: GitPanelWorkspaceInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    git_panel_run_network(&repo_root, &["push"]).await
}

#[tauri::command]
async fn git_panel_sync(input: GitPanelWorkspaceInput) -> Result<GitPanelRunOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let fetch_result = git_panel_run_network(&repo_root, &["fetch"]).await?;
    if fetch_result.exit_code != 0 {
        return Ok(fetch_result);
    }
    git_panel_run_network(&repo_root, &["pull"]).await
}

// ---------- 命令：历史与提交图 ----------

#[tauri::command]
async fn git_panel_log(input: GitPanelLogInput) -> Result<GitPanelLogOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let limit = input.limit.unwrap_or(100).clamp(1, 500);
    let skip = input.skip.unwrap_or(0);

    let mut args: Vec<String> = vec![
        "log".to_string(),
        "-n".to_string(),
        limit.to_string(),
        "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%B%x1e".to_string(),
    ];
    if skip > 0 {
        args.push("--skip".to_string());
        args.push(skip.to_string());
    }
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let stdout = git_panel_run(&repo_root, &args_ref).await?;
    let entries = stdout
        .split('\u{1e}')
        .filter_map(|record| {
            let trimmed = record.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut parts = trimmed.splitn(5, '\u{1f}');
            let hash = parts.next().unwrap_or("").to_string();
            let short_hash = parts.next().unwrap_or("").to_string();
            let author = parts.next().unwrap_or("").to_string();
            let date = parts.next().unwrap_or("").to_string();
            let message = parts.next().unwrap_or("").trim().to_string();
            if hash.is_empty() {
                return None;
            }
            Some(GitPanelLogEntry {
                hash,
                short_hash,
                author,
                date,
                message,
            })
        })
        .collect();
    Ok(GitPanelLogOutput { entries })
}

// ---------- 命令：提交 diff ----------

#[tauri::command]
async fn git_panel_show(input: GitPanelShowInput) -> Result<GitPanelDiffOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let hash = git_panel_validate_hash(&input.hash)?;
    let path = input.path.trim().to_string();
    // stash 是合并提交：git show 只会输出 combined diff（diff --cc），可读性差；
    // 改走 diff 第一父提交（stash 创建时的 HEAD），得到标准 diff
    let is_stash = hash.starts_with("stash@{");
    let diff = if path.is_empty() {
        if is_stash {
            git_panel_run(&repo_root, &["diff", &format!("{hash}^1"), &hash]).await?
        } else {
            git_panel_run(&repo_root, &["show", "--format=", "--no-ext-diff", &hash]).await?
        }
    } else {
        let path = git_panel_validate_path(&path)?;
        if is_stash {
            git_panel_run(
                &repo_root,
                &["diff", &format!("{hash}^1"), &hash, "--", &path],
            )
            .await?
        } else {
            git_panel_run(
                &repo_root,
                &["show", "--format=", "--no-ext-diff", &hash, "--", &path],
            )
            .await?
        }
    };
    Ok(GitPanelDiffOutput { diff })
}

// ---------- 命令：提交文件列表 ----------

#[tauri::command]
async fn git_panel_commit_files(input: GitPanelCommitFilesInput) -> Result<GitPanelCommitFilesOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    let repo_root = git_panel_resolve_root(&workspace_path).await?;
    let hash = git_panel_validate_hash(&input.hash)?;
    // --name-status 输出 "状态\t路径"；--no-renames 保证 rename 也按 删+增 输出，避免旧路径行干扰
    let stdout = git_panel_run(
        &repo_root,
        &["show", "--format=", "--name-status", "--no-renames", &hash],
    )
    .await?;
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(3, '\t');
        let Some(status) = parts.next() else { continue };
        let Some(path) = parts.next() else { continue };
        if status.is_empty() || path.is_empty() {
            continue;
        }
        entries.push(GitPanelCommitFileEntry {
            path: path.to_string(),
            status: status.chars().next().unwrap_or('?').to_string(),
        });
    }
    Ok(GitPanelCommitFilesOutput { entries })
}

// ---------- 命令：仓库列表（多仓库切换） ----------

/// 仓库扫描最大深度：相对 workspace_path 的目录层级数（workspace 自身为 0）。
const GIT_REPO_SCAN_MAX_DEPTH: usize = 3;

/// 扫描时跳过的构建/依赖目录。
const GIT_REPO_SCAN_SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", "build"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelRepoEntry {
    path: String,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitPanelReposOutput {
    repos: Vec<GitPanelRepoEntry>,
}

/// 扫描结果缓存：workspace_path → 仓库列表；展开时只读缓存，点刷新（refresh=true）才重扫。
static GIT_REPO_SCAN_CACHE: OnceLock<Mutex<HashMap<String, Vec<GitPanelRepoEntry>>>> = OnceLock::new();

fn git_panel_repo_cache() -> &'static Mutex<HashMap<String, Vec<GitPanelRepoEntry>>> {
    GIT_REPO_SCAN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 仓库根路径比较（Windows 下忽略大小写）。
fn git_panel_repo_path_eq(a: &str, b: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        a.eq_ignore_ascii_case(b)
    }
    #[cfg(not(target_os = "windows"))]
    {
        a == b
    }
}

/// 取路径最后一段作为仓库名。
fn git_panel_repo_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string())
}

/// 从 workspace_path 向下递归扫描 .git（目录或文件都算），每个命中的父目录即仓库根。
/// 不进入 .git 内部；跳过忽略名单目录；深度不超过 max_depth。
fn git_panel_scan_repos_inner(workspace_path: &str, max_depth: usize) -> Vec<GitPanelRepoEntry> {
    let mut repos = Vec::new();
    let mut stack = vec![(workspace_path.to_string(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" {
                repos.push(GitPanelRepoEntry {
                    path: dir.clone(),
                    name: git_panel_repo_name(&dir),
                });
                continue;
            }
            if GIT_REPO_SCAN_SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            if depth + 1 > max_depth {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push((entry.path().to_string_lossy().to_string(), depth + 1));
            }
        }
    }
    repos
}

#[tauri::command]
async fn git_panel_repos(
    input: GitPanelWorkspaceInput,
    refresh: bool,
) -> Result<GitPanelReposOutput, String> {
    let workspace_path = git_panel_validate_path(&input.workspace_path)?;
    // 缓存命中且非强制刷新：直接返回
    {
        let cache = git_panel_repo_cache()
            .lock()
            .map_err(|_| "仓库列表缓存读取失败".to_string())?;
        if !refresh {
            if let Some(cached) = cache.get(&workspace_path) {
                return Ok(GitPanelReposOutput {
                    repos: cached.clone(),
                });
            }
        }
    }
    // 扫描走阻塞线程，避免卡住异步 runtime
    let scanned = tokio::task::spawn_blocking({
        let workspace_path = workspace_path.clone();
        move || git_panel_scan_repos_inner(&workspace_path, GIT_REPO_SCAN_MAX_DEPTH)
    })
    .await
    .map_err(|err| format!("仓库扫描失败：{err}"))?;
    // 合并当前仓库根（可能在工作区上方，扫描不到；恒保留在列表中）
    let mut repos = scanned;
    if let Ok(root) = git_panel_resolve_root(&workspace_path).await {
        if !repos.iter().any(|repo| git_panel_repo_path_eq(&repo.path, &root)) {
            repos.push(GitPanelRepoEntry {
                path: root.clone(),
                name: git_panel_repo_name(&root),
            });
        }
    }
    repos.sort_by(|a, b| a.path.cmp(&b.path));
    git_panel_repo_cache()
        .lock()
        .map_err(|_| "仓库列表缓存写入失败".to_string())?
        .insert(workspace_path, repos.clone());
    Ok(GitPanelReposOutput { repos })
}

#[cfg(test)]
mod git_panel_repos_tests {
    use super::*;

    /// 构造临时目录树（用完由 guard 清理）：
    /// - root/.git                     → root 仓库（深度 0）
    /// - root/sub1/.git                → sub1 仓库（深度 1）
    /// - root/sub1/inner/.git          → inner 仓库（深度 2）
    /// - root/node_modules/dep/.git    → 忽略名单内，不命中
    /// - root/sub2/a/b/.git            → 深度 3 仓库（边界内）
    /// - root/sub2/a/b/deeper/.git     → 深度 4，超限不命中
    /// - root/wt/.git（文件）           → worktree 模拟，命中
    fn make_fixture() -> (TempDirGuard, String) {
        let base = std::env::temp_dir().join(format!(
            "git-panel-repos-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        for rel in [
            ".git",
            "sub1/.git",
            "sub1/inner/.git",
            "node_modules/dep/.git",
            "sub2/a/b/.git",
            "sub2/a/b/deeper/.git",
        ] {
            std::fs::create_dir_all(base.join(rel)).expect("创建 fixture 失败");
        }
        std::fs::create_dir_all(base.join("wt")).expect("创建 fixture 失败");
        std::fs::write(base.join("wt/.git"), "gitdir: ../.git/worktrees/wt")
            .expect("创建 worktree 模拟失败");
        let root = base.to_string_lossy().to_string();
        (TempDirGuard(base), root)
    }

    struct TempDirGuard(std::path::PathBuf);

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn scans_nested_repos_within_depth_and_skips_ignored() {
        let (_guard, root) = make_fixture();
        let repos = git_panel_scan_repos_inner(&root, GIT_REPO_SCAN_MAX_DEPTH);
        let paths: Vec<&str> = repos.iter().map(|repo| repo.path.as_str()).collect();
        let sub1 = Path::new(&root).join("sub1").to_string_lossy().into_owned();
        let inner = Path::new(&root).join("sub1").join("inner").to_string_lossy().into_owned();
        let sub3 = Path::new(&root).join("sub2").join("a").join("b").to_string_lossy().into_owned();
        let ignored = Path::new(&root).join("node_modules").join("dep").to_string_lossy().into_owned();
        let deep = Path::new(&root).join("sub2").join("a").join("b").join("deeper").to_string_lossy().into_owned();
        let wt = Path::new(&root).join("wt").to_string_lossy().into_owned();
        assert!(paths.contains(&root.as_str()), "深度 0 仓库应命中，实际 {paths:?}");
        assert!(paths.contains(&sub1.as_str()), "深度 1 仓库应命中");
        assert!(paths.contains(&inner.as_str()), "深度 2 仓库应命中");
        assert!(paths.contains(&sub3.as_str()), "深度 3 仓库应命中（边界内）");
        assert!(!paths.contains(&ignored.as_str()), "忽略名单目录内的仓库不应命中");
        assert!(!paths.contains(&deep.as_str()), "超过深度上限的仓库不应命中");
        assert!(paths.contains(&wt.as_str()), ".git 文件（worktree）也应命中");
    }

    #[test]
    fn repo_name_takes_last_segment() {
        assert_eq!(git_panel_repo_name("E:\\github\\easy_call_ai"), "easy_call_ai");
        assert_eq!(git_panel_repo_name("/home/user/project"), "project");
    }

    #[test]
    fn repo_path_eq_ignores_case_on_windows() {
        #[cfg(target_os = "windows")]
        assert!(git_panel_repo_path_eq("E:\\GitHub\\Demo", "e:\\github\\demo"));
        #[cfg(not(target_os = "windows"))]
        assert!(!git_panel_repo_path_eq("/A/B", "/a/b"));
    }
}
