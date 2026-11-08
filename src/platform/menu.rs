//! Small owner-painted popup menus used by the custom menu bar.
//!
//! Windows' native `#32768` menu window always draws a system frame and a
//! white submenu-arrow gutter that cannot be recoloured from `WM_DRAWITEM`.
//! These popups are ordinary layered-looking popup windows painted entirely
//! with the application palette, so the frame, selection colour and submenu
//! arrows are fully under our control.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{draw_line, draw_text, fill_rect, text_width, wide};
use crate::ui::{dpi::dp_to_px, metrics};

const CLASS_NAME: &str = "Calc1MenuPopup";
const WM_MENU_SELECT: u32 = WM_APP + 17;
const WM_MENU_CANCEL: u32 = WM_APP + 18;

/// One row in a popup menu.
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub accel: String,
    pub id: u16,
    pub checked: bool,
    pub enabled: bool,
    pub separator: bool,
    pub submenu: Vec<MenuItem>,
}

impl MenuItem {
    pub fn item(label: &str, accel: &str, id: u16, checked: bool, enabled: bool) -> Self {
        Self {
            label: label.to_string(),
            accel: accel.to_string(),
            id,
            checked,
            enabled,
            separator: false,
            submenu: Vec::new(),
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            accel: String::new(),
            id: 0,
            checked: false,
            enabled: false,
            separator: true,
            submenu: Vec::new(),
        }
    }

    pub fn submenu(label: &str, submenu: Vec<MenuItem>) -> Self {
        Self {
            label: label.to_string(),
            accel: String::new(),
            id: 0,
            checked: false,
            enabled: true,
            separator: false,
            submenu,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MenuStyle {
    pub panel: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub selection: u32,
    pub selection_text: u32,
    pub disabled: u32,
    pub body: HFONT,
    pub small: HFONT,
    pub scale: f32,
}

struct MenuState {
    owner: HWND,
    root: HWND,
    parent: HWND,
    child: HWND,
    items: Vec<MenuItem>,
    style: MenuStyle,
    selected: Option<usize>,
    child_index: Option<usize>,
    width: i32,
    height: i32,
    row_h: i32,
    sep_h: i32,
    check_w: i32,
    pad: i32,
    arrow_w: i32,
}

static CLASS_READY: AtomicBool = AtomicBool::new(false);

/// When a menu is dismissed by clicking back into the owner window, the
/// click must not also reach the menu bar or a key. Native menus consume
/// that first click; store its tick so the app can do the same.
static DISMISS_CLICK_AT: AtomicU32 = AtomicU32::new(0);
static DISMISS_PENDING: AtomicBool = AtomicBool::new(false);
static ACTIVE_ROOT: AtomicIsize = AtomicIsize::new(0);
static PREVIOUS_FOCUS: AtomicIsize = AtomicIsize::new(0);

fn mark_dismiss_click() {
    DISMISS_CLICK_AT.store(unsafe { GetMessageTime() } as u32, Ordering::Relaxed);
    DISMISS_PENDING.store(true, Ordering::Relaxed);
}

/// Returns true for the click that dismissed a popup, then clears the mark.
pub fn take_dismiss_click() -> bool {
    DISMISS_PENDING.swap(false, Ordering::Relaxed)
        && DISMISS_CLICK_AT.load(Ordering::Relaxed) == unsafe { GetMessageTime() } as u32
}

/// The root's lifetime is recorded without calling back into the owner handler.
pub fn root_is_open() -> bool {
    ACTIVE_ROOT.load(Ordering::Relaxed) != 0
}

pub fn install_class() {
    if CLASS_READY.swap(true, Ordering::Relaxed) {
        return;
    }
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide(CLASS_NAME);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(menu_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        if RegisterClassExW(&wc) == 0 {
            CLASS_READY.store(false, Ordering::Relaxed);
        }
    }
}

/// Shows a popup menu anchored to a client coordinate of `owner`.
pub fn show_menu(owner: HWND, items: Vec<MenuItem>, x: i32, y: i32, style: MenuStyle) -> HWND {
    if items.is_empty() {
        return std::ptr::null_mut();
    }
    install_class();
    let focus = unsafe { GetFocus() };
    if unsafe { IsChild(owner, focus) } != 0
        || (focus == owner && PREVIOUS_FOCUS.load(Ordering::Relaxed) == 0)
    {
        PREVIOUS_FOCUS.store(focus as isize, Ordering::Relaxed);
    }
    let mut point = POINT { x, y };
    unsafe {
        ClientToScreen(owner, &mut point);
    }
    create_popup(
        owner,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        items,
        style,
        point.x,
        point.y,
        true,
    )
}

/// Destroys any popup menu owned by the current UI thread.
pub fn close_all() {
    unsafe {
        let thread = GetCurrentThreadId();
        let _ = EnumThreadWindows(thread, Some(close_proc), 0);
    }
}

unsafe extern "system" fn close_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut class = [0u16; 32];
    let len = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
    if len > 0 {
        let name = String::from_utf16_lossy(&class[..len as usize]);
        if name == CLASS_NAME {
            unsafe {
                DestroyWindow(hwnd);
            }
        }
    }
    1
}

/// True when `hwnd` is one of the windows in this popup stack.
///
/// `WM_ACTIVATE` gives the window that is gaining activation in `lParam`.
/// Comparing against that handle is reliable; querying `GetForegroundWindow`
/// inside the handler is not, because it can still return the popup that is
/// being deactivated. That made every outside click look like activation of
/// "some popup" and kept the menu open until an item was chosen.
fn activation_stays_in_menu(state: &MenuState, hwnd: HWND) -> bool {
    !hwnd.is_null() && (hwnd == state.root || hwnd == state.parent || hwnd == state.child)
}

unsafe fn state_mut(hwnd: HWND) -> Option<&'static mut MenuState> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState;
        ptr.as_mut()
    }
}

fn popup_width(owner: HWND, items: &[MenuItem], style: MenuStyle) -> (i32, i32, i32, i32, i32) {
    let s = |dp| dp_to_px(dp, style.scale);
    let check_w = s(metrics::POPUP_CHECK_W);
    let pad = s(metrics::MENU_TEXT_PAD_X);
    let arrow_w = s(metrics::POPUP_ARROW_W);
    let mut width = s(metrics::POPUP_MIN_W);
    for item in items {
        if item.separator {
            continue;
        }
        let text = text_width(owner, style.body, &item.label);
        let accel = text_width(owner, style.small, &item.accel);
        let arrow = if item.submenu.is_empty() { 0 } else { arrow_w };
        width = width.max(check_w + text + pad * 2 + accel + arrow);
    }
    (width, check_w, pad, arrow_w, 0)
}

fn popup_placement(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work: RECT,
    pad_px: i32,
    parent: Option<RECT>,
) -> RECT {
    let usable_w = (work.right - work.left - pad_px * 2).max(1);
    let usable_h = (work.bottom - work.top - pad_px * 2).max(1);
    let width = width.min(usable_w).max(1);
    let height = height.min(usable_h).max(1);
    let min_x = work.left + pad_px;
    let min_y = work.top + pad_px;
    let max_x = (work.right - pad_px - width).max(min_x);
    let max_y = (work.bottom - pad_px - height).max(min_y);
    let preferred_x = match parent {
        Some(parent) if x + width > work.right - pad_px => parent.left - width,
        _ => x,
    };
    let left = preferred_x.clamp(min_x, max_x);
    let top = y.clamp(min_y, max_y);
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn create_popup(
    owner: HWND,
    parent: HWND,
    root: HWND,
    items: Vec<MenuItem>,
    style: MenuStyle,
    x: i32,
    y: i32,
    show: bool,
) -> HWND {
    let s = |dp| dp_to_px(dp, style.scale);
    let row_h = s(metrics::POPUP_ROW_H).max(1);
    let sep_h = s(metrics::POPUP_SEPARATOR_H).max(1);
    let (mut width, check_w, pad, arrow_w, _) = popup_width(owner, &items, style);
    let mut height = 0;
    for item in &items {
        height += if item.separator { sep_h } else { row_h };
    }
    // Work areas and anchors are already physical desktop coordinates.
    let monitor = unsafe {
        if parent.is_null() {
            MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST)
        } else {
            MonitorFromWindow(parent, MONITOR_DEFAULTTONEAREST)
        }
    };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let mut work = RECT {
        left: 0,
        top: 0,
        right: 1,
        bottom: 1,
    };
    unsafe {
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            work = info.rcWork;
        } else if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0) == 0 {
            work.left = GetSystemMetrics(SM_XVIRTUALSCREEN);
            work.top = GetSystemMetrics(SM_YVIRTUALSCREEN);
            work.right = work.left + GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
            work.bottom = work.top + GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);
        }
    }
    let parent_rect = if parent.is_null() {
        None
    } else {
        let mut rect = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(parent, &mut rect);
        }
        Some(rect)
    };
    let placed = popup_placement(
        x,
        y,
        width,
        height,
        work,
        s(metrics::POPUP_EDGE_PAD),
        parent_rect,
    );
    let left = placed.left;
    let top = placed.top;
    width = placed.right - placed.left;
    height = placed.bottom - placed.top;

    let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class = wide(CLASS_NAME);
    // A submenu must never take activation: doing so deactivates the root
    // popup and any later activation fallback closes the whole menu. Keeping
    // the child non-activating also stops hover movement from bouncing
    // activation between the parent and the child.
    let ex_style = if parent.is_null() {
        WS_EX_TOOLWINDOW
    } else {
        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
    };
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            left,
            top,
            width,
            height,
            if parent.is_null() { owner } else { parent },
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return std::ptr::null_mut();
    }
    let mut state = Box::new(MenuState {
        owner,
        root,
        parent,
        child: std::ptr::null_mut(),
        items,
        style,
        selected: None,
        child_index: None,
        width,
        height,
        row_h,
        sep_h,
        check_w,
        pad,
        arrow_w,
    });
    if state.root.is_null() {
        state.root = hwnd;
    }
    let ptr = Box::into_raw(state);
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
        if parent.is_null() {
            ACTIVE_ROOT.store(hwnd as isize, Ordering::Relaxed);
        }
        if show {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }
    hwnd
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    #[test]
    fn popups_stay_on_positive_and_negative_monitors_and_flip_submenus() {
        for left in [-2560, 1920] {
            let work = RECT {
                left,
                top: -200,
                right: left + 2560,
                bottom: 1200,
            };
            for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
                let scale = crate::ui::DpiScale::new(dpi);
                let pad = scale.px(metrics::POPUP_EDGE_PAD);
                let root = popup_placement(
                    left + 100,
                    0,
                    scale.px(200.0),
                    scale.px(300.0),
                    work,
                    pad,
                    None,
                );
                assert_eq!(root.left, left + 100);
                assert!(root.right <= work.right - pad);
                let parent = RECT {
                    left: work.right - 300,
                    top: 0,
                    right: work.right - pad,
                    bottom: 400,
                };
                let child = popup_placement(
                    parent.right,
                    parent.top,
                    scale.px(150.0),
                    scale.px(200.0),
                    work,
                    pad,
                    Some(parent),
                );
                assert_eq!(child.right, parent.left);
                assert!(child.left >= work.left + pad && child.bottom <= work.bottom - pad);
                let clamped = popup_placement(left - 100, -1000, 100, 100, work, pad, None);
                assert_eq!(
                    (clamped.left, clamped.top),
                    (work.left + pad, work.top + pad)
                );
            }
        }
    }
}

fn close_root(hwnd: HWND, state: &MenuState) {
    let root = if state.root.is_null() {
        hwnd
    } else {
        state.root
    };
    unsafe {
        PostMessageW(root, WM_CLOSE, 0, 0);
    }
}

fn select(hwnd: HWND, state: &MenuState, index: usize) {
    let Some(item) = state.items.get(index) else {
        return;
    };
    if item.separator || !item.enabled || !item.submenu.is_empty() {
        return;
    }
    let root = if state.root.is_null() {
        hwnd
    } else {
        state.root
    };
    let id = item.id;
    unsafe {
        PostMessageW(root, WM_MENU_SELECT, id as WPARAM, 0);
    }
}

impl MenuState {
    fn row_height(&self, index: usize) -> i32 {
        match self.items.get(index) {
            Some(item) if item.separator => self.sep_h,
            _ => self.row_h,
        }
    }

    fn row_top(&self, index: usize) -> i32 {
        let mut top = 0;
        for i in 0..index {
            top += self.row_height(i);
        }
        top
    }

    fn row_rect(&self, index: usize) -> RECT {
        let top = self.row_top(index);
        RECT {
            left: 0,
            top,
            right: self.width,
            bottom: top + self.row_height(index),
        }
    }

    fn item_at(&self, y: i32) -> Option<usize> {
        let mut top = 0;
        for (index, item) in self.items.iter().enumerate() {
            let h = if item.separator {
                self.sep_h
            } else {
                self.row_h
            };
            if y >= top && y < top + h {
                return if item.separator || !item.enabled {
                    None
                } else {
                    Some(index)
                };
            }
            top += h;
        }
        None
    }

    fn open_child(&mut self, hwnd: HWND, index: usize) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        if item.submenu.is_empty() || !item.enabled {
            return;
        }
        let style = self.style;
        let owner = self.owner;
        let root = if self.root.is_null() { hwnd } else { self.root };
        let submenu = item.submenu.clone();
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }
        let x = rect.right;
        let y = rect.top + self.row_top(index);
        self.close_child();
        let child = create_popup(owner, hwnd, root, submenu, style, x, y, false);
        self.child = child;
        self.child_index = Some(index);
        if !child.is_null() {
            unsafe {
                // The child is `WS_EX_NOACTIVATE`, so the root keeps focus
                // and activation while the submenu is hovered.
                ShowWindow(child, SW_SHOWNA);
                InvalidateRect(child, std::ptr::null(), 0);
            }
        }
    }

    fn close_child(&mut self) {
        if !self.child.is_null() {
            let child = std::mem::replace(&mut self.child, std::ptr::null_mut());
            unsafe {
                // Prevent the child's WM_DESTROY from borrowing this parent
                // while its caller still holds &mut self.
                if let Some(child_state) = state_mut(child) {
                    child_state.parent = std::ptr::null_mut();
                }
                DestroyWindow(child);
            }
        }
        self.child_index = None;
    }

    fn move_selection(&mut self, hwnd: HWND, down: bool) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let mut current = self.selected;
        for _ in 0..count {
            let next = match current {
                Some(index) => {
                    if down {
                        (index + 1) % count
                    } else {
                        (index + count - 1) % count
                    }
                }
                None => {
                    if down {
                        0
                    } else {
                        count - 1
                    }
                }
            };
            current = Some(next);
            if let Some(item) = self.items.get(next) {
                if !item.separator && item.enabled {
                    self.selected = Some(next);
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    return;
                }
            }
        }
    }

    fn on_mouse_move(&mut self, hwnd: HWND, y: i32) {
        let index = self.item_at(y);
        if index != self.selected {
            self.selected = index;
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        if let Some(index) = index {
            let has_submenu = self
                .items
                .get(index)
                .map(|item| !item.submenu.is_empty())
                .unwrap_or(false);
            if has_submenu {
                if self.child_index != Some(index) || self.child.is_null() {
                    self.open_child(hwnd, index);
                }
            } else if !self.child.is_null() {
                self.close_child();
            }
        }
    }

    fn on_key(&mut self, hwnd: HWND, vk: u16) {
        match vk {
            VK_DOWN => self.move_selection(hwnd, true),
            VK_UP => self.move_selection(hwnd, false),
            VK_HOME => {
                self.selected = None;
                self.move_selection(hwnd, true);
            }
            VK_END => {
                self.selected = None;
                self.move_selection(hwnd, false);
            }
            VK_RIGHT => {
                if let Some(index) = self.selected.filter(|&i| !self.items[i].submenu.is_empty()) {
                    self.open_child(hwnd, index);
                } else if self.parent.is_null() {
                    unsafe {
                        PostMessageW(hwnd, WM_MENU_SELECT, super::CMD_MENU_NEXT as usize, 0);
                    }
                }
            }
            VK_LEFT | VK_ESCAPE => {
                if !self.parent.is_null() {
                    unsafe {
                        PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    }
                } else if vk == VK_ESCAPE {
                    unsafe {
                        PostMessageW(hwnd, WM_MENU_CANCEL, 0, 0);
                    }
                } else {
                    unsafe {
                        PostMessageW(hwnd, WM_MENU_SELECT, super::CMD_MENU_PREVIOUS as usize, 0);
                    }
                }
            }
            VK_RETURN => {
                if let Some(index) = self.selected {
                    if !self.items[index].submenu.is_empty() {
                        self.open_child(hwnd, index);
                    } else {
                        select(hwnd, self, index);
                    }
                }
            }
            _ => {}
        }
    }

    fn paint(&self, hdc: HDC) {
        let client = RECT {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        };
        fill_rect(hdc, client, self.style.panel);
        for (index, item) in self.items.iter().enumerate() {
            let rect = self.row_rect(index);
            if item.separator {
                let y = (rect.top + rect.bottom) / 2;
                draw_line(
                    hdc,
                    self.pad,
                    y,
                    self.width - self.pad,
                    y,
                    self.style.border,
                );
                continue;
            }
            let selected = self.selected == Some(index) && item.enabled;
            if selected {
                fill_rect(hdc, rect, self.style.selection);
            }
            let color = if !item.enabled {
                self.style.disabled
            } else if selected {
                self.style.selection_text
            } else {
                self.style.text
            };
            if item.checked {
                let cx = dp_to_px(metrics::PAD_PANEL, self.style.scale);
                let cy = (rect.top + rect.bottom) / 2;
                let step = dp_to_px(metrics::MENU_EDGE_PAD, self.style.scale);
                draw_line(hdc, cx, cy, cx + step, cy + step, color);
                draw_line(hdc, cx + step, cy + step, cx + step * 3, cy - step, color);
            }
            let arrow = if item.submenu.is_empty() {
                0
            } else {
                self.arrow_w
            };
            let accel = text_width(self.owner, self.style.small, &item.accel);
            let mut text_rect = RECT {
                left: self.check_w,
                top: rect.top,
                right: self.width - self.pad - arrow - if accel > 0 { accel + self.pad } else { 0 },
                bottom: rect.bottom,
            };
            draw_text(
                hdc,
                &mut text_rect,
                &item.label,
                self.style.body,
                color,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            if accel > 0 {
                let mut accel_rect = RECT {
                    left: self.width - self.pad - arrow - accel,
                    top: rect.top,
                    right: self.width - self.pad - arrow,
                    bottom: rect.bottom,
                };
                draw_text(
                    hdc,
                    &mut accel_rect,
                    &item.accel,
                    self.style.small,
                    if selected {
                        self.style.selection_text
                    } else {
                        self.style.muted
                    },
                    DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
            if !item.submenu.is_empty() {
                let cx = self.width - dp_to_px(metrics::GAP_BLOCK, self.style.scale);
                let cy = (rect.top + rect.bottom) / 2;
                let step = dp_to_px(metrics::MENU_EDGE_PAD, self.style.scale);
                draw_line(hdc, cx - step, cy - step, cx, cy, color);
                draw_line(hdc, cx, cy, cx - step, cy + step, color);
            }
        }
    }
}

unsafe extern "system" fn menu_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Destroy outside any live &mut MenuState. The earlier callback only
    // queued this message, so nested destruction cannot free its own borrow.
    if msg == WM_CLOSE || msg == WM_MENU_SELECT || msg == WM_MENU_CANCEL {
        let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const MenuState };
        if !ptr.is_null() {
            let owner = unsafe { (*ptr).owner };
            let parent = unsafe { (*ptr).parent };
            unsafe {
                DestroyWindow(hwnd);
                if msg == WM_MENU_SELECT {
                    SetForegroundWindow(owner);
                    PostMessageW(owner, WM_COMMAND, wparam, 0);
                } else if msg == WM_MENU_CANCEL {
                    let focus = PREVIOUS_FOCUS.swap(0, Ordering::Relaxed) as HWND;
                    SetForegroundWindow(owner);
                    if !focus.is_null()
                        && IsWindow(focus) != 0
                        && (focus == owner || IsChild(owner, focus) != 0)
                    {
                        SetFocus(focus);
                    }
                } else if !parent.is_null() {
                    SetForegroundWindow(parent);
                    SetFocus(parent);
                }
            }
        }
        return 0;
    }
    if msg == WM_NCDESTROY {
        let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState };
        if !ptr.is_null() {
            unsafe {
                if (*ptr).root == hwnd {
                    let _ = ACTIVE_ROOT.compare_exchange(
                        hwnd as isize,
                        0,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
                    InvalidateRect((*ptr).owner, std::ptr::null(), 0);
                }
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let Some(state) = (unsafe { state_mut(hwnd) }) else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };

    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            // Hovering changes the selected row on every mouse move. Painting
            // the whole popup through a memory DC keeps that repaint from
            // flashing the frame or the item labels.
            let mut client: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(hwnd, &mut client);
            }
            let width = client.right.max(1);
            let height = client.bottom.max(1);
            let mem = unsafe { CreateCompatibleDC(hdc) };
            let bmp = unsafe { CreateCompatibleBitmap(hdc, width, height) };
            if !mem.is_null() && !bmp.is_null() {
                let old = unsafe { SelectObject(mem, bmp as HGDIOBJ) };
                state.paint(mem);
                unsafe {
                    BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
                    SelectObject(mem, old);
                    DeleteObject(bmp as HGDIOBJ);
                    DeleteDC(mem);
                }
            } else {
                state.paint(hdc);
                if !mem.is_null() {
                    unsafe {
                        DeleteDC(mem);
                    }
                }
                if !bmp.is_null() {
                    unsafe {
                        DeleteObject(bmp as HGDIOBJ);
                    }
                }
            }
            unsafe {
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_MOUSEMOVE => {
            let y = ((lparam >> 16) & 0xffff) as i16 as i32;
            state.on_mouse_move(hwnd, y);
            0
        }
        WM_LBUTTONDOWN => {
            let y = ((lparam >> 16) & 0xffff) as i16 as i32;
            if let Some(index) = state.item_at(y) {
                state.selected = Some(index);
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            let y = ((lparam >> 16) & 0xffff) as i16 as i32;
            if let Some(index) = state.item_at(y) {
                if state.items[index].submenu.is_empty() {
                    select(hwnd, state, index);
                } else {
                    state.open_child(hwnd, index);
                }
            }
            0
        }
        WM_RBUTTONUP | WM_MBUTTONUP => {
            close_root(hwnd, state);
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as u16;
            // Focus stays on the root popup because submenus are
            // non-activating; forward navigation keys to the open submenu.
            if !state.child.is_null()
                && matches!(
                    vk,
                    VK_UP | VK_DOWN | VK_HOME | VK_END | VK_RETURN | VK_LEFT | VK_RIGHT | VK_ESCAPE
                )
            {
                let child = state.child;
                unsafe {
                    SendMessageW(child, WM_KEYDOWN, wparam, lparam);
                }
                return 0;
            }
            state.on_key(hwnd, vk);
            0
        }
        WM_ACTIVATE => {
            let code = (wparam & 0xffff) as u32;
            if code == WA_INACTIVE {
                // `lParam` is the window that is gaining activation. Keep
                // the stack alive only when focus moves inside it (for
                // example to a submenu). Any other target — the owner, the
                // desktop, another app — is an outside click and must close
                // the whole menu.
                let next = lparam as HWND;
                if !activation_stays_in_menu(state, next) {
                    if next == state.owner {
                        mark_dismiss_click();
                    }
                    close_root(hwnd, state);
                }
            }
            0
        }
        WM_ACTIVATEAPP => {
            if wparam == 0 {
                close_root(hwnd, state);
            }
            0
        }
        WM_MOUSEACTIVATE => {
            if state.parent.is_null() {
                MA_ACTIVATE as LRESULT
            } else {
                MA_NOACTIVATE as LRESULT
            }
        }
        WM_DPICHANGED => {
            // Owner owns the fonts; never retain a popup at a stale scale.
            close_root(hwnd, state);
            0
        }
        WM_NCHITTEST => HTCLIENT as LRESULT,
        WM_DESTROY => {
            state.close_child();
            if !state.parent.is_null() {
                if let Some(parent) = unsafe { state_mut(state.parent) } {
                    if parent.child == hwnd {
                        parent.child = std::ptr::null_mut();
                    }
                }
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
