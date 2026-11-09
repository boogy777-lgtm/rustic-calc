//! Palette-owned chrome for native EDIT, COMBOBOX and their drop-down list.
//! Text editing, selection, keyboard navigation and opening remain native.
use std::sync::atomic::{AtomicPtr, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::{
    GetComboBoxInfo, COMBOBOXINFO, EM_SETSEL, HOVER_DEFAULT, WM_MOUSELEAVE,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::ui::{
    metrics,
    palette::{Palette, DARK},
    DpiScale,
};

static PALETTE: AtomicPtr<Palette> = AtomicPtr::new(&DARK as *const Palette as *mut Palette);
const OLD_PROC: *const u16 = windows_sys::w!("Calc1ControlOldProc");
const HOT: *const u16 = windows_sys::w!("Calc1ControlHot");
const FOCUSED: *const u16 = windows_sys::w!("Calc1ControlFocused");

pub fn set_control_palette(palette: &'static Palette) {
    PALETTE.store(palette as *const Palette as *mut Palette, Ordering::Release);
    super::set_edit_colors(palette.selection_bg, palette.selection_text);
}

fn palette() -> &'static Palette {
    unsafe { &*PALETTE.load(Ordering::Acquire) }
}

fn border_color(p: &Palette, enabled: bool, focused: bool, hot: bool) -> u32 {
    if !enabled {
        p.disabled_border
    } else if focused {
        p.control_focus_border
    } else if hot {
        p.control_hover_border
    } else {
        p.border
    }
}

fn current_border(hwnd: HWND, dropped: bool) -> u32 {
    unsafe {
        border_color(
            palette(),
            IsWindowEnabled(hwnd) != 0,
            !GetPropW(hwnd, FOCUSED).is_null() || dropped,
            !GetPropW(hwnd, HOT).is_null(),
        )
    }
}

fn install(hwnd: HWND, proc: WNDPROC) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        if !GetPropW(hwnd, OLD_PROC).is_null() {
            return;
        }
        let old = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc.unwrap() as *const () as isize);
        SetPropW(hwnd, OLD_PROC, old as HANDLE);
        if GetFocus() == hwnd {
            SetPropW(hwnd, FOCUSED, 1usize as HANDLE);
        }
        RedrawWindow(
            hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_FRAME,
        );
    }
}

pub fn install_edit_paint(hwnd: HWND) {
    install(hwnd, Some(edit_proc));
}

pub fn install_combo_paint(hwnd: HWND) {
    install(hwnd, Some(combo_proc));
    let mut info: COMBOBOXINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<COMBOBOXINFO>() as u32;
    if unsafe { GetComboBoxInfo(hwnd, &mut info) } != 0 {
        install(info.hwndList, Some(list_proc));
    }
}

fn old_proc(hwnd: HWND) -> WNDPROC {
    unsafe { std::mem::transmute(GetPropW(hwnd, OLD_PROC) as isize) }
}

fn clean_properties(hwnd: HWND) {
    unsafe {
        RemovePropW(hwnd, OLD_PROC);
        RemovePropW(hwnd, HOT);
        RemovePropW(hwnd, FOCUSED);
    }
}

fn state_message(hwnd: HWND, msg: u32) -> bool {
    unsafe {
        match msg {
            WM_SETFOCUS => {
                SetPropW(hwnd, FOCUSED, 1usize as HANDLE);
                true
            }
            WM_KILLFOCUS => {
                RemovePropW(hwnd, FOCUSED);
                true
            }
            WM_MOUSEMOVE if GetPropW(hwnd, HOT).is_null() => {
                SetPropW(hwnd, HOT, 1usize as HANDLE);
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: HOVER_DEFAULT,
                };
                TrackMouseEvent(&mut tracking);
                true
            }
            WM_MOUSELEAVE => {
                RemovePropW(hwnd, HOT);
                true
            }
            WM_ENABLE | WM_THEMECHANGED | WM_SIZE => true,
            _ => false,
        }
    }
}

/// The whole non-client band is repainted, not just an overlay on a 3D edge.
fn paint_frame(hwnd: HWND, dc: HDC) {
    unsafe {
        let mut window: RECT = std::mem::zeroed();
        let mut client: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut window) == 0 {
            return;
        }
        GetClientRect(hwnd, &mut client);
        let mut origin = POINT { x: 0, y: 0 };
        ClientToScreen(hwnd, &mut origin);
        let width = window.right - window.left;
        let height = window.bottom - window.top;
        let x = origin.x - window.left;
        let y = origin.y - window.top;
        let saved = SaveDC(dc);
        let scale = DpiScale::new(GetDpiForWindow(hwnd));
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let scroll_w = if style & WS_VSCROLL != 0 {
            GetSystemMetricsForDpi(SM_CXVSCROLL, scale.dpi())
        } else {
            0
        };
        let scroll_h = if style & WS_HSCROLL != 0 {
            GetSystemMetricsForDpi(SM_CYHSCROLL, scale.dpi())
        } else {
            0
        };
        let p = palette();
        // Single-line EDIT may include WS_BORDER in its client area. Never
        // erase that client; replace only its outline after native painting.
        if x != 0 || y != 0 || client.right != width || client.bottom != height {
            ExcludeClipRect(
                dc,
                x,
                y,
                x + client.right + scroll_w,
                y + client.bottom + scroll_h,
            );
            super::fill_rect(
                dc,
                RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                },
                p.control_bg,
            );
        }
        let brush = CreateSolidBrush(current_border(hwnd, false));
        let outer = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        FrameRect(dc, &outer, brush);
        DeleteObject(brush as HGDIOBJ);
        RestoreDC(dc, saved);
        if style & WS_VSCROLL != 0 {
            paint_scrollbar(hwnd, dc, window);
        }
    }
}

/// Recolor native scrollbar geometry without replacing its hit targets/scrolling.
fn paint_scrollbar(hwnd: HWND, dc: HDC, window: RECT) {
    unsafe {
        let mut info: SCROLLBARINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SCROLLBARINFO>() as u32;
        if GetScrollBarInfo(hwnd, OBJID_VSCROLL, &mut info) == 0 {
            return;
        }
        let r = RECT {
            left: info.rcScrollBar.left - window.left,
            top: info.rcScrollBar.top - window.top,
            right: info.rcScrollBar.right - window.left,
            bottom: info.rcScrollBar.bottom - window.top,
        };
        if r.right <= r.left || r.bottom <= r.top {
            return;
        }
        let p = palette();
        let saved = SaveDC(dc);
        super::fill_rect(dc, r, p.control_bg);
        let scale = DpiScale::new(GetDpiForWindow(hwnd));
        let pad = scale.px(metrics::SCROLL_THUMB_PAD);
        let thumb_color = p.control_hover_border;
        let old_brush = SelectObject(dc, GetStockObject(DC_BRUSH));
        let old_pen = SelectObject(dc, GetStockObject(DC_PEN));
        SetDCBrushColor(dc, thumb_color);
        SetDCPenColor(dc, thumb_color);
        let radius = scale.px(metrics::RADIUS_ITEM) * 2;
        if info.xyThumbBottom > info.xyThumbTop {
            RoundRect(
                dc,
                r.left + pad,
                r.top + info.xyThumbTop,
                r.right - pad,
                r.top + info.xyThumbBottom,
                radius,
                radius,
            );
        }
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        if info.dxyLineButton > 0 {
            let pen = CreatePen(PS_SOLID, 1, p.muted);
            let old = SelectObject(dc, pen);
            let cx = (r.left + r.right) / 2;
            let half = scale.px(metrics::COMBO_ARROW_MIN_HALF).max(1);
            let dy = scale.px(metrics::COMBO_ARROW_MIN_H).max(1);
            for (cy, direction) in [
                (r.top + info.dxyLineButton / 2, -1),
                (r.bottom - info.dxyLineButton / 2, 1),
            ] {
                MoveToEx(dc, cx - half, cy - dy * direction, std::ptr::null_mut());
                LineTo(dc, cx, cy + dy * direction);
                LineTo(dc, cx + half, cy - dy * direction);
            }
            SelectObject(dc, old);
            DeleteObject(pen as HGDIOBJ);
        }
        RestoreDC(dc, saved);
    }
}

fn repaint_frame(hwnd: HWND) {
    unsafe {
        let dc = GetWindowDC(hwnd);
        if !dc.is_null() {
            paint_frame(hwnd, dc);
            ReleaseDC(hwnd, dc);
        }
    }
}

fn paint_combo(hwnd: HWND, dc: HDC) {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        let p = palette();
        super::fill_rect(dc, rect, p.control_bg);
        let dropped = SendMessageW(hwnd, CB_GETDROPPEDSTATE, 0, 0) != 0;
        let brush = CreateSolidBrush(current_border(hwnd, dropped));
        FrameRect(dc, &rect, brush);
        DeleteObject(brush as HGDIOBJ);
        let scale = DpiScale::new(GetDpiForWindow(hwnd));
        let arrow_w = super::combo_arrow_width(hwnd);
        let pad = scale.px(metrics::PAD_PANEL);
        let mut text_rect = rect;
        text_rect.left += pad;
        text_rect.right -= arrow_w + pad;
        let font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as HFONT;
        let text = super::combo_item_text(hwnd, super::combo_selected(hwnd));
        let text_color = if IsWindowEnabled(hwnd) != 0 {
            p.control_text
        } else {
            p.disabled_text
        };
        super::draw_text(
            dc,
            &mut text_rect,
            &text,
            font,
            text_color,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        let cx = rect.right - arrow_w / 2;
        let cy = rect.bottom / 2;
        let half = scale.px(metrics::COMBO_ARROW_MIN_HALF).max(1);
        let dy = scale.px(metrics::COMBO_ARROW_MIN_H).max(1) * if dropped { -1 } else { 1 };
        let pen = CreatePen(PS_SOLID, 1, text_color);
        let old = SelectObject(dc, pen);
        MoveToEx(dc, cx - half, cy - dy, std::ptr::null_mut());
        LineTo(dc, cx, cy + dy);
        LineTo(dc, cx + half, cy - dy);
        SelectObject(dc, old);
        DeleteObject(pen as HGDIOBJ);
    }
}

unsafe extern "system" fn combo_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let old = old_proc(hwnd);
    if old.is_none() {
        return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
    }
    if msg == WM_ERASEBKGND {
        return 1;
    }
    if msg == WM_PAINT {
        unsafe {
            let mut ps = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            let mem = CreateCompatibleDC(dc);
            let bmp = CreateCompatibleBitmap(dc, rect.right.max(1), rect.bottom.max(1));
            if !mem.is_null() && !bmp.is_null() {
                let previous = SelectObject(mem, bmp as HGDIOBJ);
                paint_combo(hwnd, mem);
                BitBlt(dc, 0, 0, rect.right, rect.bottom, mem, 0, 0, SRCCOPY);
                SelectObject(mem, previous);
            } else {
                paint_combo(hwnd, dc);
            }
            if !bmp.is_null() {
                DeleteObject(bmp as HGDIOBJ);
            }
            if !mem.is_null() {
                DeleteDC(mem);
            }
            EndPaint(hwnd, &ps);
        }
        return 0;
    }
    if msg == WM_PRINTCLIENT || msg == WM_PRINT {
        paint_combo(hwnd, wp as HDC);
        return 0;
    }
    let changed = state_message(hwnd, msg);
    if msg == WM_LBUTTONDOWN {
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }
    let result = unsafe { CallWindowProcW(old, hwnd, msg, wp, lp) };
    if changed
        || matches!(
            msg,
            CB_SETCURSEL | CB_SHOWDROPDOWN | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_KEYUP
        )
    {
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }
    if msg == WM_NCDESTROY {
        clean_properties(hwnd);
    }
    result
}

unsafe extern "system" fn edit_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let old = old_proc(hwnd);
    if old.is_none() {
        return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
    }
    if msg == WM_NCPAINT {
        repaint_frame(hwnd);
        return 0;
    }
    // Standard EDIT has no default Ctrl+A binding. Keep selection native;
    // do not read or modify the clipboard here (also works for read-only text).
    if msg == WM_KEYDOWN && wp as u16 == VK_A && unsafe { GetKeyState(VK_CONTROL as i32) } < 0 {
        unsafe {
            SendMessageW(hwnd, EM_SETSEL, 0, -1);
        }
        return 0;
    }
    let changed = state_message(hwnd, msg);
    let result = unsafe { CallWindowProcW(old, hwnd, msg, wp, lp) };
    if msg == WM_PAINT {
        unsafe {
            super::paint_edit_selection(hwnd);
        }
        repaint_frame(hwnd);
    }
    if msg == WM_PRINT && lp & PRF_NONCLIENT as isize != 0 {
        paint_frame(hwnd, wp as HDC);
    }
    if changed {
        repaint_frame(hwnd);
    }
    if msg == WM_NCDESTROY {
        clean_properties(hwnd);
    }
    result
}

unsafe extern "system" fn list_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let old = old_proc(hwnd);
    if old.is_none() {
        return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
    }
    let result = unsafe { CallWindowProcW(old, hwnd, msg, wp, lp) };
    if matches!(
        msg,
        WM_PAINT
            | WM_NCPAINT
            | WM_NCMOUSEMOVE
            | WM_MOUSEMOVE
            | WM_VSCROLL
            | WM_MOUSEWHEEL
            | WM_KEYDOWN
            | WM_KEYUP
            | WM_NCLBUTTONUP
    ) {
        repaint_frame(hwnd);
    }
    if msg == WM_PRINT && lp & PRF_NONCLIENT as isize != 0 {
        paint_frame(hwnd, wp as HDC);
    }
    if msg == WM_THEMECHANGED {
        repaint_frame(hwnd);
    }
    if msg == WM_NCDESTROY {
        clean_properties(hwnd);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette::{LIGHT, WHITE};
    #[test]
    fn every_theme_uses_shared_native_control_states() {
        for p in [&LIGHT, &WHITE, &DARK] {
            assert_eq!(border_color(p, true, false, false), p.border);
            assert_eq!(border_color(p, true, false, true), p.control_hover_border);
            assert_eq!(border_color(p, true, true, true), p.control_focus_border);
            assert_eq!(border_color(p, false, true, true), p.disabled_border);
            assert_ne!(p.control_focus_border, p.border);
        }
    }
}
