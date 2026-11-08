use windows_sys::Win32::Graphics::Gdi::*;

use super::metrics;

pub struct Fonts {
    pub small: HFONT,
    pub body: HFONT,
    pub body_height: i32,
    pub label: HFONT,
    pub button: HFONT,
    pub button_small: HFONT,
    pub title: HFONT,
    pub display: HFONT,
    pub display_mid: HFONT,
    pub display_small: HFONT,
}

fn make_font(size: i32, weight: i32, dpi: u32) -> HFONT {
    let height = -super::DpiScale::new(dpi).px(size as f32).max(1);
    let family = metrics::FONT_FAMILY
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            family.as_ptr(),
        )
    }
}

impl Fonts {
    pub fn new(dpi: u32) -> Self {
        let body = make_font(metrics::FONT_BODY, FW_NORMAL as i32, dpi);
        let body_height = unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            let old = SelectObject(dc, body);
            let mut metric: TEXTMETRICW = std::mem::zeroed();
            GetTextMetricsW(dc, &mut metric);
            SelectObject(dc, old);
            DeleteDC(dc);
            metric.tmHeight.max(1)
        };
        Self {
            small: make_font(metrics::FONT_SMALL, FW_NORMAL as i32, dpi),
            body,
            body_height,
            label: make_font(metrics::FONT_LABEL, FW_NORMAL as i32, dpi),
            button: make_font(metrics::FONT_BUTTON, FW_NORMAL as i32, dpi),
            button_small: make_font(metrics::FONT_BUTTON_SMALL, FW_NORMAL as i32, dpi),
            title: make_font(metrics::FONT_TITLE, FW_SEMIBOLD as i32, dpi),
            display: make_font(metrics::FONT_DISPLAY, FW_NORMAL as i32, dpi),
            display_mid: make_font(metrics::FONT_DISPLAY_MID, FW_NORMAL as i32, dpi),
            display_small: make_font(metrics::FONT_DISPLAY_SMALL, FW_NORMAL as i32, dpi),
        }
    }

    fn all(&self) -> [HFONT; 9] {
        [
            self.small,
            self.body,
            self.label,
            self.button,
            self.button_small,
            self.title,
            self.display,
            self.display_mid,
            self.display_small,
        ]
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            for font in self.all() {
                if !font.is_null() {
                    DeleteObject(font as HGDIOBJ);
                }
            }
        }
    }
}
