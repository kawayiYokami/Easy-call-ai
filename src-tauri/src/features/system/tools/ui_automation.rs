// ==================== 控件树数据契约（平台无关，re-export） ====================
//
// UiElementInfo 定义与 collect 入口已迁移到 platform/ 模块：
// - windows：手写 UIA COM vtable 绑定（见 platform/windows.rs）
// - linux：atspi（AT-SPI2 over D-Bus，见 platform/linux.rs）
// - macos：占位（见 platform/macos.rs）
// 此处仅 re-export，保持 operate_actions 等调用方的引用不变（tools 模块内直接使用）。

pub use crate::platform::{collect_ui_tree_for_windows, collect_window_ui_elements, UiElementInfo};
