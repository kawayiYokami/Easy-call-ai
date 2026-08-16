use enigo::{Keyboard, Mouse};

fn ensure_dpi_awareness_once() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    let _ = ONCE.get_or_init(|| {
        #[cfg(target_os = "windows")]
        let _ = enigo::set_dpi_awareness();
    });
}

fn map_mouse_button(button: OperateMouseButton) -> enigo::Button {
    match button {
        OperateMouseButton::Left => enigo::Button::Left,
        OperateMouseButton::Right => enigo::Button::Right,
        OperateMouseButton::Middle => enigo::Button::Middle,
        OperateMouseButton::Back => enigo::Button::Back,
        OperateMouseButton::Forward => enigo::Button::Forward,
    }
}

fn map_input_err(err: enigo::InputError, context: &str) -> DesktopToolError {
    DesktopToolError::internal_error(format!("{context}: {err}"))
}

fn parse_named_key(name: &str) -> Option<enigo::Key> {
    let normalized = name.trim().to_lowercase().replace(['_', ' ', '-'], "");
    match normalized.as_str() {
        "ctrl" | "control" => Some(enigo::Key::Control),
        "lctrl" | "leftcontrol" => Some(enigo::Key::LControl),
        "rctrl" | "rightcontrol" => Some(enigo::Key::RControl),
        "shift" => Some(enigo::Key::Shift),
        "lshift" | "leftshift" => Some(enigo::Key::LShift),
        "rshift" | "rightshift" => Some(enigo::Key::RShift),
        "alt" | "option" => Some(enigo::Key::Alt),
        "meta" | "win" | "windows" | "command" | "cmd" => Some(enigo::Key::Meta),
        "enter" | "return" => Some(enigo::Key::Return),
        "tab" => Some(enigo::Key::Tab),
        "esc" | "escape" => Some(enigo::Key::Escape),
        "space" | "spacebar" => Some(enigo::Key::Space),
        "backspace" => Some(enigo::Key::Backspace),
        "delete" | "del" => Some(enigo::Key::Delete),
        "insert" => {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            { Some(enigo::Key::Insert) }
            #[cfg(target_os = "macos")]
            { None }
        }
        "up" | "arrowup" => Some(enigo::Key::UpArrow),
        "down" | "arrowdown" => Some(enigo::Key::DownArrow),
        "left" | "arrowleft" => Some(enigo::Key::LeftArrow),
        "right" | "arrowright" => Some(enigo::Key::RightArrow),
        "home" => Some(enigo::Key::Home),
        "end" => Some(enigo::Key::End),
        "pageup" => Some(enigo::Key::PageUp),
        "pagedown" => Some(enigo::Key::PageDown),
        "capslock" => Some(enigo::Key::CapsLock),
        "printscreen" => {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            { Some(enigo::Key::PrintScr) }
            #[cfg(target_os = "macos")]
            { None }
        }
        "pause" => {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            { Some(enigo::Key::Pause) }
            #[cfg(target_os = "macos")]
            { None }
        }
        "numlock" => {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            { Some(enigo::Key::Numlock) }
            #[cfg(target_os = "macos")]
            { None }
        }
        "f1" => Some(enigo::Key::F1),
        "f2" => Some(enigo::Key::F2),
        "f3" => Some(enigo::Key::F3),
        "f4" => Some(enigo::Key::F4),
        "f5" => Some(enigo::Key::F5),
        "f6" => Some(enigo::Key::F6),
        "f7" => Some(enigo::Key::F7),
        "f8" => Some(enigo::Key::F8),
        "f9" => Some(enigo::Key::F9),
        "f10" => Some(enigo::Key::F10),
        "f11" => Some(enigo::Key::F11),
        "f12" => Some(enigo::Key::F12),
        _ => None,
    }
}

fn parse_key(name: &str, line: usize) -> DesktopToolResult<ParsedKey> {
    if let Some(key) = parse_named_key(name) {
        return Ok(ParsedKey::Named(key));
    }
    let trimmed = name.trim();
    let mut chars = trimmed.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Ok(ParsedKey::Char(ch)),
        _ => Err(operate_line_error(line, "key", format!("非法：不支持的按键 `{name}`"))),
    }
}

/// 已解析的按键：命名键（Control/Enter/F1 等）或单字符键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedKey {
    Named(enigo::Key),
    Char(char),
}

/// 组合键按下保持时长：全部按键 Press 后等待该时长再释放，
/// 避免修饰键与普通键被系统/应用识别为两次独立点击。
const COMBO_KEY_PRESS_HOLD: std::time::Duration = std::time::Duration::from_millis(15);

/// ASCII 字符到 Windows 虚拟键（VK）的映射。组合键中的字母/数字/常见符号
/// 必须走真实按键事件才能触发系统与应用快捷键；非 ASCII 字符（如中文）返回 None。
#[cfg(target_os = "windows")]
fn char_to_vk(ch: char) -> Option<u16> {
    let lower = ch.to_ascii_lowercase();
    match lower {
        'a'..='z' => Some(0x41 + (lower as u16 - 'a' as u16)), // VK_A..VK_Z
        '0'..='9' => Some(0x30 + (lower as u16 - '0' as u16)), // VK_0..VK_9
        '-' => Some(0xBD),  // VK_OEM_MINUS
        '=' => Some(0xBB),  // VK_OEM_PLUS
        '[' => Some(0xDB),  // VK_OEM_4
        ']' => Some(0xDD),  // VK_OEM_6
        '\\' => Some(0xDC), // VK_OEM_5
        ';' => Some(0xBA),  // VK_OEM_1
        '\'' => Some(0xDE), // VK_OEM_7
        '`' => Some(0xC0),  // VK_OEM_3
        ',' => Some(0xBC),  // VK_OEM_COMMA
        '.' => Some(0xBE),  // VK_OEM_PERIOD
        '/' => Some(0xBF),  // VK_OEM_2
        ' ' => Some(0x20),  // VK_SPACE
        _ => None,
    }
}

/// 获取前台窗口线程的键盘布局，与 enigo 的 VK→scan 转换保持一致，
/// 避免 Tokio worker 线程布局与目标应用布局不一致导致符号键映射错位。
#[cfg(target_os = "windows")]
fn foreground_keyboard_layout() -> windows_sys::Win32::UI::Input::KeyboardAndMouse::HKL {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let foreground = unsafe { GetForegroundWindow() };
    let thread_id = unsafe { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) };
    unsafe { GetKeyboardLayout(thread_id) }
}

/// 发送单字符按键。组合键（prefer_real_key=true）在 Windows 上用
/// VK→scan code→enigo.raw 注入真实按键事件；单键文本输入保持 Unicode 注入
/// （绕过输入法直接上屏，现状行为不变）。组合键遇到不可映射字符直接报错，
/// 不回退 Unicode——enigo 对 Unicode 键的 Press/Release 会各自注入一次完整
/// 文本（down+up），回退会导致字符重复输入且快捷键仍不生效。
fn send_char_key(
    enigo: &mut enigo::Enigo,
    ch: char,
    direction: enigo::Direction,
    context: &str,
    prefer_real_key: bool,
) -> DesktopToolResult<()> {
    #[cfg(target_os = "windows")]
    if prefer_real_key {
        let Some(vk) = char_to_vk(ch) else {
            return Err(DesktopToolError::internal_error(format!(
                "{context}: 不支持的按键字符 `{ch}`，组合键请使用基础键并显式携带修饰键（如 Ctrl+Shift+/ 而非 Ctrl+?）"
            )));
        };
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyExW, MAPVK_VK_TO_VSC_EX};
        let scan = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC_EX, foreground_keyboard_layout()) };
        if scan != 0 {
            return enigo
                .raw(scan as u16, direction)
                .map_err(|err| map_input_err(err, context));
        }
        return Err(DesktopToolError::internal_error(format!(
            "{context}: 按键字符 `{ch}` 无法映射为扫描码（VK={vk:#04x}）"
        )));
    }
    enigo
        .key(enigo::Key::Unicode(ch), direction)
        .map_err(|err| map_input_err(err, context))
}

fn press_parsed_key(enigo: &mut enigo::Enigo, key: ParsedKey, prefer_real_key: bool) -> DesktopToolResult<()> {
    match key {
        ParsedKey::Named(k) => enigo.key(k, enigo::Direction::Press).map_err(|err| map_input_err(err, "key press failed")),
        ParsedKey::Char(ch) => send_char_key(enigo, ch, enigo::Direction::Press, "key press failed", prefer_real_key),
    }
}

fn release_parsed_key(enigo: &mut enigo::Enigo, key: ParsedKey, prefer_real_key: bool) -> DesktopToolResult<()> {
    match key {
        ParsedKey::Named(k) => enigo.key(k, enigo::Direction::Release).map_err(|err| map_input_err(err, "key release failed")),
        ParsedKey::Char(ch) => send_char_key(enigo, ch, enigo::Direction::Release, "key release failed", prefer_real_key),
    }
}

fn primary_monitor_bounds() -> DesktopToolResult<ScreenBounds> {
    let monitors = monitor_list()?;
    let monitor = resolve_primary_monitor(&monitors);
    let x = monitor.x().unwrap_or(0);
    let y = monitor.y().unwrap_or(0);
    let width = monitor.width().map_err(|err| DesktopToolError::internal_error(format!("read monitor width failed: {err}")))?;
    let height = monitor.height().map_err(|err| DesktopToolError::internal_error(format!("read monitor height failed: {err}")))?;
    Ok(ScreenBounds { x, y, width, height })
}

fn normalized_point_to_screen(point: &NormalizedPoint, bounds: &ScreenBounds) -> (i32, i32) {
    let max_x = bounds.width.saturating_sub(1) as f64;
    let max_y = bounds.height.saturating_sub(1) as f64;
    (bounds.x + (point.x * max_x).round() as i32, bounds.y + (point.y * max_y).round() as i32)
}

fn normalized_region_to_screen(region: &NormalizedRegion, bounds: &ScreenBounds) -> ScreenBounds {
    let width_f = bounds.width as f64;
    let height_f = bounds.height as f64;
    ScreenBounds {
        x: bounds.x + (region.x * width_f).round() as i32,
        y: bounds.y + (region.y * height_f).round() as i32,
        width: (region.width * width_f).round().max(1.0) as u32,
        height: (region.height * height_f).round().max(1.0) as u32,
    }
}

async fn sleep_duration(duration: std::time::Duration) {
    if !duration.is_zero() {
        tokio::time::sleep(duration).await;
    }
}

async fn execute_mouse_click(enigo: &mut enigo::Enigo, button: OperateMouseButton, target: &NormalizedPoint, repeat: u32, delay: std::time::Duration, pre_delay: std::time::Duration, press: std::time::Duration) -> DesktopToolResult<()> {
    sleep_duration(pre_delay).await;
    let bounds = primary_monitor_bounds()?;
    let (x, y) = normalized_point_to_screen(target, &bounds);
    enigo.move_mouse(x, y, enigo::Coordinate::Abs).map_err(|err| map_input_err(err, "move mouse failed"))?;
    let mapped = map_mouse_button(button);
    for idx in 0..repeat {
        if press.is_zero() {
            enigo.button(mapped, enigo::Direction::Click).map_err(|err| map_input_err(err, "mouse click failed"))?;
        } else {
            enigo.button(mapped, enigo::Direction::Press).map_err(|err| map_input_err(err, "mouse down failed"))?;
            sleep_duration(press).await;
            enigo.button(mapped, enigo::Direction::Release).map_err(|err| map_input_err(err, "mouse up failed"))?;
        }
        if idx + 1 < repeat {
            sleep_duration(delay).await;
        }
    }
    Ok(())
}

async fn execute_mouse_scroll(enigo: &mut enigo::Enigo, direction: i32, repeat: u32, delay: std::time::Duration, pre_delay: std::time::Duration) -> DesktopToolResult<()> {
    sleep_duration(pre_delay).await;
    for idx in 0..repeat {
        enigo.scroll(direction, enigo::Axis::Vertical).map_err(|err| map_input_err(err, "mouse scroll failed"))?;
        if idx + 1 < repeat {
            sleep_duration(delay).await;
        }
    }
    Ok(())
}

async fn execute_key_action(enigo: &mut enigo::Enigo, keys: &[String], line: usize, repeat: u32, delay: std::time::Duration, pre_delay: std::time::Duration, press: std::time::Duration) -> DesktopToolResult<()> {
    sleep_duration(pre_delay).await;
    let parsed = keys.iter().map(|key| parse_key(key, line)).collect::<DesktopToolResult<Vec<_>>>()?;
    for idx in 0..repeat {
        if parsed.len() == 1 && press.is_zero() {
            // 单键点击：命名键直接 tap，字符键走 Unicode 注入（输入场景直接上屏）
            match parsed[0] {
                ParsedKey::Named(key) => enigo.key(key, enigo::Direction::Click).map_err(|err| map_input_err(err, "key tap failed"))?,
                ParsedKey::Char(ch) => send_char_key(enigo, ch, enigo::Direction::Click, "key tap failed", false)?,
            }
        } else {
            // 组合键（或长按）：字符键必须注入真实按键事件，否则系统/应用快捷键不识别。
            // press 阶段任一键失败时，主动逆序释放已按下的键，避免修饰键残留（不依赖 Enigo Drop 兜底）。
            for (pressed_idx, key) in parsed.iter().enumerate() {
                if let Err(err) = press_parsed_key(enigo, *key, true) {
                    for released in parsed[..pressed_idx].iter().rev() {
                        let _ = release_parsed_key(enigo, *released, true);
                    }
                    return Err(err);
                }
            }
            let hold = if press.is_zero() { COMBO_KEY_PRESS_HOLD } else { press };
            sleep_duration(hold).await;
            for key in parsed.iter().rev() {
                release_parsed_key(enigo, *key, true)?;
            }
        }
        if idx + 1 < repeat {
            sleep_duration(delay).await;
        }
    }
    Ok(())
}

async fn execute_text_action(enigo: &mut enigo::Enigo, text: &str, repeat: u32, delay: std::time::Duration, pre_delay: std::time::Duration) -> DesktopToolResult<()> {
    sleep_duration(pre_delay).await;
    for idx in 0..repeat {
        execute_text_once(enigo, text).await?;
        if idx + 1 < repeat {
            sleep_duration(delay).await;
        }
    }
    Ok(())
}

/// 单次 text 注入。Windows 上含非 ASCII 字符时改走剪贴板粘贴：
/// enigo 的 KEYEVENTF_UNICODE（VK_PACKET）注入会被中文 IME 拦截进
/// composition 缓冲，与 Enter 交替时提交顺序错乱；剪贴板粘贴完全绕开
/// 键盘事件与 IME。纯 ASCII 保持 enigo 注入（避免无谓的剪贴板覆盖）。
#[cfg(target_os = "windows")]
async fn execute_text_once(enigo: &mut enigo::Enigo, text: &str) -> DesktopToolResult<()> {
    if contains_non_ascii(text) {
        let previous = read_clipboard_unicode_text();
        write_clipboard_unicode_text(text)?;
        let paste_result = (|| -> DesktopToolResult<()> {
            enigo
                .key(enigo::Key::Control, enigo::Direction::Press)
                .map_err(|err| map_input_err(err, "text paste failed"))?;
            // 'v' 走真实扫描码注入，确保系统识别 Ctrl+V 组合
            send_char_key(enigo, 'v', enigo::Direction::Click, "text paste failed", true)?;
            enigo
                .key(enigo::Key::Control, enigo::Direction::Release)
                .map_err(|err| map_input_err(err, "text paste failed"))?;
            Ok(())
        })();
        // Ctrl+V 是异步注入：事件进入系统队列后目标窗口还需时间处理粘贴。
        // 立即恢复剪贴板会抢跑，导致目标窗口粘贴到恢复后的旧值。
        // 等待粘贴处理完成（约 150ms 足够记事本等标准控件完成 WM_PASTE）。
        sleep_duration(std::time::Duration::from_millis(150)).await;
        restore_clipboard_unicode_text(previous);
        return paste_result;
    }
    enigo.text(text).map_err(|err| map_input_err(err, "text input failed"))
}

#[cfg(not(target_os = "windows"))]
async fn execute_text_once(enigo: &mut enigo::Enigo, text: &str) -> DesktopToolResult<()> {
    enigo.text(text).map_err(|err| map_input_err(err, "text input failed"))
}

/// 是否包含非 ASCII 字符：Windows 分支据此决定走剪贴板粘贴还是 enigo 注入。
#[cfg(target_os = "windows")]
fn contains_non_ascii(text: &str) -> bool {
    text.chars().any(|c| !c.is_ascii())
}

/// 读取剪贴板文本（CF_UNICODETEXT）；无文本格式时返回 None。
#[cfg(target_os = "windows")]
fn read_clipboard_unicode_text() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT as u32);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut len = 0usize;
        while *ptr.cast::<u16>().add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr.cast::<u16>(), len));
        GlobalUnlock(handle);
        CloseClipboard();
        Some(text)
    }
}

/// 写入剪贴板文本（CF_UNICODETEXT，UTF-16 + 结尾 NUL）。
#[cfg(target_os = "windows")]
fn write_clipboard_unicode_text(text: &str) -> DesktopToolResult<()> {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err(DesktopToolError::internal_error("open clipboard failed"));
        }
        if EmptyClipboard() == 0 {
            CloseClipboard();
            return Err(DesktopToolError::internal_error("empty clipboard failed"));
        }
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * std::mem::size_of::<u16>();
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if handle.is_null() {
            CloseClipboard();
            return Err(DesktopToolError::internal_error("global alloc failed"));
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return Err(DesktopToolError::internal_error("global lock failed"));
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), ptr.cast::<u8>(), bytes);
        GlobalUnlock(handle);
        if SetClipboardData(CF_UNICODETEXT as u32, handle).is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return Err(DesktopToolError::internal_error("set clipboard data failed"));
        }
        CloseClipboard();
    }
    Ok(())
}

/// 恢复剪贴板：有原文本则写回；无文本格式则清空（原非文本内容在
/// 写入时已被 EmptyClipboard 清除，此为计划内声明的限制）。
#[cfg(target_os = "windows")]
fn restore_clipboard_unicode_text(previous: Option<String>) {
    match previous {
        Some(text) => {
            let _ = write_clipboard_unicode_text(&text);
        }
        None => unsafe {
            use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                EmptyClipboard();
                CloseClipboard();
            }
        },
    }
}

/// 生成 operate 截图默认保存路径：{screenshots_root}/operate_{毫秒时间戳}.webp
fn default_operate_screenshot_path(screenshots_root: &std::path::Path) -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    screenshots_root
        .join(format!("operate_{ms}.webp"))
        .to_string_lossy()
        .to_string()
}

async fn execute_screenshot_action(
    mode: &ScreenshotModeSpec,
    save_path: Option<String>,
    quality: f32,
    screenshots_root: &std::path::Path,
    include_base64: bool,
    elements: bool,
) -> DesktopToolResult<(ScreenshotResponse, String, Option<Vec<UiElementInfo>>)> {
    let save_path = save_path.or_else(|| Some(default_operate_screenshot_path(screenshots_root)));
    let request = ScreenshotRequest {
        mode: match mode {
            ScreenshotModeSpec::Desktop | ScreenshotModeSpec::FocusedWindow => ScreenshotMode::Desktop,
            ScreenshotModeSpec::Region(_) => ScreenshotMode::Region,
        },
        monitor_id: None,
        region: match mode {
            ScreenshotModeSpec::Region(region) => {
                let bounds = primary_monitor_bounds()?;
                Some(normalized_region_to_screen(region, &bounds))
            }
            _ => None,
        },
        save_path,
        webp_quality: quality,
        include_base64,
    };
    let result = match mode {
        ScreenshotModeSpec::Desktop | ScreenshotModeSpec::Region(_) => run_screenshot_tool(request).await?,
        ScreenshotModeSpec::FocusedWindow => run_capture_window_tool(request, None)?,
    };
    let mode_name = match mode {
        ScreenshotModeSpec::Desktop => "desktop",
        ScreenshotModeSpec::FocusedWindow => "focused_window",
        ScreenshotModeSpec::Region(_) => "region",
    }
    .to_string();

    // elements=true：扫描可交互元素树（当前 Windows 实现；其他平台返回空并在 summary 提示）。
    // UIA 遍历是同步阻塞调用（数百 ms），放到阻塞线程池执行，避免占用 Tokio 工作线程。
    let tree = if elements {
        let mode_for_scan = mode.clone();
        Some(
            tokio::task::spawn_blocking(move || collect_ui_tree_for_mode(&mode_for_scan))
                .await
                .unwrap_or_default(),
        )
    } else {
        None
    };

    Ok((result, mode_name, tree))
}

/// 元素矩形与归一化 region 矩形是否有交集（region 为相对主屏 0~1 归一化）。
fn element_intersects_region(e: &UiElementInfo, rx0: f64, ry0: f64, rx1: f64, ry1: f64) -> bool {
    let (ex0, ey0) = (e.x, e.y);
    let (ex1, ey1) = (e.x + e.width, e.y + e.height);
    ex0 < rx1 && ex1 > rx0 && ey0 < ry1 && ey1 > ry0
}

/// 按截图模式扫描可交互元素树：focused_window 只扫聚焦窗口，desktop 扫全部可见窗口，
/// region 只返回与截图区域相交窗口的元素（元素矩形与 region 有交集才保留）。
fn collect_ui_tree_for_mode(mode: &ScreenshotModeSpec) -> Vec<UiElementInfo> {
    let bounds = match primary_monitor_bounds() {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let origin_x = bounds.x as f64;
    let origin_y = bounds.y as f64;
    let primary_width = bounds.width as f64;
    let primary_height = bounds.height as f64;

    let windows = match window_list() {
        Ok(w) => w,
        Err(_) => return Vec::new(),
    };
    // region 归一化矩形（相对主屏），用于窗口级与元素级过滤
    let region_rect = match mode {
        ScreenshotModeSpec::Region(region) => Some((
            region.x,
            region.y,
            region.x + region.width,
            region.y + region.height,
        )),
        _ => None,
    };
    let targets: Vec<(usize, String)> = match mode {
        ScreenshotModeSpec::FocusedWindow => windows
            .iter()
            .filter(|w| w.is_focused().unwrap_or(false))
            .map(|w| (w.id().unwrap_or(0) as usize, w.title().unwrap_or_default()))
            .collect(),
        ScreenshotModeSpec::Desktop => windows
            .iter()
            .map(|w| (w.id().unwrap_or(0) as usize, w.title().unwrap_or_default()))
            .collect(),
        ScreenshotModeSpec::Region(_) => windows
            .iter()
            // 窗口级过滤：窗口矩形与 region 有交集才扫，减少无用窗口的 UIA 遍历
            .filter(|w| {
                let Some((rx0, ry0, rx1, ry1)) = region_rect else { return false };
                let (wx0, wy0) = ((w.x().unwrap_or(0) - bounds.x) as f64 / primary_width, (w.y().unwrap_or(0) - bounds.y) as f64 / primary_height);
                let (wx1, wy1) = (
                    (w.x().unwrap_or(0) + w.width().unwrap_or(0) as i32 - bounds.x) as f64 / primary_width,
                    (w.y().unwrap_or(0) + w.height().unwrap_or(0) as i32 - bounds.y) as f64 / primary_height,
                );
                wx0 < rx1 && wx1 > rx0 && wy0 < ry1 && wy1 > ry0
            })
            .map(|w| (w.id().unwrap_or(0) as usize, w.title().unwrap_or_default()))
            .collect(),
    };
    let mut elements = collect_ui_tree_for_windows(&targets, origin_x, origin_y, primary_width, primary_height);
    if let Some((rx0, ry0, rx1, ry1)) = region_rect {
        // 元素级过滤：元素矩形与 region 有交集才保留（region 截图区域之外的元素不返回）
        elements.retain(|e| element_intersects_region(e, rx0, ry0, rx1, ry1));
    }
    elements
}

#[cfg(test)]
mod operate_actions_tests {
    use super::*;

    #[test]
    fn region_tree_should_filter_out_of_region_elements() {
        // region = @0.16,0.04,0.36,0.9（归一化矩形 x:0.16~0.52, y:0.04~0.94）
        let region = ScreenshotModeSpec::Region(NormalizedRegion { x: 0.16, y: 0.04, width: 0.36, height: 0.9 });
        let all = vec![
            // region 内
            UiElementInfo { window_id: 1, window_title: "in".into(), control_type: "Button".into(), name: "in".into(), x: 0.3, y: 0.5, width: 0.05, height: 0.05 },
            // 完全在 region 外（任务栏 y=0.958 场景）
            UiElementInfo { window_id: 2, window_title: "taskbar".into(), control_type: "Button".into(), name: "taskbar".into(), x: 0.3, y: 0.958, width: 0.05, height: 0.03 },
            // x 越界（Chrome 场景，y 高达 4.x）
            UiElementInfo { window_id: 3, window_title: "chrome".into(), control_type: "Button".into(), name: "chrome".into(), x: 0.3, y: 4.2, width: 0.05, height: 0.05 },
            // 部分相交：矩形左边缘在 region 内，右边缘超出
            UiElementInfo { window_id: 4, window_title: "partial".into(), control_type: "Edit".into(), name: "partial".into(), x: 0.4, y: 0.5, width: 0.3, height: 0.05 },
            // 负坐标（PAI 窗口主屏外元素）
            UiElementInfo { window_id: 5, window_title: "neg".into(), control_type: "Button".into(), name: "neg".into(), x: -0.2, y: 0.5, width: 0.05, height: 0.05 },
        ];
        let kept: Vec<&UiElementInfo> = all
            .iter()
            .filter(|e| element_intersects_region(e, 0.16, 0.04, 0.52, 0.94))
            .collect();
        let names: Vec<&str> = kept.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["in", "partial"]);
        let _ = region;
    }

    #[test]
    fn parse_key_should_keep_named_key_as_named() {
        assert_eq!(parse_key("Control", 1).unwrap(), ParsedKey::Named(enigo::Key::Control));
        assert_eq!(parse_key("Enter", 1).unwrap(), ParsedKey::Named(enigo::Key::Return));
        assert_eq!(parse_key("F5", 1).unwrap(), ParsedKey::Named(enigo::Key::F5));
    }

    #[test]
    fn parse_key_should_treat_single_char_as_char() {
        assert_eq!(parse_key("L", 1).unwrap(), ParsedKey::Char('L'));
        assert_eq!(parse_key("a", 1).unwrap(), ParsedKey::Char('a'));
        assert_eq!(parse_key("0", 1).unwrap(), ParsedKey::Char('0'));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn contains_non_ascii_should_detect_non_ascii_text() {
        assert!(!contains_non_ascii("hello world 123"));
        assert!(contains_non_ascii("派蒙和旅行者"));
        assert!(contains_non_ascii("中文标点「」"));
        assert!(contains_non_ascii("emoji \u{1F600}"));
        assert!(!contains_non_ascii(""));
    }

    #[test]
    fn parse_key_should_reject_multi_char_unknown() {
        let err = parse_key("ab", 1).unwrap_err();
        assert!(err.message.contains("不支持的按键"));
    }

    #[test]
    fn key_combo_should_parse_mixed_named_and_char() {
        // 组合键 = 命名键 + 字符键，字符键必须能被识别为 Char，
        // 才能在后端注入真实按键事件触发快捷键
        let action = parse_script(&OperateRequest { script: "key Control+L".to_string(), timeout_ms: None })
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        match action {
            DesktopScriptAction::Key { keys, .. } => {
                let parsed = keys.iter().map(|key| parse_key(key, 1)).collect::<DesktopToolResult<Vec<_>>>().unwrap();
                assert_eq!(parsed, vec![ParsedKey::Named(enigo::Key::Control), ParsedKey::Char('L')]);
            }
            _ => panic!("expected key action"),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn char_to_vk_should_map_ascii_keys() {
        assert_eq!(char_to_vk('a'), Some(0x41)); // VK_A
        assert_eq!(char_to_vk('z'), Some(0x5A)); // VK_Z
        assert_eq!(char_to_vk('A'), Some(0x41)); // 大小写同 VK，由 Shift 修饰键区分
        assert_eq!(char_to_vk('0'), Some(0x30)); // VK_0
        assert_eq!(char_to_vk('9'), Some(0x39)); // VK_9
        assert_eq!(char_to_vk('-'), Some(0xBD)); // VK_OEM_MINUS
        assert_eq!(char_to_vk(' '), Some(0x20)); // VK_SPACE
        assert_eq!(char_to_vk('/'), Some(0xBF)); // VK_OEM_2
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn char_to_vk_should_not_map_non_ascii() {
        assert_eq!(char_to_vk('中'), None);
        assert_eq!(char_to_vk('你'), None);
    }

    #[test]
    fn normalized_region_should_include_screen_offsets() {
        let region = NormalizedRegion {
            x: 0.25,
            y: 0.5,
            width: 0.4,
            height: 0.25,
        };
        let bounds = ScreenBounds {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        let screen = normalized_region_to_screen(&region, &bounds);

        assert_eq!(screen.x, 300);
        assert_eq!(screen.y, 500);
        assert_eq!(screen.width, 320);
        assert_eq!(screen.height, 150);
    }

    #[test]
    fn default_operate_screenshot_path_should_be_named_with_timestamp() {
        let root = std::path::Path::new("C:/tmp/screenshots");
        let path = default_operate_screenshot_path(root);
        let file_name = std::path::Path::new(&path)
            .file_name()
            .expect("path should have a file name")
            .to_string_lossy()
            .to_string();
        assert!(file_name.starts_with("operate_"), "unexpected file name: {file_name}");
        assert!(file_name.ends_with(".webp"), "unexpected file name: {file_name}");
        let ms_part = file_name
            .trim_start_matches("operate_")
            .trim_end_matches(".webp");
        assert!(
            ms_part.parse::<u128>().is_ok(),
            "timestamp part should be numeric: {file_name}"
        );
        // 默认路径必须落在传入的会话截图根目录下（按会话建目录）。
        let parent = std::path::Path::new(&path)
            .parent()
            .expect("path should have a parent");
        let parent_norm = parent.to_string_lossy().replace('\\', "/");
        assert_eq!(
            parent_norm, "C:/tmp/screenshots",
            "default path must stay inside the per-conversation screenshots root"
        );
    }
}
