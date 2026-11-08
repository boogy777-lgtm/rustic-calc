use std::ffi::c_void;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;

use super::color::{blue, green, red, Color};
use super::geometry::{Rect, TextAlign};

/// Double-buffered GDI surface. All drawing goes through this low-level layer.
pub struct Canvas {
    target: HDC,
    mem: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    width: i32,
    height: i32,
}

impl Canvas {
    pub fn new(target: HDC, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        unsafe {
            let mem = CreateCompatibleDC(target);
            if mem.is_null() {
                return None;
            }
            let bmp = CreateCompatibleBitmap(target, width, height);
            if bmp.is_null() {
                DeleteDC(mem);
                return None;
            }
            let old = SelectObject(mem, bmp);
            Some(Self {
                target,
                mem,
                bmp,
                old,
                width,
                height,
            })
        }
    }

    pub fn present(&self) {
        unsafe {
            BitBlt(
                self.target,
                0,
                0,
                self.width,
                self.height,
                self.mem,
                0,
                0,
                SRCCOPY,
            );
        }
    }

    pub fn gradient(&self, rect: Rect, top: Color, bottom: Color) {
        let vertex = |color: Color, x: i32, y: i32| TRIVERTEX {
            x,
            y,
            Red: (red(color) as u16) << 8,
            Green: (green(color) as u16) << 8,
            Blue: (blue(color) as u16) << 8,
            Alpha: 0,
        };
        unsafe {
            let mut vertices = [
                vertex(top, rect.x, rect.y),
                vertex(bottom, rect.right(), rect.bottom()),
            ];
            let mesh = GRADIENT_RECT {
                UpperLeft: 0,
                LowerRight: 1,
            };
            GradientFill(
                self.mem,
                vertices.as_mut_ptr(),
                2,
                &mesh as *const _ as *const c_void,
                1,
                GRADIENT_FILL_RECT_V,
            );
        }
    }

    pub fn round_rect(&self, rect: Rect, radius: i32, fill: Color, border: Color) {
        unsafe {
            let brush = GetStockObject(DC_BRUSH);
            let pen = GetStockObject(DC_PEN);
            let old_brush = SelectObject(self.mem, brush);
            let old_pen = SelectObject(self.mem, pen);
            SetDCBrushColor(self.mem, fill);
            SetDCPenColor(self.mem, border);
            RoundRect(
                self.mem,
                rect.x,
                rect.y,
                rect.right(),
                rect.bottom(),
                radius * 2,
                radius * 2,
            );
            SelectObject(self.mem, old_brush);
            SelectObject(self.mem, old_pen);
        }
    }

    pub fn hline(&self, x1: i32, x2: i32, y: i32, color: Color) {
        unsafe {
            let pen = GetStockObject(DC_PEN);
            let old = SelectObject(self.mem, pen);
            SetDCPenColor(self.mem, color);
            MoveToEx(self.mem, x1, y, std::ptr::null_mut());
            LineTo(self.mem, x2, y);
            SelectObject(self.mem, old);
        }
    }

    pub fn vline(&self, x: i32, y1: i32, y2: i32, color: Color) {
        unsafe {
            let pen = GetStockObject(DC_PEN);
            let old = SelectObject(self.mem, pen);
            SetDCPenColor(self.mem, color);
            MoveToEx(self.mem, x, y1, std::ptr::null_mut());
            LineTo(self.mem, x, y2);
            SelectObject(self.mem, old);
        }
    }

    fn select_font(&self, font: HFONT) -> HGDIOBJ {
        unsafe { SelectObject(self.mem, font) }
    }

    pub fn text(&self, rect: Rect, text: &str, font: HFONT, color: Color, align: TextAlign) {
        if text.is_empty() {
            return;
        }
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut r = rect.to_win();
        let flags = DT_NOPREFIX | DT_VCENTER | DT_SINGLELINE | align.flags();
        unsafe {
            let old = self.select_font(font);
            SetBkMode(self.mem, TRANSPARENT as i32);
            SetTextColor(self.mem, color);
            DrawTextW(self.mem, wide.as_ptr(), wide.len() as i32, &mut r, flags);
            SelectObject(self.mem, old);
        }
    }

    pub fn measure(&self, text: &str, font: HFONT) -> i32 {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut size: SIZE = unsafe { std::mem::zeroed() };
        unsafe {
            let old = self.select_font(font);
            GetTextExtentPoint32W(self.mem, wide.as_ptr(), wide.len() as i32, &mut size);
            SelectObject(self.mem, old);
        }
        size.cx
    }

    /// Visible bounds relative to the text's baseline, excluding font leading.
    pub fn ink_bounds(&self, text: &str, font: HFONT) -> Option<Rect> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let unit = FIXED { fract: 0, value: 1 };
        let zero = FIXED { fract: 0, value: 0 };
        let transform = MAT2 {
            eM11: unit,
            eM12: zero,
            eM21: zero,
            eM22: unit,
        };
        let mut bounds: Option<Rect> = None;
        let mut valid = true;
        unsafe {
            let old_font = self.select_font(font);
            for (i, &ch) in wide.iter().enumerate() {
                let mut glyph: GLYPHMETRICS = std::mem::zeroed();
                if GetGlyphOutlineW(
                    self.mem,
                    ch as u32,
                    GGO_METRICS,
                    &mut glyph,
                    0,
                    std::ptr::null_mut(),
                    &transform,
                ) == GDI_ERROR as u32
                {
                    valid = false;
                    break;
                }
                if glyph.gmBlackBoxX == 0 || glyph.gmBlackBoxY == 0 {
                    continue;
                }
                let mut advance: SIZE = std::mem::zeroed();
                if i > 0 {
                    GetTextExtentPoint32W(self.mem, wide.as_ptr(), i as i32, &mut advance);
                }
                let glyph_rect = Rect::new(
                    advance.cx + glyph.gmptGlyphOrigin.x,
                    -glyph.gmptGlyphOrigin.y,
                    glyph.gmBlackBoxX as i32,
                    glyph.gmBlackBoxY as i32,
                );
                bounds = Some(match bounds {
                    None => glyph_rect,
                    Some(previous) => {
                        let left = previous.x.min(glyph_rect.x);
                        let top = previous.y.min(glyph_rect.y);
                        Rect::new(
                            left,
                            top,
                            previous.right().max(glyph_rect.right()) - left,
                            previous.bottom().max(glyph_rect.bottom()) - top,
                        )
                    }
                });
            }
            SelectObject(self.mem, old_font);
        }
        bounds.filter(|_| valid)
    }

    /// Center visible glyphs horizontally and the shared label line vertically.
    /// All titles retain one baseline even when only some contain descenders.
    pub fn text_ink_centered(
        &self,
        rect: Rect,
        text: &str,
        font: HFONT,
        color: Color,
        line_ink: Option<Rect>,
    ) {
        if text.is_empty() {
            return;
        }
        let wide: Vec<u16> = text.encode_utf16().collect();
        if let Some(ink) = self.ink_bounds(text, font) {
            let line = line_ink.unwrap_or(ink);
            unsafe {
                let old_font = self.select_font(font);
                let x = rect.x + (rect.w - ink.w) / 2 - ink.x;
                let y = rect.y + (rect.h - line.h) / 2 - line.y;
                let old_align = SetTextAlign(self.mem, TA_LEFT | TA_BASELINE | TA_NOUPDATECP);
                SetBkMode(self.mem, TRANSPARENT as i32);
                SetTextColor(self.mem, color);
                ExtTextOutW(
                    self.mem,
                    x,
                    y,
                    ETO_CLIPPED,
                    &rect.to_win(),
                    wide.as_ptr(),
                    wide.len() as u32,
                    std::ptr::null(),
                );
                SetTextAlign(self.mem, old_align);
                SelectObject(self.mem, old_font);
                return;
            }
        }
        self.text(rect, text, font, color, TextAlign::Center);
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem, self.old);
            DeleteObject(self.bmp as HGDIOBJ);
            DeleteDC(self.mem);
        }
    }
}
