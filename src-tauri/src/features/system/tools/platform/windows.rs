// ==================== Windows 平台实现（由 windows_tool.rs + ui_automation.rs 平移，不重写） ====================
//
// 仅 Windows 编译（platform/mod.rs 按 cfg 引入）。包含：
// - list windows：EnumWindows 全量枚举（绕 xcap 同进程过滤，含 PAI 自身窗口）
// - activate window：SW_RESTORE + Alt 键技巧 + SetForegroundWindow + 轮询验证
// - 控件树：手写 UIA COM vtable 绑定（不引入 windows crate，仅 windows-core + windows-sys）

use super::{MAX_ELEMENTS, UiElementInfo, WindowInfo};

// ==================== UI Automation 最小绑定 ====================
// 接口定义复制自 windows-0.61.3 生成代码（windows_core::imp::define_interface! 宏），
// 仅保留本项目用到的 4 个接口与 8 个方法；未用到的方法槽位用 usize 占位（宽度一致，偏移正确）。
// UIA_CONTROLTYPE_ID / UIA_*ControlTypeId 沿用 Windows SDK 官方命名，便于对照微软文档。

use windows_core::imp::define_interface;
use windows_core::imp::interface_hierarchy;
use windows_core::Interface;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct UIA_CONTROLTYPE_ID(pub i32);

// 可交互控件类型白名单（UI Automation ControlType 常量，沿用 SDK 官方命名）
#[allow(non_upper_case_globals)]
pub const UIA_ButtonControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50000);
#[allow(non_upper_case_globals)]
pub const UIA_CheckBoxControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50002);
#[allow(non_upper_case_globals)]
pub const UIA_ComboBoxControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50003);
#[allow(non_upper_case_globals)]
pub const UIA_EditControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50004);
#[allow(non_upper_case_globals)]
pub const UIA_HyperlinkControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50005);
#[allow(non_upper_case_globals)]
pub const UIA_ListItemControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50007);
#[allow(non_upper_case_globals)]
pub const UIA_MenuItemControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50011);
#[allow(non_upper_case_globals)]
pub const UIA_RadioButtonControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50013);
#[allow(non_upper_case_globals)]
pub const UIA_SliderControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50015);
#[allow(non_upper_case_globals)]
pub const UIA_TabItemControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50019);
#[allow(non_upper_case_globals)]
pub const UIA_TreeItemControlTypeId: UIA_CONTROLTYPE_ID = UIA_CONTROLTYPE_ID(50024);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TreeScope(pub i32);

#[allow(non_upper_case_globals)]
pub const TreeScope_Descendants: TreeScope = TreeScope(4);

/// 可交互控件类型白名单判定（纯函数，便于单测）
#[allow(non_upper_case_globals)]
pub fn is_interactive_control_type(ct: UIA_CONTROLTYPE_ID) -> bool {
    matches!(
        ct,
        UIA_ButtonControlTypeId
            | UIA_CheckBoxControlTypeId
            | UIA_ComboBoxControlTypeId
            | UIA_EditControlTypeId
            | UIA_HyperlinkControlTypeId
            | UIA_ListItemControlTypeId
            | UIA_MenuItemControlTypeId
            | UIA_RadioButtonControlTypeId
            | UIA_SliderControlTypeId
            | UIA_TabItemControlTypeId
            | UIA_TreeItemControlTypeId
    )
}

/// UIA 返回的控件类型数值转可读名称（用于模型理解）
#[allow(non_upper_case_globals)]
pub fn control_type_name(ct: UIA_CONTROLTYPE_ID) -> &'static str {
    match ct {
        UIA_ButtonControlTypeId => "Button",
        UIA_CheckBoxControlTypeId => "CheckBox",
        UIA_ComboBoxControlTypeId => "ComboBox",
        UIA_EditControlTypeId => "Edit",
        UIA_HyperlinkControlTypeId => "Hyperlink",
        UIA_ListItemControlTypeId => "ListItem",
        UIA_MenuItemControlTypeId => "MenuItem",
        UIA_RadioButtonControlTypeId => "RadioButton",
        UIA_SliderControlTypeId => "Slider",
        UIA_TabItemControlTypeId => "TabItem",
        UIA_TreeItemControlTypeId => "TreeItem",
        _ => "Unknown",
    }
}

// ==================== IUIAutomation ====================

define_interface!(IUIAutomation, IUIAutomation_Vtbl, 0x30cbe57d_d9d0_452a_ab13_7ac5ac4825ee);
interface_hierarchy!(IUIAutomation, windows_core::IUnknown);

#[allow(non_snake_case)]
#[repr(C)]
pub struct IUIAutomation_Vtbl {
    pub base__: windows_core::IUnknown_Vtbl,
    _compare_elements: usize,
    _compare_runtime_ids: usize,
    _get_root_element: usize,
    pub ElementFromHandle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    _element_from_point: usize,
    _get_focused_element: usize,
    _get_root_element_build_cache: usize,
    _element_from_handle_build_cache: usize,
    _element_from_point_build_cache: usize,
    _get_focused_element_build_cache: usize,
    _create_tree_walker: usize,
    _control_view_walker: usize,
    _content_view_walker: usize,
    _raw_view_walker: usize,
    _raw_view_condition: usize,
    _control_view_condition: usize,
    _content_view_condition: usize,
    _create_cache_request: usize,
    pub CreateTrueCondition: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

#[allow(non_snake_case)]
impl IUIAutomation {
    pub unsafe fn ElementFromHandle(&self, hwnd: *mut core::ffi::c_void) -> windows_core::Result<IUIAutomationElement> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).ElementFromHandle)(windows_core::Interface::as_raw(self), hwnd, &mut result__)
            .and_then(|| windows_core::Type::from_abi(result__))
    }

    pub unsafe fn CreateTrueCondition(&self) -> windows_core::Result<IUIAutomationCondition> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CreateTrueCondition)(windows_core::Interface::as_raw(self), &mut result__)
            .and_then(|| windows_core::Type::from_abi(result__))
    }
}

// ==================== IUIAutomationElement ====================

define_interface!(IUIAutomationElement, IUIAutomationElement_Vtbl, 0xd22108aa_8ac5_49a5_837b_37bbb3d7591e);
interface_hierarchy!(IUIAutomationElement, windows_core::IUnknown);

#[allow(non_snake_case)]
#[repr(C)]
pub struct IUIAutomationElement_Vtbl {
    pub base__: windows_core::IUnknown_Vtbl,
    _set_focus: usize,
    _get_runtime_id: usize,
    _find_first: usize,
    pub FindAll: unsafe extern "system" fn(*mut core::ffi::c_void, TreeScope, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    _find_first_build_cache: usize,
    _find_all_build_cache: usize,
    _build_updated_cache: usize,
    _get_current_property_value: usize,
    _get_current_property_value_ex: usize,
    _get_cached_property_value: usize,
    _get_cached_property_value_ex: usize,
    _get_current_pattern_as: usize,
    _get_cached_pattern_as: usize,
    _get_current_pattern: usize,
    _get_cached_pattern: usize,
    _get_cached_parent: usize,
    _get_cached_children: usize,
    _current_process_id: usize,
    pub CurrentControlType: unsafe extern "system" fn(*mut core::ffi::c_void, *mut UIA_CONTROLTYPE_ID) -> windows_core::HRESULT,
    _current_localized_control_type: usize,
    pub CurrentName: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    _current_accelerator_key: usize,
    _current_access_key: usize,
    pub CurrentHasKeyboardFocus: unsafe extern "system" fn(*mut core::ffi::c_void, *mut windows_core::BOOL) -> windows_core::HRESULT,
    _current_is_keyboard_focusable: usize,
    pub CurrentIsEnabled: unsafe extern "system" fn(*mut core::ffi::c_void, *mut windows_core::BOOL) -> windows_core::HRESULT,
    _current_automation_id: usize,
    _current_class_name: usize,
    _current_help_text: usize,
    _current_culture: usize,
    _current_is_control_element: usize,
    _current_is_content_element: usize,
    _current_is_password: usize,
    _current_native_window_handle: usize,
    _current_item_type: usize,
    _current_is_offscreen: usize,
    _current_orientation: usize,
    _current_framework_id: usize,
    _current_is_required_for_form: usize,
    _current_item_status: usize,
    pub CurrentBoundingRectangle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut windows_sys::Win32::Foundation::RECT) -> windows_core::HRESULT,
}

#[allow(non_snake_case)]
impl IUIAutomationElement {
    pub unsafe fn FindAll(&self, scope: TreeScope, condition: &IUIAutomationCondition) -> windows_core::Result<IUIAutomationElementArray> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).FindAll)(windows_core::Interface::as_raw(self), scope, condition.as_raw(), &mut result__)
            .and_then(|| windows_core::Type::from_abi(result__))
    }

    pub unsafe fn CurrentControlType(&self) -> windows_core::Result<UIA_CONTROLTYPE_ID> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CurrentControlType)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| result__)
    }

    pub unsafe fn CurrentName(&self) -> windows_core::Result<windows_core::BSTR> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CurrentName)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| core::mem::transmute(result__))
    }

    pub unsafe fn CurrentIsEnabled(&self) -> windows_core::Result<bool> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CurrentIsEnabled)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| result__.0 != 0)
    }

    pub unsafe fn CurrentHasKeyboardFocus(&self) -> windows_core::Result<bool> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CurrentHasKeyboardFocus)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| result__.0 != 0)
    }

    pub unsafe fn CurrentBoundingRectangle(&self) -> windows_core::Result<windows_sys::Win32::Foundation::RECT> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).CurrentBoundingRectangle)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| result__)
    }
}

// ==================== IUIAutomationElementArray ====================

define_interface!(IUIAutomationElementArray, IUIAutomationElementArray_Vtbl, 0x14314595_b4bc_4055_95f2_58f2e42c9855);
interface_hierarchy!(IUIAutomationElementArray, windows_core::IUnknown);

#[allow(non_snake_case)]
#[repr(C)]
pub struct IUIAutomationElementArray_Vtbl {
    pub base__: windows_core::IUnknown_Vtbl,
    pub Length: unsafe extern "system" fn(*mut core::ffi::c_void, *mut i32) -> windows_core::HRESULT,
    pub GetElement: unsafe extern "system" fn(*mut core::ffi::c_void, i32, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

#[allow(non_snake_case)]
impl IUIAutomationElementArray {
    pub unsafe fn Length(&self) -> windows_core::Result<i32> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).Length)(windows_core::Interface::as_raw(self), &mut result__)
            .map(|| result__)
    }

    pub unsafe fn GetElement(&self, index: i32) -> windows_core::Result<IUIAutomationElement> {
        let mut result__ = core::mem::zeroed();
        (windows_core::Interface::vtable(self).GetElement)(windows_core::Interface::as_raw(self), index, &mut result__)
            .and_then(|| windows_core::Type::from_abi(result__))
    }
}

// ==================== IUIAutomationCondition（空接口） ====================

define_interface!(IUIAutomationCondition, IUIAutomationCondition_Vtbl, 0x352ffba8_0973_437c_a61f_f64cafd81df9);
interface_hierarchy!(IUIAutomationCondition, windows_core::IUnknown);

#[allow(non_snake_case)]
#[repr(C)]
pub struct IUIAutomationCondition_Vtbl {
    pub base__: windows_core::IUnknown_Vtbl,
}

// ==================== 遍历入口 ====================

/// 批量扫描指定窗口列表的可交互元素树（一次 COM/automation 实例复用）。
/// windows 为 (hwnd, 窗口标题) 列表；返回扁平元素列表（归一化坐标，带 window 字段）。
/// 归一化基准为主屏：primary_origin_x/y 为主屏在虚拟屏幕中的原点，width/height 为主屏尺寸。
pub fn collect_ui_tree_for_windows(
    windows: &[(usize, String)],
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    if windows.is_empty() || primary_width <= 0.0 || primary_height <= 0.0 {
        return Vec::new();
    }
    let mut all = Vec::with_capacity(64);

    // COM 初始化：S_OK=本代码首次初始化（需配平 CoUninitialize）；S_FALSE=线程已初始化（直接复用，无需配平）；
    // RPC_E_CHANGED_MODE=线程已以其他模式初始化（COM 仍可用，无需配平）；其他失败则放弃扫描。
    let coinit = unsafe { windows_sys::Win32::System::Com::CoInitializeEx(core::ptr::null(), 0 /* COINIT_MULTITHREADED */) };
    let owns_com = coinit == 0; // 仅 S_OK 表示本代码首次初始化
    let com_ok = coinit >= 0 || coinit == windows_sys::Win32::Foundation::RPC_E_CHANGED_MODE;
    if !com_ok {
        return Vec::new();
    }

    let _co_guard = ComGuard::new(owns_com);

    // CoCreateInstance(CUIAutomation) → IUIAutomation
    let clsid = windows_sys::core::GUID::from_u128(0xff48dba4_60ef_4201_aa87_54103eef594e); // CLSID_CUIAutomation
    let iid = windows_sys::core::GUID::from_u128(0x30cbe57d_d9d0_452a_ab13_7ac5ac4825ee); // IID_IUIAutomation
    let mut automation_raw: *mut core::ffi::c_void = core::ptr::null_mut();
    let hr = unsafe {
        windows_sys::Win32::System::Com::CoCreateInstance(
            &clsid,
            core::ptr::null_mut(),
            windows_sys::Win32::System::Com::CLSCTX_INPROC_SERVER,
            &iid,
            &mut automation_raw,
        )
    };
    if hr >= 0 && !automation_raw.is_null() {
        let automation = unsafe { IUIAutomation::from_raw(automation_raw) };
        for (hwnd, title) in windows {
            if all.len() >= MAX_ELEMENTS {
                break;
            }
            let remaining = MAX_ELEMENTS - all.len();
            let mut scanned = scan_window(
                &automation,
                *hwnd,
                primary_origin_x,
                primary_origin_y,
                primary_width,
                primary_height,
                remaining,
            );
            for elem in scanned.iter_mut() {
                elem.window_id = *hwnd as u32;
                elem.window_title = title.clone();
            }
            all.append(&mut scanned);
        }
    }
    all
}

/// 对指定窗口句柄扫描可交互元素树。
/// hwnd 为 0 时忽略该窗口（失败跳过）；返回扁平元素列表（归一化坐标）。
pub fn collect_window_ui_elements(
    hwnd: usize,
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
) -> Vec<UiElementInfo> {
    collect_ui_tree_for_windows(
        &[(hwnd, String::new())],
        primary_origin_x,
        primary_origin_y,
        primary_width,
        primary_height,
    )
}

fn scan_window(
    automation: &IUIAutomation,
    hwnd: usize,
    primary_origin_x: f64,
    primary_origin_y: f64,
    primary_width: f64,
    primary_height: f64,
    max_elements: usize,
) -> Vec<UiElementInfo> {
    if hwnd == 0 || max_elements == 0 {
        return Vec::new();
    }
    // 最小化窗口没有真实屏幕位置，UIA 会返回垃圾坐标，直接跳过
    unsafe {
        if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd as _) != 0 {
            return Vec::new();
        }
    }
    let mut elements = Vec::with_capacity(max_elements.min(64));
    unsafe {
        let root = match automation.ElementFromHandle(hwnd as *mut core::ffi::c_void) {
            Ok(e) => e,
            Err(_) => return elements,
        };
        let condition = match automation.CreateTrueCondition() {
            Ok(c) => c,
            Err(_) => return elements,
        };
        let array = match root.FindAll(TreeScope_Descendants, &condition) {
            Ok(a) => a,
            Err(_) => return elements,
        };
        let len = array.Length().unwrap_or(0);
        for i in 0..len {
            if elements.len() >= max_elements {
                break;
            }
            let elem = match array.GetElement(i) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let ct = match elem.CurrentControlType() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if !is_interactive_control_type(ct) {
                continue;
            }
            let enabled = elem.CurrentIsEnabled().unwrap_or(false);
            if !enabled {
                continue;
            }
            let rect = match elem.CurrentBoundingRectangle() {
                Ok(r) => r,
                Err(_) => continue,
            };
            let w = (rect.right - rect.left) as f64;
            let h = (rect.bottom - rect.top) as f64;
            if w <= 0.0 || h <= 0.0 {
                continue;
            }
            let name = elem
                .CurrentName()
                .ok()
                .map(|b| b.to_string())
                .unwrap_or_default();
            elements.push(UiElementInfo {
                window_id: hwnd as u32,
                window_title: String::new(),
                control_type: control_type_name(ct).to_string(),
                name,
                x: (rect.left as f64 - primary_origin_x) / primary_width,
                y: (rect.top as f64 - primary_origin_y) / primary_height,
                width: w / primary_width,
                height: h / primary_height,
                focused: elem.CurrentHasKeyboardFocus().unwrap_or(false),
            });
        }
    }
    elements
}

/// COM 初始化守卫：仅当本代码首次初始化 COM（S_OK）时，Drop 时配平 CoUninitialize。
/// S_FALSE / RPC_E_CHANGED_MODE（线程已初始化）不执行清理，panic 时也会正确释放。
struct ComGuard {
    owns_com: bool,
}

impl ComGuard {
    fn new(owns_com: bool) -> Self {
        Self { owns_com }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owns_com {
            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

// ==================== 窗口枚举与激活 ====================

/// 全量枚举顶层可见窗口（含当前进程窗口；xcap 的 window_list 会过滤当前进程窗口，这里不用它）。
/// 返回按 z-order 排序（EnumWindows 顺序）的窗口信息列表。
pub fn list_all_windows() -> Vec<WindowInfo> {
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
        // 只枚举顶层可见窗口；跳过工具窗口（WS_EX_TOOLWINDOW），保留无标题窗口（真实存在）
        if unsafe { IsWindow(hwnd) != 0 } && unsafe { IsWindowVisible(hwnd) != 0 } {
            let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
            if ex_style & WS_EX_TOOLWINDOW != 0 {
                return 1;
            }
            let title = read_window_title(hwnd);
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
                window_id: hwnd as usize,
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

fn read_window_title(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

/// 激活窗口：还原最小化 → AttachThreadInput 绕前台锁 → SetForegroundWindow + BringWindowToTop → 轮询验证。
/// 返回 (窗口标题, 是否激活成功)。
///
/// 前台锁说明：SetForegroundWindow 默认只允许前台进程/收到最后输入事件的进程成功调用，
/// 后台进程直接调用会被系统静默拒绝。这里用 AttachThreadInput 把当前线程输入队列挂到
/// 目标窗口线程，使本进程获得设置前台窗口的资格（标准 API，无 UI 副作用）。
/// 不使用「模拟 Alt 键」的绕法：Alt 按下/释放会激活目标窗口菜单栏并显示 Key Tips，
/// 导致键盘焦点落在菜单栏而非正文控件，后续按键注入全部失效。
pub fn activate_window(window_id: usize) -> (String, bool) {
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
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
        // 取目标窗口线程并挂接输入队列，绕前台锁（挂接失败不阻断，继续尝试激活）
        let mut target_pid = 0u32;
        let target_tid = GetWindowThreadProcessId(hwnd, &mut target_pid);
        let current_tid = GetCurrentThreadId();
        let attached = target_tid != 0
            && target_tid != current_tid
            && AttachThreadInput(current_tid, target_tid, 1) != 0;
        let _ = SetForegroundWindow(hwnd);
        let _ = BringWindowToTop(hwnd);
        if attached {
            AttachThreadInput(current_tid, target_tid, 0);
        }
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

#[cfg(test)]
mod windows_platform_tests {
    use super::*;
    // tools.rs 被 include 进 main（crate root），window_list / primary_monitor_bounds /
    // runtime_log_info 都在 crate root；eprintln! 宏在 main.rs:29 被重定义为 runtime_log_info
    use crate::{primary_monitor_bounds, runtime_log_info, window_list};

    #[test]
    fn control_type_whitelist_should_match_interactive_types() {
        assert!(is_interactive_control_type(UIA_ButtonControlTypeId));
        assert!(is_interactive_control_type(UIA_EditControlTypeId));
        assert!(is_interactive_control_type(UIA_CheckBoxControlTypeId));
        assert!(is_interactive_control_type(UIA_ComboBoxControlTypeId));
        assert!(is_interactive_control_type(UIA_ListItemControlTypeId));
        assert!(is_interactive_control_type(UIA_TabItemControlTypeId));
        assert!(is_interactive_control_type(UIA_MenuItemControlTypeId));
        assert!(is_interactive_control_type(UIA_HyperlinkControlTypeId));
        assert!(is_interactive_control_type(UIA_TreeItemControlTypeId));
        assert!(is_interactive_control_type(UIA_SliderControlTypeId));
        assert!(is_interactive_control_type(UIA_RadioButtonControlTypeId));
    }

    #[test]
    fn non_interactive_types_should_be_rejected() {
        assert!(!is_interactive_control_type(UIA_CONTROLTYPE_ID(50001))); // unknown
        assert!(!is_interactive_control_type(UIA_CONTROLTYPE_ID(0)));
        assert!(!is_interactive_control_type(UIA_CONTROLTYPE_ID(50025)));
    }

    #[test]
    fn control_type_name_should_map_known_types() {
        assert_eq!(control_type_name(UIA_ButtonControlTypeId), "Button");
        assert_eq!(control_type_name(UIA_EditControlTypeId), "Edit");
        assert_eq!(control_type_name(UIA_CONTROLTYPE_ID(999)), "Unknown");
    }

    #[test]
    fn zero_inputs_should_return_empty() {
        assert!(collect_window_ui_elements(0, 0.0, 0.0, 1920.0, 1080.0).is_empty());
        assert!(collect_window_ui_elements(123, 0.0, 0.0, 0.0, 1080.0).is_empty());
        assert!(collect_window_ui_elements(123, 0.0, 0.0, 1920.0, 0.0).is_empty());
    }

    /// 真实桌面冒烟测试：枚举窗口 + 激活前台窗口验证。
    /// 依赖真实 Windows 桌面，CI/无头环境不可用，默认忽略，手动 `--ignored` 跑。
    #[test]
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

    /// 真实桌面冒烟测试：对本机主屏可见窗口扫一次控件树。
    /// 依赖真实 Windows 桌面，CI/无头环境不可用，默认忽略，手动 `--ignored` 跑。
    #[test]
    #[ignore = "需要真实 Windows 桌面"]
    fn real_desktop_should_scan_window_tree() {
        // window_list / primary_monitor_bounds 是 crate root（main include）私有函数
        let windows = crate::window_list().expect("list windows");
        let targets: Vec<(usize, String)> = windows
            .iter()
            .take(3)
            .map(|w| (w.id().unwrap_or(0) as usize, w.title().unwrap_or_default()))
            .collect();
        assert!(!targets.is_empty(), "桌面应有可见窗口");
        let bounds = crate::primary_monitor_bounds().expect("primary bounds");
        eprintln!("primary bounds: x={} y={} w={} h={}", bounds.x, bounds.y, bounds.width, bounds.height);
        let elems = collect_ui_tree_for_windows(
            &targets,
            bounds.x as f64,
            bounds.y as f64,
            bounds.width as f64,
            bounds.height as f64,
        );
        // 不强制非空（自绘窗口/游戏可能不暴露 UIA），但扫描本身不能 panic
        eprintln!("scanned {} elements from {} windows", elems.len(), targets.len());
        for e in elems.iter().take(5) {
            eprintln!("  {}({}) at {}x{}", e.control_type, e.name, e.x, e.y);
        }
    }
}
