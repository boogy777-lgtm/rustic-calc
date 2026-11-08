mod controls;
mod menu;
pub use controls::{install_combo_paint, install_edit_paint, set_control_palette};
pub mod png;
pub use menu::{close_all as close_menus, install_class as install_menu_class};
pub use menu::{root_is_open, show_menu, take_dismiss_click, MenuItem, MenuStyle};
pub const CMD_MENU_PREVIOUS: u16 = 7900;
pub const CMD_MENU_NEXT: u16 = 7901;

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::System::DataExchange::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::{
    GetComboBoxInfo, SetWindowTheme, COMBOBOXINFO, DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODT_COMBOBOX,
};
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const MODE_STANDARD: u16 = 1001;
pub const MODE_SCIENTIFIC: u16 = 1002;
pub const MODE_PROGRAMMER: u16 = 1003;
pub const MODE_STATISTICS: u16 = 1004;
pub const MODE_UNIT: u16 = 1005;
pub const MODE_DATE: u16 = 1006;
pub const MODE_MORTGAGE: u16 = 1010;
pub const MODE_LEASE: u16 = 1011;
pub const MODE_MPG: u16 = 1012;
pub const MODE_L100: u16 = 1013;
pub const CMD_HISTORY: u16 = 1020;
pub const CMD_GROUPING: u16 = 1021;
pub const CMD_COPY: u16 = 1030;
pub const CMD_PASTE: u16 = 1031;
pub const CMD_HELP: u16 = 1040;
pub const CMD_ABOUT: u16 = 1041;
pub const CMD_EXIT: u16 = 1042;
pub const CMD_LANG_RU: u16 = 1110;
pub const CMD_LANG_EN: u16 = 1111;
pub const CMD_THEME_SYSTEM: u16 = 1120;
pub const CMD_THEME_WHITE: u16 = 1121;
pub const CMD_THEME_DARK: u16 = 1122;

pub trait WindowHandler {
    fn on_created(&mut self, hwnd: HWND, dpi: u32);
    fn on_paint(&mut self, hdc: HDC);
    fn on_resize(&mut self, width: i32, height: i32);
    fn on_mouse_move(&mut self, x: i32, y: i32);
    fn on_lbutton_down(&mut self, x: i32, y: i32);
    fn on_lbutton_up(&mut self, x: i32, y: i32);
    fn on_key(&mut self, vk: u16, ctrl: bool) -> bool;
    fn on_char(&mut self, ch: char) -> bool;
    fn on_command(&mut self, id: u16, code: u16) -> bool;
    fn on_timer(&mut self, id: usize);
    fn on_focus(&mut self);
    fn on_wheel(&mut self, delta: i16);
    fn on_dpi(&mut self, dpi: u32);
    fn on_setting_change(&mut self);
    /// Minimum client size for the current mode, in 96-DPI logical pixels.
    fn min_client_size(&self) -> (i32, i32) {
        (0, 0)
    }
    /// Background and text colors for native EDIT/COMBOBOX children.
    fn control_colors(&self) -> (u32, u32);
    fn control_brush(&self) -> HBRUSH;
    fn measure_combo_item(&mut self, _mis: &mut MEASUREITEMSTRUCT) -> bool {
        false
    }
    fn draw_combo_item(&mut self, _dis: &mut DRAWITEMSTRUCT) -> bool {
        false
    }
    fn on_destroy(&mut self);
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn local_date() -> (i32, u32, u32) {
    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe {
        GetLocalTime(&mut time);
    }
    (time.wYear as i32, time.wMonth as u32, time.wDay as u32)
}

pub fn scale(value: i32, dpi: u32) -> i32 {
    crate::ui::DpiScale::new(dpi).px(value as f32)
}

pub fn solid_brush(color: u32) -> HBRUSH {
    unsafe { CreateSolidBrush(color) }
}

pub fn text_width(hwnd: HWND, font: HFONT, text: &str) -> i32 {
    if text.is_empty() || font.is_null() {
        return 0;
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let hdc = unsafe { GetDC(hwnd) };
    if hdc.is_null() {
        return 0;
    }
    let mut size: SIZE = unsafe { std::mem::zeroed() };
    unsafe {
        let old = SelectObject(hdc, font);
        GetTextExtentPoint32W(hdc, wide.as_ptr(), wide.len() as i32, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(hwnd, hdc);
    }
    size.cx
}

pub fn fill_rect(hdc: HDC, rect: RECT, color: u32) {
    let brush = unsafe { CreateSolidBrush(color) };
    if !brush.is_null() {
        unsafe {
            FillRect(hdc, &rect, brush);
            DeleteObject(brush as HGDIOBJ);
        }
    }
}

pub fn draw_text(hdc: HDC, rect: &mut RECT, text: &str, font: HFONT, color: u32, flags: u32) {
    if text.is_empty() {
        return;
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, color);
        DrawTextW(hdc, wide.as_ptr(), wide.len() as i32, rect, flags);
        SelectObject(hdc, old);
    }
}

pub fn draw_line(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
    let pen = unsafe { CreatePen(PS_SOLID, 1, color) };
    if pen.is_null() {
        return;
    }
    unsafe {
        let old = SelectObject(hdc, pen);
        MoveToEx(hdc, x1, y1, std::ptr::null_mut());
        LineTo(hdc, x2, y2);
        SelectObject(hdc, old);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub fn delete_brush(brush: HBRUSH) {
    if !brush.is_null() {
        unsafe {
            DeleteObject(brush as HGDIOBJ);
        }
    }
}

/// Windows "app mode" light/dark preference.
pub fn system_prefers_dark() -> bool {
    let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut c_void,
            &mut size,
        )
    };
    status == 0 && data == 0
}

/// Applies the immersive dark title bar when available.
pub fn set_dark_titlebar(hwnd: HWND, dark: bool) {
    let value: BOOL = if dark { 1 } else { 0 };
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &value as *const BOOL as *const c_void,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
}

static EDIT_SEL_BG: AtomicU32 = AtomicU32::new(0x0063_4d3e);
static EDIT_SEL_TEXT: AtomicU32 = AtomicU32::new(0x00fa_f3ee);

/// Palette used to repaint the system selection of native EDIT controls.
pub fn set_edit_colors(selection_bg: u32, selection_text: u32) {
    EDIT_SEL_BG.store(selection_bg, Ordering::Relaxed);
    EDIT_SEL_TEXT.store(selection_text, Ordering::Relaxed);
}

/// Applies the native dark/light theme to one child control.
pub fn theme_control(hwnd: HWND, dark: bool) {
    if hwnd.is_null() {
        return;
    }
    let mut class = [0u16; 32];
    let len =
        unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) }.max(0) as usize;
    let class = String::from_utf16_lossy(&class[..len]);
    let list = class.eq_ignore_ascii_case("ComboLBox");
    unsafe {
        SetPropW(
            hwnd,
            windows_sys::w!("Calc1Dark"),
            (if dark { 1 } else { 0 }) as usize as HANDLE,
        );
    }
    let theme = if dark {
        wide("DarkMode_CFD")
    } else {
        Vec::new()
    };
    let ptr = if list {
        // Native theme animations must not repaint a light scrollbar over ours.
        windows_sys::w!("")
    } else if theme.is_empty() {
        std::ptr::null()
    } else {
        theme.as_ptr()
    };
    unsafe {
        SetWindowTheme(
            hwnd,
            ptr,
            if list {
                windows_sys::w!("")
            } else {
                std::ptr::null()
            },
        );
        SendMessageW(hwnd, WM_THEMECHANGED, 0, 0);
        RedrawWindow(
            hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_FRAME | RDW_ERASE | RDW_ALLCHILDREN,
        );
        // The popup list is not enumerated among the parent's child windows.
        if class.eq_ignore_ascii_case("COMBOBOX") {
            let mut info: COMBOBOXINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<COMBOBOXINFO>() as u32;
            if GetComboBoxInfo(hwnd, &mut info) != 0 && !info.hwndList.is_null() {
                theme_control(info.hwndList, dark);
            }
        }
    }
}

/// Replaces the system accent highlight inside an EDIT control with the
/// palette selection colour. Standard EDIT controls have no message for this,
/// so the selected cell is overpainted after the control's own WM_PAINT.
unsafe fn paint_edit_selection(hwnd: HWND) {
    unsafe {
        let mut start: u32 = 0;
        let mut end: u32 = 0;
        SendMessageW(
            hwnd,
            0x00B0, // EM_GETSEL
            &mut start as *mut u32 as usize,
            &mut end as *mut u32 as isize,
        );
        if start == end {
            return;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return;
        }
        let mut text = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32);
        if copied <= 0 {
            return;
        }
        let len = copied as u32;
        let (start, end) = if start <= end {
            (start, end.min(len))
        } else {
            (end, start.min(len))
        };
        if start >= end || start >= len {
            return;
        }
        let pos = |index: u32| -> Option<POINT> {
            // Standard EDIT returns the position packed into the LRESULT;
            // unlike RichEdit it does not fill the POINT passed in lParam.
            let result = SendMessageW(
                hwnd,
                0x00D6, // EM_POSFROMCHAR
                index as usize,
                0,
            );
            if result < 0 {
                None
            } else {
                Some(POINT {
                    x: (result & 0xffff) as i16 as i32,
                    y: ((result >> 16) & 0xffff) as i16 as i32,
                })
            }
        };
        let Some(mut first) = pos(start) else {
            return;
        };
        let last = if end < len {
            pos(end)
        } else {
            pos(len - 1).map(|point| {
                let hdc = GetDC(hwnd);
                let font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as HFONT;
                let mut size = SIZE { cx: 0, cy: 0 };
                if !hdc.is_null() {
                    let old = SelectObject(hdc, font);
                    GetTextExtentPoint32W(hdc, text[len as usize - 1..].as_ptr(), 1, &mut size);
                    SelectObject(hdc, old);
                    ReleaseDC(hwnd, hdc);
                }
                POINT {
                    x: point.x + size.cx,
                    y: point.y,
                }
            })
        };
        let Some(mut last) = last else {
            return;
        };
        if first.x > last.x {
            std::mem::swap(&mut first, &mut last);
        }
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return;
        }
        let font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as HFONT;
        let old_font = SelectObject(hdc, font);
        let mut metrics: TEXTMETRICW = std::mem::zeroed();
        GetTextMetricsW(hdc, &mut metrics);
        let y = ((client.bottom - metrics.tmHeight) / 2).max(0);
        let fill = RECT {
            left: if start == 0 { 0 } else { first.x.max(0) },
            top: 0,
            right: last.x.max(first.x).min(client.right),
            bottom: client.bottom,
        };
        if fill.right > fill.left {
            let brush = CreateSolidBrush(EDIT_SEL_BG.load(Ordering::Relaxed));
            FillRect(hdc, &fill, brush);
            DeleteObject(brush as HGDIOBJ);
        }
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, EDIT_SEL_TEXT.load(Ordering::Relaxed));
        let selected = &text[start as usize..end as usize];
        TextOutW(hdc, first.x, y, selected.as_ptr(), selected.len() as i32);
        SelectObject(hdc, old_font);
        ReleaseDC(hwnd, hdc);
    }
}

/// Re-themes every native child control after a theme change.
pub fn theme_children(parent: HWND, dark: bool) {
    if parent.is_null() {
        return;
    }
    unsafe {
        EnumChildWindows(parent, Some(theme_child_proc), if dark { 1 } else { 0 });
    }
}

unsafe extern "system" fn theme_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    theme_control(hwnd, lparam != 0);
    1
}

pub fn set_window_title(hwnd: HWND, title: &str) {
    let title = wide(title);
    unsafe {
        SetWindowTextW(hwnd, title.as_ptr());
    }
}

pub fn menu_bar_height(dpi: u32, line_height: i32) -> i32 {
    use crate::ui::metrics;
    line_height
        + scale(metrics::MENU_TEXT_PAD_Y as i32, dpi) * 2
        + scale(metrics::MENU_FOOT_GAP as i32, dpi) * 2
        + 1
}

pub fn clipboard_has_text() -> bool {
    const CF_UNICODETEXT_VALUE: u32 = 13;
    unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT_VALUE) != 0 }
}

pub fn set_client_size(hwnd: HWND, width: i32, height: i32, dpi: u32) {
    unsafe {
        let mut frame = RECT {
            left: 0,
            top: 0,
            right: scale(width, dpi),
            bottom: scale(height, dpi),
        };
        AdjustWindowRectExForDpi(&mut frame, WS_OVERLAPPEDWINDOW, 0, 0, dpi);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            frame.right - frame.left,
            frame.bottom - frame.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

pub fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

pub fn invalidate(hwnd: HWND) {
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
}

/// Invalidates one logical rectangle of the client area. Used by live
/// result readouts so typing does not repaint (and disturb) the rest of the
/// window, including combo boxes.
pub fn invalidate_rect(hwnd: HWND, left: i32, top: i32, right: i32, bottom: i32) {
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    unsafe {
        InvalidateRect(hwnd, &rect, 0);
    }
}

pub fn redraw(hwnd: HWND) {
    unsafe {
        RedrawWindow(
            hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN | RDW_ERASE,
        );
    }
}

pub fn set_focus(hwnd: HWND) {
    unsafe {
        SetFocus(hwnd);
    }
}

pub fn message_box(hwnd: HWND, text: &str, title: &str) {
    let text = wide(text);
    let title = wide(title);
    unsafe {
        MessageBoxW(
            hwnd,
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

pub fn create_edit(parent: HWND, id: u16, x: i32, y: i32, w: i32, h: i32, text: &str) -> HWND {
    let text = wide(text);
    let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
    unsafe {
        CreateWindowExW(
            0,
            windows_sys::w!("EDIT"),
            text.as_ptr(),
            (WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32) as u32,
            x,
            y,
            w,
            h,
            parent,
            id as usize as HMENU,
            hinst,
            std::ptr::null(),
        )
    }
}

pub fn create_combo(
    parent: HWND,
    id: u16,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    items: &[&str],
    selected: usize,
) -> HWND {
    let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            windows_sys::w!("COMBOBOX"),
            std::ptr::null(),
            (WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | CBS_DROPDOWNLIST as u32
                | CBS_OWNERDRAWFIXED as u32
                | CBS_HASSTRINGS as u32) as u32,
            x,
            y,
            w,
            h,
            parent,
            id as usize as HMENU,
            hinst,
            std::ptr::null(),
        )
    };
    let dpi = crate::ui::DpiScale::new(unsafe { GetDpiForWindow(hwnd) }).dpi();
    unsafe {
        SendMessageW(
            hwnd,
            CB_SETITEMHEIGHT,
            0,
            crate::ui::DpiScale::new(dpi).px(crate::ui::metrics::COMBO_ITEM_H) as isize,
        );
        SendMessageW(
            hwnd,
            CB_SETITEMHEIGHT,
            usize::MAX,
            crate::ui::DpiScale::new(dpi).px(crate::ui::metrics::FIELD_H) as isize,
        );
    }
    combo_reset(hwnd);
    for item in items {
        combo_add(hwnd, item);
    }
    combo_select(hwnd, selected);
    install_combo_paint(hwnd);
    hwnd
}

pub fn combo_reset(hwnd: HWND) {
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
    }
}

pub fn combo_add(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        SendMessageW(hwnd, CB_ADDSTRING, 0, text.as_ptr() as isize);
    }
}

pub fn combo_select(hwnd: HWND, index: usize) {
    unsafe {
        SendMessageW(hwnd, CB_SETCURSEL, index, 0);
    }
}

pub fn combo_arrow_width(hwnd: HWND) -> i32 {
    let scale = crate::ui::DpiScale::new(unsafe { GetDpiForWindow(hwnd) });
    unsafe { GetSystemMetricsForDpi(SM_CXVSCROLL, scale.dpi()) }
        .max(scale.px(crate::ui::metrics::COMBO_ARROW_MIN_W))
}

pub fn combo_item_text(hwnd: HWND, index: usize) -> String {
    if hwnd.is_null() || index == usize::MAX {
        return String::new();
    }
    let len = unsafe { SendMessageW(hwnd, CB_GETLBTEXTLEN, index, 0) };
    if len < 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let copied = unsafe { SendMessageW(hwnd, CB_GETLBTEXT, index, buf.as_mut_ptr() as isize) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

pub fn combo_selected(hwnd: HWND) -> usize {
    let result = unsafe { SendMessageW(hwnd, CB_GETCURSEL, 0, 0) };
    if result < 0 {
        0
    } else {
        result as usize
    }
}

pub fn get_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

pub fn set_font(hwnd: HWND, font: HFONT) {
    unsafe {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
}

pub fn move_control(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        MoveWindow(hwnd, x, y, w, h, 1);
    }
}

pub fn show_control(hwnd: HWND, show: bool) {
    unsafe {
        ShowWindow(hwnd, if show { SW_SHOW } else { SW_HIDE });
    }
}

pub fn destroy_control(hwnd: HWND) {
    if !hwnd.is_null() {
        unsafe {
            DestroyWindow(hwnd);
        }
    }
}

pub fn copy_text(hwnd: HWND, text: &str) -> bool {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return false;
        }
        EmptyClipboard();
        let bytes = wide.len() * 2;
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if handle.is_null() {
            CloseClipboard();
            return false;
        }
        let ptr = GlobalLock(handle) as *mut u16;
        if ptr.is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        GlobalUnlock(handle);
        let ok = !SetClipboardData(13, handle).is_null();
        CloseClipboard();
        ok
    }
}

pub fn paste_text(hwnd: HWND) -> Option<String> {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return None;
        }
        let handle = GetClipboardData(13);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
            if len > 1_000_000 {
                break;
            }
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        GlobalUnlock(handle);
        CloseClipboard();
        Some(text)
    }
}

pub fn paste_to_focus(parent: HWND) -> bool {
    unsafe {
        let focus = GetFocus();
        if focus.is_null() || focus == parent {
            return false;
        }
        SendMessageW(focus, WM_PASTE, 0, 0);
        true
    }
}

pub fn set_hand_cursor(hand: bool) {
    unsafe {
        let cursor = LoadCursorW(
            std::ptr::null_mut(),
            if hand { IDC_HAND } else { IDC_ARROW },
        );
        SetCursor(cursor);
    }
}

pub fn capture_window_png(hwnd: HWND, path: &str) -> Result<(), String> {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err("GetWindowRect failed".into());
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err("invalid window size".into());
        }
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, width, height);
        if screen.is_null() || mem.is_null() || bmp.is_null() {
            return Err("GDI capture setup failed".into());
        }
        let old = SelectObject(mem, bmp);
        let printed = PrintWindow(hwnd, mem, 2);
        if printed == 0 {
            let ok = BitBlt(
                mem, 0, 0, width, height, screen, rect.left, rect.top, SRCCOPY,
            );
            if ok == 0 {
                SelectObject(mem, old);
                DeleteObject(bmp as HGDIOBJ);
                DeleteDC(mem);
                ReleaseDC(std::ptr::null_mut(), screen);
                return Err("PrintWindow and BitBlt failed".into());
            }
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let lines = GetDIBits(
            mem,
            bmp,
            0,
            height as u32,
            pixels.as_mut_ptr() as *mut c_void,
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(mem, old);
        DeleteObject(bmp as HGDIOBJ);
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
        if lines == 0 {
            return Err("GetDIBits failed".into());
        }
        png::write_bgra(path, width as u32, height as u32, &pixels).map_err(|e| e.to_string())
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Default-only messages (notably activation sent during popup creation and
    // destruction) must not form a second mutable borrow of the App handler.
    if !matches!(
        msg,
        WM_PAINT
            | WM_ERASEBKGND
            | WM_SIZE
            | WM_GETMINMAXINFO
            | WM_MOUSEMOVE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_MOUSEWHEEL
            | WM_KEYDOWN
            | WM_SYSKEYDOWN
            | WM_CHAR
            | WM_COMMAND
            | WM_TIMER
            | WM_DPICHANGED
            | WM_SETTINGCHANGE
            | WM_DESTROY
            | WM_CTLCOLOREDIT
            | WM_CTLCOLORLISTBOX
            | WM_CTLCOLORSTATIC
            | WM_MEASUREITEM
            | WM_DRAWITEM
    ) {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut c_void;
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let handler = unsafe { &mut *(ptr as *mut Box<dyn WindowHandler>) };
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            handler.on_paint(hdc);
            unsafe {
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            let w = (lparam as u32 & 0xffff) as i16 as i32;
            let h = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
            handler.on_resize(w, h);
            0
        }
        WM_GETMINMAXINFO => {
            let info = unsafe { &mut *(lparam as *mut MINMAXINFO) };
            let (w, h) = handler.min_client_size();
            if w > 0 && h > 0 {
                let dpi = crate::ui::DpiScale::new(unsafe { GetDpiForWindow(hwnd) }).dpi();
                let mut frame = RECT {
                    left: 0,
                    top: 0,
                    right: scale(w, dpi),
                    bottom: scale(h, dpi),
                };
                unsafe {
                    AdjustWindowRectExForDpi(&mut frame, WS_OVERLAPPEDWINDOW, 0, 0, dpi);
                }
                info.ptMinTrackSize.x = frame.right - frame.left;
                info.ptMinTrackSize.y = frame.bottom - frame.top;
            }
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam as u32 & 0xffff) as i16 as i32;
            let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
            handler.on_mouse_move(x, y);
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam as u32 & 0xffff) as i16 as i32;
            let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
            unsafe {
                SetCapture(hwnd);
            }
            handler.on_lbutton_down(x, y);
            0
        }
        WM_LBUTTONUP => {
            let x = (lparam as u32 & 0xffff) as i16 as i32;
            let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
            unsafe {
                ReleaseCapture();
            }
            handler.on_lbutton_up(x, y);
            0
        }
        WM_MOUSEWHEEL => {
            handler.on_wheel(((wparam as u32 >> 16) & 0xffff) as i16);
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam as u16;
            let ctrl = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
            if handler.on_key(vk, ctrl) {
                0
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        WM_CHAR => {
            let code = wparam as u32;
            if let Some(ch) = char::from_u32(code) {
                if handler.on_char(ch) {
                    return 0;
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as u16;
            let code = ((wparam >> 16) & 0xffff) as u16;
            if handler.on_command(id, code) {
                0
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        WM_TIMER => {
            handler.on_timer(wparam);
            0
        }
        WM_SETFOCUS => {
            handler.on_focus();
            0
        }
        WM_DPICHANGED => {
            let dpi = crate::ui::DpiScale::new((wparam & 0xffff) as u32).dpi();
            let suggested = unsafe { *(lparam as *const RECT) };
            unsafe {
                SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
            }
            // Update logical state/fonts before SetWindowPos sends WM_SIZE.
            handler.on_dpi(dpi);
            let (min_w, min_h) = handler.min_client_size();
            let mut minimum = RECT {
                left: 0,
                top: 0,
                right: scale(min_w, dpi),
                bottom: scale(min_h, dpi),
            };
            unsafe {
                AdjustWindowRectExForDpi(&mut minimum, WS_OVERLAPPEDWINDOW, 0, 0, dpi);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    suggested.left,
                    suggested.top,
                    (suggested.right - suggested.left).max(minimum.right - minimum.left),
                    (suggested.bottom - suggested.top).max(minimum.bottom - minimum.top),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
                RedrawWindow(
                    hwnd,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN,
                );
            }
            0
        }
        WM_SETTINGCHANGE => {
            handler.on_setting_change();
            0
        }
        WM_DESTROY => {
            handler.on_destroy();
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            let (bg, fg) = handler.control_colors();
            unsafe {
                SetBkColor(hdc, bg);
                SetTextColor(hdc, fg);
                handler.control_brush() as LRESULT
            }
        }
        WM_MEASUREITEM => {
            let item = unsafe { &mut *(lparam as *mut MEASUREITEMSTRUCT) };
            let handled = item.CtlType == ODT_COMBOBOX && handler.measure_combo_item(item);
            if handled {
                1
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        WM_DRAWITEM => {
            let item = unsafe { &mut *(lparam as *mut DRAWITEMSTRUCT) };
            let handled = item.CtlType == ODT_COMBOBOX && handler.draw_combo_item(item);
            if handled {
                1
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub fn run(
    handler: &mut Box<dyn WindowHandler>,
    title: &str,
    client_width: i32,
    client_height: i32,
) {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        install_menu_class();
        let hinst = GetModuleHandleW(std::ptr::null());
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        let big_icon = LoadImageW(
            hinst,
            1 as *const u16,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXICON),
            GetSystemMetrics(SM_CYICON),
            LR_SHARED,
        ) as HICON;
        let small_icon = LoadImageW(
            hinst,
            1 as *const u16,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_SHARED,
        ) as HICON;
        let class = wide("CalcFrameNative");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: big_icon,
            hCursor: cursor,
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: small_icon,
        };
        RegisterClassExW(&wc);
        let dpi = crate::ui::DpiScale::new(GetDpiForSystem()).dpi();
        let mut frame = RECT {
            left: 0,
            top: 0,
            right: scale(client_width, dpi),
            bottom: scale(client_height, dpi),
        };
        AdjustWindowRectExForDpi(&mut frame, WS_OVERLAPPEDWINDOW, 0, 0, dpi);
        let width = frame.right - frame.left;
        let height = frame.bottom - frame.top;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let title = wide(title);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            (screen_w - width) / 2,
            (screen_h - height) / 2,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        let handler_ptr = handler as *mut Box<dyn WindowHandler> as isize;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, handler_ptr);
        let window_dpi = crate::ui::DpiScale::new(GetDpiForWindow(hwnd)).dpi();
        handler.on_created(hwnd, window_dpi);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetForegroundWindow(hwnd);

        // View switching must work no matter which child currently owns the
        // focus. Unit/date/worksheet views create native EDIT and COMBOBOX
        // children, and without an accelerator table their WM_KEYDOWN is sent
        // to that child, so Ctrl+5 and later never reach the main window.
        let accelerators = mode_accelerators();
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if !accelerators.is_null() && TranslateAcceleratorW(hwnd, accelerators, &msg) != 0 {
                continue;
            }
            if msg.message == WM_KEYDOWN && IsChild(hwnd, msg.hwnd) != 0 {
                if msg.wParam as u16 == VK_TAB {
                    let previous = GetKeyState(VK_SHIFT as i32) < 0;
                    let next = GetNextDlgTabItem(hwnd, msg.hwnd, previous as i32);
                    if !next.is_null() {
                        SetFocus(next);
                    }
                    continue;
                }
                if msg.wParam as u16 == VK_F10 {
                    SendMessageW(hwnd, WM_KEYDOWN, VK_F10 as usize, 0);
                    continue;
                }
                if msg.wParam as u16 == VK_RETURN {
                    let id = GetDlgCtrlID(msg.hwnd);
                    if (300..=306).contains(&id) || (399..=405).contains(&id) {
                        SendMessageW(hwnd, WM_KEYDOWN, VK_RETURN as usize, 0);
                        continue;
                    }
                }
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if !accelerators.is_null() {
            DestroyAcceleratorTable(accelerators);
        }
    }
}

/// Ctrl+1…Ctrl+0 view switching plus the legacy Ctrl+U/Ctrl+E aliases.
/// Kept as data so the mapping can be regression-tested without a message loop.
pub(crate) const MODE_ACCELERATORS: [(u16, u16); 12] = [
    (VK_1, MODE_STANDARD),
    (VK_2, MODE_SCIENTIFIC),
    (VK_3, MODE_PROGRAMMER),
    (VK_4, MODE_STATISTICS),
    (VK_5, MODE_UNIT),
    (VK_6, MODE_DATE),
    (VK_7, MODE_MORTGAGE),
    (VK_8, MODE_LEASE),
    (VK_9, MODE_MPG),
    (VK_0, MODE_L100),
    (VK_U, MODE_UNIT),
    (VK_E, MODE_DATE),
];

fn mode_accelerators() -> HACCEL {
    // Command IDs match the View menu and the original mode commands.
    let table: Vec<ACCEL> = MODE_ACCELERATORS
        .iter()
        .map(|&(key, cmd)| ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key,
            cmd,
        })
        .collect();
    unsafe { CreateAcceleratorTableW(table.as_ptr(), table.len() as i32) }
}

pub fn destroy_window(hwnd: HWND) {
    unsafe {
        DestroyWindow(hwnd);
    }
}

pub fn set_timer(hwnd: HWND, id: usize, ms: u32) {
    unsafe {
        SetTimer(hwnd, id, ms, None);
    }
}

pub fn kill_timer(hwnd: HWND, id: usize) {
    unsafe {
        KillTimer(hwnd, id);
    }
}
