// ==================== macOS 平台实现（暂未实现，占位） ====================
//
// 后续实现方案（已联网调研确认）：
// - list windows：复用 xcap window_list()（CGWindowListCopyWindowInfo，无同进程过滤）
// - activate window：NSRunningApplication（objc2-app-kit 已在依赖树）
// - 控件树：objc2-application-services（AXUIElement，与 xcap 同 objc2 生态）
// - 权限：辅助功能 + 屏幕录制（CGWindowList 拿标题需要）
// 当前返回空实现，与未支持平台兜底行为一致。

use super::{UiElementInfo, WindowInfo};

pub fn list_all_windows() -> Vec<WindowInfo> {
    Vec::new()
}

pub fn activate_window(_window_id: usize) -> (String, bool) {
    (String::new(), false)
}

pub fn collect_ui_tree_for_windows(
    _windows: &[(usize, String)],
    _primary_origin_x: f64,
    _primary_origin_y: f64,
    _primary_width: f64,
    _primary_height: f64,
) -> Vec<UiElementInfo> {
    Vec::new()
}

pub fn collect_window_ui_elements(
    _window_id: usize,
    _primary_origin_x: f64,
    _primary_origin_y: f64,
    _primary_width: f64,
    _primary_height: f64,
) -> Vec<UiElementInfo> {
    Vec::new()
}
