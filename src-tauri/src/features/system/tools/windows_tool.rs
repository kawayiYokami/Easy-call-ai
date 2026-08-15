// ==================== windows 工具：列出窗口 / 激活窗口 ====================
//
// 独立于 operate 的窗口管理工具：模型可用 list windows 获取全量窗口清单
// （含 PAI 自身窗口，xcap 枚举会过滤当前进程窗口，这里用 EnumWindows 全量枚举），
// 再用 activate window 把目标窗口切到前台，配合 operate 的 focused_window 截图使用。
//
// 语法（一行一个动作，与 operate 同风格）：
//   list windows
//   activate window id=<windowId 或 0x 前缀十六进制>

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(description = "窗口管理脚本请求。只接收一个 script 字段；script 必须是多行字符串，一行一个动作。")]
struct WindowsRequest {
    #[schemars(description = "窗口管理脚本文本，一行一个动作。")]
    script: String,
    #[serde(default)]
    #[schemars(description = "本次工具调用的超时时间，单位毫秒；未指定时默认 60000ms。")]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsResponse {
    ok: bool,
    executed_count: usize,
    steps: Vec<WindowsStepResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsStepResult {
    kind: WindowsStepKind,
    summary: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    windows: Option<Vec<WindowInfo>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum WindowsStepKind {
    ListWindows,
    ActivateWindow,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowInfo {
    window_id: u32,
    title: String,
    process_id: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    minimized: bool,
    focused: bool,
}

#[derive(Debug, Clone)]
enum WindowsAction {
    ListWindows { line: usize },
    ActivateWindow { line: usize, window_id: u32 },
}

fn windows_invalid(message: impl Into<String>) -> DesktopToolError {
    DesktopToolError::invalid_params(message)
}

fn parse_windows_script(script: &str) -> DesktopToolResult<Vec<WindowsAction>> {
    let mut actions = Vec::new();
    for (idx, raw_line) in script.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens = tokenize_script_line(trimmed)
            .map_err(|err| windows_invalid(format!("第 {line_no} 行：{err}")))?;
        if tokens.is_empty() {
            continue;
        }
        let head = tokens[0].to_ascii_lowercase();
        let action = match head.as_str() {
            "list" => {
                if tokens.len() < 2 || tokens[1].to_ascii_lowercase() != "windows" {
                    return Err(windows_invalid(format!("第 {line_no} 行：list 后必须是 windows，当前为 `{}`", tokens.join(" "))));
                }
                WindowsAction::ListWindows { line: line_no }
            }
            "activate" => {
                if tokens.len() < 2 || tokens[1].to_ascii_lowercase() != "window" {
                    return Err(windows_invalid(format!("第 {line_no} 行：activate 后必须是 window，当前为 `{}`", tokens.join(" "))));
                }
                let id_raw = tokens[2..].join(" ");
                // 兼容 `id=xxx` 与裸 `xxx` 两种写法（描述以 id= 形式给出）
                let id_raw = id_raw
                    .strip_prefix("id=")
                    .or_else(|| id_raw.strip_prefix("ID="))
                    .unwrap_or(&id_raw);
                let id = parse_window_id(id_raw)
                    .ok_or_else(|| windows_invalid(format!("第 {line_no} 行：activate 缺少合法 id，当前为 `{id_raw}`")))?;
                WindowsAction::ActivateWindow { line: line_no, window_id: id }
            }
            other => return Err(windows_invalid(format!("第 {line_no} 行：不支持的窗口动作 `{other}`"))),
        };
        actions.push(action);
    }
    Ok(actions)
}

/// 解析窗口 id：支持十进制数字（windowId）或 0x 前缀十六进制。
fn parse_window_id(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u32>().ok()
    }
}

fn run_windows_tool(input: WindowsRequest) -> DesktopToolResult<WindowsResponse> {
    let actions = parse_windows_script(&input.script)?;
    let mut steps = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            WindowsAction::ListWindows { line } => {
                let windows = list_all_windows();
                let step = WindowsStepResult {
                    kind: WindowsStepKind::ListWindows,
                    summary: format!("windows listed, count={}", windows.len()),
                    ok: true,
                    windows: Some(windows),
                };
                runtime_log_info(format!(
                    "[窗口工具] 步骤完成，任务=run_windows_tool，line={}，kind=ListWindows，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
            WindowsAction::ActivateWindow { line, window_id } => {
                let (title, activated) = activate_window(window_id);
                let step = WindowsStepResult {
                    kind: WindowsStepKind::ActivateWindow,
                    summary: format!("activate window id={window_id} title={title:?} activated={activated}"),
                    ok: activated,
                    windows: None,
                };
                runtime_log_info(format!(
                    "[窗口工具] 步骤完成，任务=run_windows_tool，line={}，kind=ActivateWindow，summary={}",
                    line, step.summary
                ));
                steps.push(step);
            }
        }
    }
    Ok(WindowsResponse {
        ok: steps.iter().all(|s| s.ok),
        executed_count: steps.len(),
        steps,
    })
}

/// 全量枚举顶层可见窗口（含当前进程窗口；xcap 的 window_list 会过滤当前进程窗口，这里不用它）。
/// 返回按 z-order 排序（EnumWindows 顺序）的窗口信息列表。
#[cfg(target_os = "windows")]
fn list_all_windows() -> Vec<WindowInfo> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId,
        IsIconic, IsWindow, IsWindowVisible, EnumWindows, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };
    use std::sync::Mutex;

    struct EnumCtx {
        windows: Vec<WindowInfo>,
    }
    let ctx = Mutex::new(EnumCtx { windows: Vec::new() });
    let ctx_ptr = &ctx as *const Mutex<EnumCtx> as isize;

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
        let ctx = unsafe { &*(lparam as *const Mutex<EnumCtx>) };
        let mut guard = match ctx.lock() {
            Ok(g) => g,
            Err(_) => return 1,
        };
        // 只枚举顶层可见窗口；跳过工具窗口（WS_EX_TOOLWINDOW）和无标题的不可见窗口
        if unsafe { IsWindow(hwnd) != 0 } && unsafe { IsWindowVisible(hwnd) != 0 } {
            let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
            if ex_style & WS_EX_TOOLWINDOW != 0 {
                return 1;
            }
            let title = read_window_title(hwnd);
            if title.trim().is_empty() {
                return 1;
            }
            let mut pid = 0u32;
            unsafe {
                GetWindowThreadProcessId(hwnd, &mut pid);
            }
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            unsafe {
                GetWindowRect(hwnd, &mut rect);
            }
            let minimized = unsafe { IsIconic(hwnd) != 0 };
            let fg = unsafe { GetForegroundWindow() };
            guard.windows.push(WindowInfo {
                window_id: hwnd as u32,
                title,
                process_id: pid,
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
                minimized,
                focused: fg == hwnd,
            });
        }
        1
    }

    unsafe {
        EnumWindows(Some(enum_proc), ctx_ptr as isize);
    }
    match ctx.into_inner() {
        Ok(inner) => inner.windows,
        Err(_) => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn list_all_windows() -> Vec<WindowInfo> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn read_window_title(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

/// 激活窗口：还原最小化 → 绕前台锁 → SetForegroundWindow + BringWindowToTop → 轮询验证。
/// 返回 (窗口标题, 是否激活成功)。
#[cfg(target_os = "windows")]
fn activate_window(window_id: u32) -> (String, bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    let hwnd = window_id as *mut core::ffi::c_void;
    if hwnd.is_null() {
        return (String::new(), false);
    }
    let title = read_window_title(hwnd);
    unsafe {
        // 最小化则先还原
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // Alt 键技巧：模拟一次按键让系统认为有用户输入，解除前台锁定（等价于 AttachThreadInput 的绕锁效果）
        keybd_event(VK_MENU as u8, 0, 0, 0);
        keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
        let _ = SetForegroundWindow(hwnd);
        let _ = BringWindowToTop(hwnd);
        // 轮询验证（最多 1.5s）
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
        let mut activated = false;
        while std::time::Instant::now() < deadline {
            if GetForegroundWindow() == hwnd {
                activated = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        (title, activated)
    }
}

#[cfg(not(target_os = "windows"))]
fn activate_window(_window_id: u32) -> (String, bool) {
    (String::new(), false)
}

#[cfg(test)]
mod windows_tool_tests {
    use super::*;

    #[test]
    fn parse_window_id_should_support_decimal_and_hex() {
        assert_eq!(parse_window_id("123"), Some(123));
        assert_eq!(parse_window_id("0x1A2B"), Some(0x1A2B));
        assert_eq!(parse_window_id("0Xff"), Some(255));
        assert_eq!(parse_window_id("abc"), None);
        assert_eq!(parse_window_id(""), None);
    }

    #[test]
    fn parse_script_should_accept_list_and_activate() {
        let actions = parse_windows_script("list windows\nactivate window 0x1234\n").unwrap();
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], WindowsAction::ListWindows { .. }));
        assert!(matches!(actions[1], WindowsAction::ActivateWindow { window_id: 0x1234, .. }));
    }

    #[test]
    fn parse_script_should_accept_id_equals_form() {
        // 描述以 id= 形式给出，也兼容裸 id
        let actions = parse_windows_script("activate window id=7541586\nactivate window 7541586\n").unwrap();
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], WindowsAction::ActivateWindow { window_id: 7541586, .. }));
        assert!(matches!(actions[1], WindowsAction::ActivateWindow { window_id: 7541586, .. }));
    }

    /// 真实桌面冒烟测试：枚举窗口 + 激活前台窗口验证。默认忽略，手动 `--ignored` 跑。
    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "需要真实 Windows 桌面"]
    fn real_desktop_should_list_and_activate_windows() {
        let windows = list_all_windows();
        assert!(!windows.is_empty(), "桌面应有可见窗口");
        eprintln!("[probe] total windows: {}", windows.len());
        for w in windows.iter().take(8) {
            eprintln!("[probe] id=0x{:x} title={:?} minimized={} focused={} rect=({},{},{},{})", w.window_id, w.title, w.minimized, w.focused, w.x, w.y, w.width, w.height);
        }
        // 激活当前前台窗口：应成功且标题一致
        if let Some(fg) = windows.iter().find(|w| w.focused) {
            let (title, activated) = activate_window(fg.window_id);
            eprintln!("[probe] activate focused window 0x{:x}: title={:?} activated={}", fg.window_id, title, activated);
            assert!(activated, "激活前台窗口应成功");
        }
    }

    #[test]
    fn parse_script_should_reject_unknown_action() {
        assert!(parse_windows_script("list windows\nfoo bar\n").is_err());
        assert!(parse_windows_script("activate\n").is_err());
        assert!(parse_windows_script("list\n").is_err());
        assert!(parse_windows_script("activate window abc\n").is_err());
    }
}
