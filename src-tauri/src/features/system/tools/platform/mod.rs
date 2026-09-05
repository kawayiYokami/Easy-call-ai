// ==================== 平台抽象层：窗口枚举 / 激活 / 控件树 ====================
//
// 三平台统一数据契约与函数签名，按 cfg 分发到 windows / linux / macos 实现模块。
// Windows 实现由原 windows_tool.rs + ui_automation.rs 平移而来（不重写）；
// Linux 为 xcap 枚举 + xcb 激活 + atspi 控件树；macOS 为 xcap 枚举 + AXUIElement 激活/控件树（手写 FFI，零新增依赖）。
// 未支持平台（FreeBSD 等）走兜底空实现，与既有行为一致。

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// 控件树扫描的元素数量上限（三平台一致，防极端桌面卡死扫描）
const MAX_ELEMENTS: usize = 500;

// ==================== 数据契约 ====================

/// 单个窗口信息（list windows 返回项）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub window_id: usize,
    pub title: String,
    pub process_id: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub minimized: bool,
    pub focused: bool,
}

/// 单个可交互元素（归一化坐标，基准为主屏，与 operate mouse @x,y 一致）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiElementInfo {
    pub window_id: u32,
    pub window_title: String,
    pub control_type: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// 该元素当前是否持有键盘焦点（激活窗口后用于确认焦点是否落在目标控件）
    pub focused: bool,
    /// 快照内元素引用编号（1 起全局唯一；仅 operate 截图响应赋值，供 app 动作 el= 引用）
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub element_ref: Option<u32>,
}

/// app 后台动作目标：按快照序号定位元素，或屏幕物理坐标定位
#[derive(Debug, Clone)]
pub enum AppTarget {
    /// el：模型侧元素引用编号（仅用于 stale 报错文案，指向模型能认识的 ref）
    Element { el: u32, ordinal: usize, control_type: String, name: String },
    Point { screen_x: i32, screen_y: i32 },
}

// ==================== app 后台动作（Windows 专用，其余平台明确报错） ====================

/// 后台点击：UIA InvokePattern 优先（dblclick=true 时跳过 Invoke 直接 PostMessage 双击序列），PostMessage 兜底。返回实际使用的投递方式。
#[cfg(target_os = "windows")]
pub fn app_click(window_id: usize, target: &AppTarget, repeat: u32, dblclick: bool) -> Result<&'static str, String> {
    windows::app_click(window_id, target, repeat, dblclick)
}

/// 后台写值：ValuePattern.SetValue 整体替换文本控件内容。
#[cfg(target_os = "windows")]
pub fn app_set_value(window_id: usize, target: &AppTarget, text: &str) -> Result<&'static str, String> {
    windows::app_set_value(window_id, target, text)
}

/// 后台读值：ValuePattern.CurrentValue 读回文本控件当前值。
#[cfg(target_os = "windows")]
pub fn app_get_value(window_id: usize, target: &AppTarget) -> Result<String, String> {
    windows::app_get_value(window_id, target)
}

/// 后台滚动：ScrollPattern 优先，WM_MOUSEWHEEL 兜底。返回实际使用的投递方式。
#[cfg(target_os = "windows")]
pub fn app_scroll(window_id: usize, target: &AppTarget, up: bool, small: bool, repeat: u32) -> Result<&'static str, String> {
    windows::app_scroll(window_id, target, up, small, repeat)
}

/// 后台按键：向目标窗口的内部焦点控件投递键盘消息。
#[cfg(target_os = "windows")]
pub fn app_key(window_id: usize, keys: &[String], repeat: u32) -> Result<&'static str, String> {
    windows::app_key(window_id, keys, repeat)
}

/// 查询目标窗口线程当前内部焦点控件（type, name）；无内部焦点或查询失败返回 None。
#[cfg(target_os = "windows")]
pub fn app_focus_summary(window_id: usize) -> Result<Option<(String, String)>, String> {
    windows::app_focus_summary(window_id)
}

#[cfg(not(target_os = "windows"))]
pub fn app_click(_window_id: usize, _target: &AppTarget, _repeat: u32, _dblclick: bool) -> Result<&'static str, String> {
    Err("app 后台操作当前仅在 Windows 平台可用".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn app_set_value(_window_id: usize, _target: &AppTarget, _text: &str) -> Result<&'static str, String> {
    Err("app 后台操作当前仅在 Windows 平台可用".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn app_get_value(_window_id: usize, _target: &AppTarget) -> Result<String, String> {
    Err("app 后台操作当前仅在 Windows 平台可用".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn app_scroll(_window_id: usize, _target: &AppTarget, _up: bool, _small: bool, _repeat: u32) -> Result<&'static str, String> {
    Err("app 后台操作当前仅在 Windows 平台可用".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn app_key(_window_id: usize, _keys: &[String], _repeat: u32) -> Result<&'static str, String> {
    Err("app 后台操作当前仅在 Windows 平台可用".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn app_focus_summary(_window_id: usize) -> Result<Option<(String, String)>, String> {
    Ok(None)
}

// ==================== 平台函数入口（按 cfg 分发） ====================

/// 全量枚举顶层可见窗口（含当前进程窗口）。返回按 z-order 排序的窗口列表。
#[cfg(target_os = "windows")]
pub fn list_all_windows() -> Vec<WindowInfo> {
    windows::list_all_windows()
}

/// 激活窗口到前台。返回 (窗口标题, 是否激活成功)。
#[cfg(target_os = "windows")]
pub fn activate_window(window_id: usize) -> (String, bool) {
    windows::activate_window(window_id)
}

/// 批量扫描指定窗口列表的可交互元素树（一次 COM/automation 实例复用）。
/// windows 为 (平台窗口 id, 窗口标题) 列表；返回扁平元素列表（归一化坐标）。
#[cfg(target_os = "windows")]
pub fn collect_ui_tree_for_windows(
    windows: &[(usize, String)],
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    windows::collect_ui_tree_for_windows(
        windows,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

/// 对指定窗口扫描可交互元素树；窗口 id 为 0 时忽略。
#[cfg(target_os = "windows")]
pub fn collect_window_ui_elements(
    window_id: usize,
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    windows::collect_window_ui_elements(
        window_id,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

#[cfg(target_os = "linux")]
pub fn list_all_windows() -> Vec<WindowInfo> {
    linux::list_all_windows()
}

#[cfg(target_os = "linux")]
pub fn activate_window(window_id: usize) -> (String, bool) {
    linux::activate_window(window_id)
}

#[cfg(target_os = "linux")]
pub fn collect_ui_tree_for_windows(
    windows: &[(usize, String)],
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    linux::collect_ui_tree_for_windows(
        windows,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

#[cfg(target_os = "linux")]
pub fn collect_window_ui_elements(
    window_id: usize,
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    linux::collect_window_ui_elements(
        window_id,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

#[cfg(target_os = "macos")]
pub fn list_all_windows() -> Vec<WindowInfo> {
    macos::list_all_windows()
}

#[cfg(target_os = "macos")]
pub fn activate_window(window_id: usize) -> (String, bool) {
    macos::activate_window(window_id)
}

#[cfg(target_os = "macos")]
pub fn collect_ui_tree_for_windows(
    windows: &[(usize, String)],
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    macos::collect_ui_tree_for_windows(
        windows,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

#[cfg(target_os = "macos")]
pub fn collect_window_ui_elements(
    window_id: usize,
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    macos::collect_window_ui_elements(
        window_id,
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

// ==================== 未支持平台兜底（FreeBSD 等） ====================

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn list_all_windows() -> Vec<WindowInfo> {
    Vec::new()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn activate_window(_window_id: usize) -> (String, bool) {
    (String::new(), false)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn collect_ui_tree_for_windows(
    _windows: &[(usize, String)],
    _primary_origin_x: f64,
    _primary_origin_y: f64,
    _primary_width: f64,
    _primary_height: f64,
) -> Vec<UiElementInfo> {
    Vec::new()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn collect_window_ui_elements(
    _window_id: usize,
    _primary_origin_x: f64,
    _primary_origin_y: f64,
    _primary_width: f64,
    _primary_height: f64,
) -> Vec<UiElementInfo> {
    Vec::new()
}
