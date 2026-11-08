use crate::actions::Action;
use crate::engine::{Mode, WorksheetKind};
use crate::i18n::Lang;
use crate::platform::{self, WindowHandler};
use crate::ui::palette::{Palette, ThemeMode};
use crate::ui::{Canvas, Fonts, Hit, Rect, Ui};
use crate::views::{mode_client_size, Shared, ViewManager};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{
    DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

pub struct App {
    hwnd: HWND,
    dpi: u32,
    fonts: Fonts,
    manager: ViewManager,
    shared: Shared,
    hits: Vec<Hit>,
    hot: Option<u64>,
    pressed: Option<u64>,
    hot_menu: Option<usize>,
    pressed_menu: Option<usize>,
    last_menu: Option<usize>,
    menu_hits: Vec<Rect>,
    mode: Mode,
    screenshot: Option<String>,
    palette: &'static Palette,
    system_dark: bool,
    control_brush: HBRUSH,
}

impl App {
    pub fn new(mode: Mode, screenshot: Option<String>, lang: Lang, theme: ThemeMode) -> Self {
        let mut shared = Shared::new();
        shared.lang = lang;
        shared.theme = theme;
        let system_dark = platform::system_prefers_dark();
        let palette = theme.resolve(system_dark);
        let dark = theme == ThemeMode::Dark || (theme == ThemeMode::System && system_dark);
        shared.dark = dark;
        platform::set_control_palette(palette);
        Self {
            hwnd: std::ptr::null_mut(),
            dpi: crate::ui::dpi::BASE_DPI,
            fonts: Fonts::new(crate::ui::dpi::BASE_DPI),
            manager: ViewManager::new(),
            shared,
            hits: Vec::new(),
            hot: None,
            pressed: None,
            hot_menu: None,
            pressed_menu: None,
            last_menu: None,
            menu_hits: Vec::new(),
            mode,
            screenshot,
            palette,
            system_dark,
            control_brush: platform::solid_brush(palette.control_bg),
        }
    }

    fn menu_bar_height(&self) -> i32 {
        platform::menu_bar_height(self.dpi, self.fonts.body_height)
    }

    pub fn preferred_client_size(&self, mode: Mode) -> (i32, i32) {
        let dpi_scale = crate::ui::DpiScale::new(self.dpi);
        let scale = dpi_scale.factor();
        let menu_height = dpi_scale.minimum_dp(self.menu_bar_height());
        let (width, height) = mode_client_size(mode, menu_height);
        // Use both locales so switching language cannot crowd the outer frame.
        let menu_width = [Lang::Ru, Lang::En]
            .into_iter()
            .map(|lang| {
                let t = lang.text();
                let widths: Vec<_> = [
                    t.menu_view,
                    t.menu_edit,
                    t.menu_language,
                    t.menu_theme,
                    t.menu_help,
                ]
                .iter()
                .map(|label| platform::text_width(self.hwnd, self.fonts.body, label))
                .collect();
                menu_required_width(&widths, scale)
            })
            .max()
            .unwrap_or(0);
        (width.max(dpi_scale.minimum_dp(menu_width)), height)
    }

    fn draw_menu_bar(&mut self, canvas: &mut Canvas, width: i32) {
        let bar_h = self.menu_bar_height();
        if bar_h <= 0 || width <= 0 {
            return;
        }
        let scale = self.shared.scale;
        let text = self.shared.text();
        let labels = [
            text.menu_view,
            text.menu_edit,
            text.menu_language,
            text.menu_theme,
            text.menu_help,
        ];
        let widths: Vec<i32> = labels
            .iter()
            .map(|label| canvas.measure(label, self.fonts.body))
            .collect();
        let rects = menu_layout(&widths, width, bar_h, scale);
        let line_ink = canvas.ink_bounds(&labels.join(" "), self.fonts.body);
        canvas.round_rect(
            Rect::new(0, 0, width, bar_h),
            0,
            self.palette.panel,
            self.palette.panel,
        );
        self.menu_hits.clear();
        for (index, label) in labels.iter().enumerate() {
            let rect = rects[index];
            let button = menu_button_rect(rect, scale);
            if menu_title_selected(
                index,
                self.hot_menu,
                self.pressed_menu,
                self.last_menu,
                platform::root_is_open(),
            ) {
                canvas.round_rect(
                    button,
                    crate::ui::dpi::dp_to_px(crate::ui::metrics::RADIUS_ITEM, scale),
                    self.palette.selection_bg,
                    self.palette.selection_bg,
                );
            }
            canvas.text_ink_centered(button, label, self.fonts.body, self.palette.text, line_ink);
            self.menu_hits.push(rect);
        }
        canvas.hline(0, width, bar_h - 1, self.palette.panel_border);
    }

    fn menu_index_at(&self, x: i32, y: i32) -> Option<usize> {
        if y >= self.menu_bar_height() {
            return None;
        }
        self.menu_hits.iter().position(|rect| rect.contains(x, y))
    }

    fn menu_items(&self, index: usize) -> Vec<platform::MenuItem> {
        let t = self.shared.text();
        let active = mode_command(self.mode);
        match index {
            0 => {
                let worksheets = vec![
                    platform::MenuItem::item(
                        t.menu_mortgage,
                        "Ctrl+7",
                        platform::MODE_MORTGAGE,
                        active == platform::MODE_MORTGAGE,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_lease,
                        "Ctrl+8",
                        platform::MODE_LEASE,
                        active == platform::MODE_LEASE,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_mpg,
                        "Ctrl+9",
                        platform::MODE_MPG,
                        active == platform::MODE_MPG,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_l100,
                        "Ctrl+0",
                        platform::MODE_L100,
                        active == platform::MODE_L100,
                        true,
                    ),
                ];
                vec![
                    platform::MenuItem::item(
                        t.menu_standard,
                        "Ctrl+1",
                        platform::MODE_STANDARD,
                        active == platform::MODE_STANDARD,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_scientific,
                        "Ctrl+2",
                        platform::MODE_SCIENTIFIC,
                        active == platform::MODE_SCIENTIFIC,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_programmer,
                        "Ctrl+3",
                        platform::MODE_PROGRAMMER,
                        active == platform::MODE_PROGRAMMER,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_statistics,
                        "Ctrl+4",
                        platform::MODE_STATISTICS,
                        active == platform::MODE_STATISTICS,
                        true,
                    ),
                    platform::MenuItem::separator(),
                    platform::MenuItem::item(
                        t.menu_unit,
                        "Ctrl+5",
                        platform::MODE_UNIT,
                        active == platform::MODE_UNIT,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_date,
                        "Ctrl+6",
                        platform::MODE_DATE,
                        active == platform::MODE_DATE,
                        true,
                    ),
                    platform::MenuItem::submenu(t.menu_worksheets, worksheets),
                    platform::MenuItem::separator(),
                    platform::MenuItem::item(
                        t.menu_history,
                        "Ctrl+H",
                        platform::CMD_HISTORY,
                        self.shared.show_history,
                        true,
                    ),
                    platform::MenuItem::item(
                        t.menu_grouping,
                        "",
                        platform::CMD_GROUPING,
                        self.shared.grouping,
                        true,
                    ),
                ]
            }
            1 => vec![
                platform::MenuItem::item(t.menu_copy, "Ctrl+C", platform::CMD_COPY, false, true),
                platform::MenuItem::item(
                    t.menu_paste,
                    "Ctrl+V",
                    platform::CMD_PASTE,
                    false,
                    platform::clipboard_has_text(),
                ),
            ],
            2 => vec![
                platform::MenuItem::item(
                    t.lang_russian,
                    "",
                    platform::CMD_LANG_RU,
                    self.shared.lang == Lang::Ru,
                    true,
                ),
                platform::MenuItem::item(
                    t.lang_english,
                    "",
                    platform::CMD_LANG_EN,
                    self.shared.lang == Lang::En,
                    true,
                ),
            ],
            3 => vec![
                platform::MenuItem::item(
                    t.theme_system,
                    "",
                    platform::CMD_THEME_SYSTEM,
                    self.shared.theme == ThemeMode::System,
                    true,
                ),
                platform::MenuItem::item(
                    t.theme_white,
                    "",
                    platform::CMD_THEME_WHITE,
                    self.shared.theme == ThemeMode::White,
                    true,
                ),
                platform::MenuItem::item(
                    t.theme_dark,
                    "",
                    platform::CMD_THEME_DARK,
                    self.shared.theme == ThemeMode::Dark,
                    true,
                ),
            ],
            4 => vec![
                platform::MenuItem::item(t.menu_help_item, "F1", platform::CMD_HELP, false, true),
                platform::MenuItem::item(t.menu_about, "", platform::CMD_ABOUT, false, true),
            ],
            _ => Vec::new(),
        }
    }

    fn open_menu(&mut self, index: usize) {
        let Some(rect) = self.menu_hits.get(index).copied() else {
            return;
        };
        let items = self.menu_items(index);
        let style = platform::MenuStyle {
            panel: self.palette.panel,
            border: self.palette.panel_border,
            text: self.palette.text,
            muted: self.palette.muted,
            selection: self.palette.selection_bg,
            selection_text: self.palette.selection_text,
            disabled: self.palette.disabled_text,
            body: self.fonts.body,
            small: self.fonts.small,
            scale: self.shared.scale,
        };
        // The menu bar's bottom border is the last pixel of the bar, so the
        // popup starts exactly at `rect.bottom()` without covering it.
        self.last_menu = Some(index);
        self.hot_menu = None;
        self.pressed_menu = None;
        platform::show_menu(self.hwnd, items, rect.x, rect.bottom(), style);
        platform::redraw(self.hwnd);
    }

    fn set_language(&mut self, lang: Lang) {
        if self.shared.lang == lang {
            return;
        }
        self.shared.lang = lang;
        platform::set_window_title(self.hwnd, lang.text().app_title);
        if let Some(view) = self.manager.active_mut() {
            view.on_exit(&mut self.shared);
            view.on_enter(&mut self.shared);
        }
        platform::invalidate(self.hwnd);
    }

    fn set_theme(&mut self, mode: ThemeMode) {
        if self.shared.theme == mode {
            return;
        }
        self.shared.theme = mode;
        self.apply_palette(mode.resolve(self.system_dark));
        self.sync_theme();
        platform::redraw(self.hwnd);
    }

    fn dark_mode(&self) -> bool {
        match self.shared.theme {
            ThemeMode::Dark => true,
            ThemeMode::White => false,
            ThemeMode::System => self.system_dark,
        }
    }

    fn sync_theme(&mut self) {
        let dark = self.dark_mode();
        self.shared.dark = dark;
        platform::set_dark_titlebar(self.hwnd, dark);
        platform::theme_children(self.hwnd, dark);
    }

    fn apply_palette(&mut self, palette: &'static Palette) {
        self.palette = palette;
        platform::delete_brush(self.control_brush);
        self.control_brush = platform::solid_brush(palette.control_bg);
        platform::set_control_palette(palette);
    }

    fn switch_mode(&mut self, mode: Mode) {
        if self.manager.active_mode() == Some(mode) {
            return;
        }
        // Set the mode before resizing: WM_GETMINMAXINFO consults it, so a
        // wide→compact switch must lower the minimum first.
        self.mode = mode;
        let (w, h) = self.preferred_client_size(mode);
        platform::set_client_size(self.hwnd, w, h, self.dpi);
        self.shared.client = client_rect(self.hwnd, self.menu_bar_height());
        self.manager.activate(mode, &mut self.shared);
        self.hot = None;
        self.pressed = None;
        platform::invalidate(self.hwnd);
    }

    fn copy(&mut self) {
        let text = self
            .manager
            .active_ref()
            .map(|view| view.display(&self.shared))
            .unwrap_or_default();
        platform::copy_text(self.hwnd, &text);
    }

    fn paste(&mut self) {
        if platform::paste_to_focus(self.hwnd) {
            return;
        }
        if let Some(text) = platform::paste_text(self.hwnd) {
            for ch in text.chars() {
                if let Some(view) = self.manager.active_mut() {
                    view.char(ch, &mut self.shared);
                }
            }
            platform::invalidate(self.hwnd);
        }
    }

    fn toggle_history(&mut self) {
        self.shared.show_history = !self.shared.show_history;
        platform::invalidate(self.hwnd);
    }

    fn toggle_grouping(&mut self) {
        self.shared.grouping = !self.shared.grouping;
        platform::invalidate(self.hwnd);
    }

    fn show_help(&self) {
        let t = self.shared.text();
        platform::message_box(self.hwnd, t.help_text, t.help_title);
    }

    fn show_about(&self) {
        let t = self.shared.text();
        platform::message_box(self.hwnd, t.about_text, t.about_title);
    }

    fn handle_menu(&mut self, id: u16) -> bool {
        if id == platform::CMD_MENU_PREVIOUS || id == platform::CMD_MENU_NEXT {
            let current = self.last_menu.unwrap_or(0);
            let count = self.menu_hits.len();
            if count != 0 {
                let next = if id == platform::CMD_MENU_NEXT {
                    (current + 1) % count
                } else {
                    (current + count - 1) % count
                };
                self.open_menu(next);
            }
            return true;
        }
        let mode = match id {
            platform::MODE_STANDARD => Some(Mode::Standard),
            platform::MODE_SCIENTIFIC => Some(Mode::Scientific),
            platform::MODE_PROGRAMMER => Some(Mode::Programmer),
            platform::MODE_STATISTICS => Some(Mode::Statistics),
            platform::MODE_UNIT => Some(Mode::UnitConversion),
            platform::MODE_DATE => Some(Mode::DateCalculation),
            platform::MODE_MORTGAGE => Some(Mode::Worksheet(WorksheetKind::Mortgage)),
            platform::MODE_LEASE => Some(Mode::Worksheet(WorksheetKind::VehicleLease)),
            platform::MODE_MPG => Some(Mode::Worksheet(WorksheetKind::FuelMpg)),
            platform::MODE_L100 => Some(Mode::Worksheet(WorksheetKind::FuelL100)),
            _ => None,
        };
        if let Some(mode) = mode {
            self.switch_mode(mode);
            return true;
        }
        match id {
            platform::CMD_HISTORY => self.toggle_history(),
            platform::CMD_GROUPING => self.toggle_grouping(),
            platform::CMD_COPY => self.copy(),
            platform::CMD_PASTE => self.paste(),
            platform::CMD_HELP => self.show_help(),
            platform::CMD_ABOUT => self.show_about(),
            platform::CMD_EXIT => platform::destroy_window(self.hwnd),
            platform::CMD_LANG_RU => self.set_language(Lang::Ru),
            platform::CMD_LANG_EN => self.set_language(Lang::En),
            platform::CMD_THEME_SYSTEM => self.set_theme(ThemeMode::System),
            platform::CMD_THEME_WHITE => self.set_theme(ThemeMode::White),
            platform::CMD_THEME_DARK => self.set_theme(ThemeMode::Dark),
            _ => return false,
        }
        true
    }
}

// Prefer roomy spacing, but distribute the available width before clipping text.
fn menu_layout(widths: &[i32], width: i32, height: i32, scale: f32) -> Vec<Rect> {
    let s = |dp| crate::ui::dpi::dp_to_px(dp, scale);
    let inset = s(crate::ui::metrics::MENU_EDGE_PAD);
    let desired_gap = s(crate::ui::metrics::MENU_ITEM_GAP);
    let desired_pad = s(crate::ui::metrics::MENU_TEXT_PAD_X);
    let n = widths.len() as i32;
    if n == 0 {
        return Vec::new();
    }
    let free = (width - inset * 2 - widths.iter().sum::<i32>()).max(0);
    let gap = desired_gap.min(free / (n * 2 + (n - 1)).max(1));
    let pad = desired_pad.min((free - gap * (n - 1)).max(0) / (2 * n));
    // When a compact window constrains padding, balance the rounding remainder
    // across the two outer edges. Wide menu bars retain native left alignment.
    let used = widths.iter().sum::<i32>() + pad * 2 * n + gap * (n - 1);
    let mut x = if pad < desired_pad {
        (width - used).max(0) / 2
    } else {
        inset
    };
    widths
        .iter()
        .map(|&w| {
            let r = Rect::new(x, 0, w + pad * 2, height);
            x = r.right() + gap;
            r
        })
        .collect()
}

fn menu_required_width(widths: &[i32], scale: f32) -> i32 {
    use crate::ui::metrics;
    let s = |v: f32| crate::ui::dpi::dp_to_px(v, scale);
    let count = widths.len() as i32;
    widths.iter().sum::<i32>()
        + count * s(metrics::MENU_TEXT_PAD_X) * 2
        + (count - 1).max(0) * s(metrics::MENU_ITEM_GAP)
        + s(metrics::MENU_EDGE_PAD) * 2
}

#[cfg(test)]
mod menu_fit_tests {
    use super::{
        dismiss_click_switches, menu_button_rect, menu_layout, menu_required_width,
        menu_title_selected,
    };

    #[test]
    fn native_date_edits_survive_dpi_round_trip_without_size_accumulation() {
        use super::App;
        use crate::{
            engine::Mode,
            i18n::Lang,
            platform::{self, WindowHandler},
            ui::palette::ThemeMode,
        };
        use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
        struct HiddenWindow(HWND);
        impl Drop for HiddenWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        // Hidden, test-owned HWND; no global display settings or foreground changes.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                windows_sys::w!("STATIC"),
                windows_sys::w!("Calc1 DPI regression"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                640,
                480,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        let _window = HiddenWindow(hwnd);
        let mut app = App::new(Mode::DateCalculation, None, Lang::Ru, ThemeMode::White);
        app.on_created(hwnd, 96);
        let original_size = app.preferred_client_size(Mode::DateCalculation);
        let value: Vec<u16> = "01.01.2030\0".encode_utf16().collect();
        let edit = unsafe { GetDlgItem(hwnd, 301) };
        assert!(!edit.is_null());
        unsafe {
            SetWindowTextW(edit, value.as_ptr());
        }
        app.on_command(
            301,
            windows_sys::Win32::UI::WindowsAndMessaging::EN_CHANGE as u16,
        );
        for dpi in [144, 192, 72, 96] {
            app.on_dpi(dpi);
            assert_eq!(app.shared.scale, crate::ui::DpiScale::new(dpi).factor());
            let edit = unsafe { GetDlgItem(hwnd, 301) };
            assert_eq!(platform::get_text(edit), "01.01.2030");
        }
        assert_eq!(
            app.preferred_client_size(Mode::DateCalculation),
            original_size
        );
        app.on_destroy();
    }

    #[test]
    fn actual_ru_en_fonts_have_full_line_height_and_frame_clearance() {
        use crate::{
            i18n::Lang,
            platform,
            ui::{metrics, Fonts},
        };
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let fonts = Fonts::new(dpi);
            let scale = crate::ui::DpiScale::new(dpi).factor();
            let s = |v: f32| (v * scale).round() as i32;
            for lang in [Lang::Ru, Lang::En] {
                let t = lang.text();
                let widths: Vec<_> = [
                    t.menu_view,
                    t.menu_edit,
                    t.menu_language,
                    t.menu_theme,
                    t.menu_help,
                ]
                .iter()
                .map(|label| platform::text_width(std::ptr::null_mut(), fonts.body, label))
                .collect();
                let width = menu_required_width(&widths, scale);
                let height = platform::menu_bar_height(dpi, fonts.body_height);
                let rects = menu_layout(&widths, width, height, scale);
                assert_eq!(rects[0].x, s(metrics::MENU_EDGE_PAD));
                assert_eq!(
                    width - rects.last().unwrap().right(),
                    s(metrics::MENU_EDGE_PAD)
                );
                for (hit, text_width) in rects.iter().zip(&widths) {
                    let button = menu_button_rect(*hit, scale);
                    assert_eq!(
                        button.h,
                        fonts.body_height + s(metrics::MENU_TEXT_PAD_Y) * 2
                    );
                    assert_eq!(button.w - text_width, s(metrics::MENU_TEXT_PAD_X) * 2);
                }
            }
        }
    }

    #[test]
    fn open_title_survives_hover_and_dismissal_does_not_reopen_same_title() {
        let selected = |open, hot, index| menu_title_selected(index, hot, None, Some(0), open);
        assert!(selected(true, None, 0));
        assert!(selected(true, Some(1), 0));
        assert!(!selected(false, None, 0));
        assert!(!dismiss_click_switches(Some(0), Some(0)));
        assert!(!dismiss_click_switches(Some(0), None));
        assert!(dismiss_click_switches(Some(0), Some(1)));
        assert!(menu_title_selected(1, None, None, Some(1), true));
        assert!(!menu_title_selected(0, None, None, Some(1), true));
    }

    #[test]
    fn measured_labels_fit_compact_bar_at_supported_scales() {
        for dpi in [96, 120, 144, 192] {
            let scale = dpi as f32 / 96.0;
            // Representative measured widths including a long Russian Help label.
            let labels: Vec<_> = [29, 55, 35, 38, 55]
                .into_iter()
                .map(|w| (w as f32 * scale).round() as i32)
                .collect();
            let width = (307.0 * scale).round() as i32;
            let rects = menu_layout(&labels, width, 30, scale);
            assert!(rects.last().unwrap().right() <= width - (4.0 * scale).round() as i32);
            for (r, label) in rects.iter().zip(labels) {
                assert!(r.w >= label);
            }
            assert!(rects.windows(2).all(|w| w[0].right() <= w[1].x));
            let left = rects[0].x;
            let right = width - rects.last().unwrap().right();
            assert!((left - right).abs() <= 1);
        }
    }
}

/// Keep the full-height mouse target, but never paint selection over the
/// separator or its breathing room. Popup anchoring still uses the full bar.
pub(crate) fn menu_button_rect(hit: Rect, scale: f32) -> Rect {
    let gap = crate::ui::dpi::dp_to_px(crate::ui::metrics::MENU_FOOT_GAP, scale);
    Rect::new(hit.x, hit.y + gap, hit.w, (hit.h - gap * 2 - 1).max(0))
}

fn mode_command(mode: Mode) -> u16 {
    match mode {
        Mode::Standard => platform::MODE_STANDARD,
        Mode::Scientific => platform::MODE_SCIENTIFIC,
        Mode::Programmer => platform::MODE_PROGRAMMER,
        Mode::Statistics => platform::MODE_STATISTICS,
        Mode::UnitConversion => platform::MODE_UNIT,
        Mode::DateCalculation => platform::MODE_DATE,
        Mode::Worksheet(WorksheetKind::Mortgage) => platform::MODE_MORTGAGE,
        Mode::Worksheet(WorksheetKind::VehicleLease) => platform::MODE_LEASE,
        Mode::Worksheet(WorksheetKind::FuelMpg) => platform::MODE_MPG,
        Mode::Worksheet(WorksheetKind::FuelL100) => platform::MODE_L100,
    }
}

fn client_rect(hwnd: HWND, menu_bar_h: i32) -> Rect {
    let (w, h) = platform::client_size(hwnd);
    Rect::new(0, menu_bar_h, w, (h - menu_bar_h).max(0))
}

impl WindowHandler for App {
    fn on_created(&mut self, hwnd: HWND, dpi: u32) {
        let dpi = crate::ui::DpiScale::new(dpi).dpi();
        self.hwnd = hwnd;
        self.dpi = dpi;
        self.fonts = Fonts::new(dpi);
        self.shared.attach(hwnd, dpi, self.fonts.body);
        let (w, h) = self.preferred_client_size(self.mode);
        platform::set_client_size(hwnd, w, h, dpi);
        self.shared.client = client_rect(hwnd, self.menu_bar_height());
        platform::set_window_title(hwnd, self.shared.text().app_title);
        self.sync_theme();
        self.manager.activate(self.mode, &mut self.shared);
        if self.screenshot.is_some() {
            platform::set_timer(hwnd, 1, 700);
        }
    }

    fn on_paint(&mut self, hdc: HDC) {
        let (w, h) = platform::client_size(self.hwnd);
        let Some(mut canvas) = Canvas::new(hdc, w, h) else {
            return;
        };
        canvas.gradient(
            Rect::new(0, 0, w, h),
            self.palette.bg_top,
            self.palette.bg_bottom,
        );
        self.draw_menu_bar(&mut canvas, w);
        let palette = self.palette;
        let scale = self.shared.scale;
        let App {
            manager,
            shared,
            fonts,
            hits,
            hot,
            pressed,
            ..
        } = self;
        let mut ui = Ui::new(&mut canvas, fonts, palette, scale, *hot, *pressed);
        if let Some(view) = manager.active_mut() {
            view.paint(&mut ui, shared);
        }
        *hits = ui.finish();
        canvas.present();
    }

    fn on_resize(&mut self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let bar_h = self.menu_bar_height();
        self.shared.client = Rect::new(0, bar_h, width, (height - bar_h).max(0));
        if let Some(view) = self.manager.active_mut() {
            view.on_resize(&mut self.shared);
        }
        platform::invalidate(self.hwnd);
    }

    fn on_mouse_move(&mut self, x: i32, y: i32) {
        if let Some(index) = self.menu_index_at(x, y) {
            platform::set_hand_cursor(false);
            if platform::root_is_open() && self.last_menu != Some(index) {
                platform::close_menus();
                self.open_menu(index);
                return;
            }
            if self.hot_menu != Some(index) {
                self.hot_menu = Some(index);
                self.hot = None;
                platform::invalidate(self.hwnd);
            }
            return;
        }
        if self.hot_menu.take().is_some() {
            platform::invalidate(self.hwnd);
        }
        let id = Ui::hit_at(&self.hits, x, y).map(|hit| hit.id);
        platform::set_hand_cursor(id.is_some());
        if id != self.hot {
            self.hot = id;
            platform::invalidate(self.hwnd);
        }
    }

    fn on_lbutton_down(&mut self, x: i32, y: i32) {
        // The first click after an open popup dismisses it, exactly like a
        // native menu. Do not also press the menu title or a key underneath.
        // Clicking a different menu-bar title still switches to that menu.
        if platform::take_dismiss_click() {
            let target = self.menu_index_at(x, y);
            let switch = dismiss_click_switches(self.last_menu, target);
            if !switch {
                self.hot_menu = None;
                self.pressed_menu = None;
                self.pressed = None;
                platform::invalidate(self.hwnd);
                return;
            }
        }
        if let Some(index) = self.menu_index_at(x, y) {
            self.hot_menu = Some(index);
            self.pressed_menu = Some(index);
            self.hot = None;
            self.pressed = None;
            platform::invalidate(self.hwnd);
            return;
        }
        self.pressed_menu = None;
        let id = Ui::hit_at(&self.hits, x, y).map(|hit| hit.id);
        self.hot = id;
        self.pressed = id;
        platform::invalidate(self.hwnd);
    }

    fn on_lbutton_up(&mut self, x: i32, y: i32) {
        if let Some(index) = self.pressed_menu {
            self.pressed_menu = None;
            if self.menu_index_at(x, y) == Some(index) {
                self.open_menu(index);
            } else {
                self.hot_menu = None;
                platform::invalidate(self.hwnd);
            }
            return;
        }
        let hit = Ui::hit_at(&self.hits, x, y);
        let pressed = self.pressed;
        self.pressed = None;
        if let Some(hit) = hit {
            if pressed == Some(hit.id) {
                if let Some(view) = self.manager.active_mut() {
                    view.action(hit.action, &mut self.shared);
                }
            }
        }
        self.hot = Ui::hit_at(&self.hits, x, y).map(|hit| hit.id);
        platform::invalidate(self.hwnd);
    }

    fn on_key(&mut self, vk: u16, ctrl: bool) -> bool {
        if !ctrl && vk == VK_F10 {
            self.open_menu(0);
            return true;
        }
        if !ctrl
            && vk == VK_RETURN
            && matches!(self.mode, Mode::DateCalculation | Mode::Worksheet(_))
        {
            if let Some(view) = self.manager.active_mut() {
                view.action(Action::Calculate, &mut self.shared);
                platform::invalidate(self.hwnd);
            }
            return true;
        }
        if ctrl {
            match vk {
                VK_1 => {
                    self.switch_mode(Mode::Standard);
                    return true;
                }
                VK_2 => {
                    self.switch_mode(Mode::Scientific);
                    return true;
                }
                VK_3 => {
                    self.switch_mode(Mode::Programmer);
                    return true;
                }
                VK_4 => {
                    self.switch_mode(Mode::Statistics);
                    return true;
                }
                VK_5 => {
                    self.switch_mode(Mode::UnitConversion);
                    return true;
                }
                VK_6 => {
                    self.switch_mode(Mode::DateCalculation);
                    return true;
                }
                VK_7 => {
                    self.switch_mode(Mode::Worksheet(WorksheetKind::Mortgage));
                    return true;
                }
                VK_8 => {
                    self.switch_mode(Mode::Worksheet(WorksheetKind::VehicleLease));
                    return true;
                }
                VK_9 => {
                    self.switch_mode(Mode::Worksheet(WorksheetKind::FuelMpg));
                    return true;
                }
                VK_0 => {
                    self.switch_mode(Mode::Worksheet(WorksheetKind::FuelL100));
                    return true;
                }
                VK_U => {
                    self.switch_mode(Mode::UnitConversion);
                    return true;
                }
                VK_E => {
                    self.switch_mode(Mode::DateCalculation);
                    return true;
                }
                VK_C => {
                    self.copy();
                    return true;
                }
                VK_V => {
                    self.paste();
                    return true;
                }
                VK_H => {
                    self.toggle_history();
                    return true;
                }
                _ => {}
            }
        }
        if vk == VK_F1 {
            self.show_help();
            return true;
        }
        if let Some(view) = self.manager.active_mut() {
            if view.key(vk, ctrl, &mut self.shared) {
                platform::invalidate(self.hwnd);
                return true;
            }
        }
        false
    }

    fn on_char(&mut self, ch: char) -> bool {
        if ch < ' ' {
            return false;
        }
        if let Some(view) = self.manager.active_mut() {
            let handled = view.char(ch, &mut self.shared);
            if handled {
                platform::invalidate(self.hwnd);
            }
            handled
        } else {
            false
        }
    }

    fn on_command(&mut self, id: u16, code: u16) -> bool {
        if self.handle_menu(id) {
            return true;
        }
        if let Some(view) = self.manager.active_mut() {
            let handled = view.command(id, code, &mut self.shared);
            if handled {
                if let Some(rect) = view.command_rect(id, &self.shared) {
                    platform::invalidate_rect(
                        self.hwnd,
                        rect.x,
                        rect.y,
                        rect.right(),
                        rect.bottom(),
                    );
                } else {
                    platform::invalidate(self.hwnd);
                }
            }
            handled
        } else {
            false
        }
    }

    fn on_timer(&mut self, id: usize) {
        if id != 1 {
            return;
        }
        platform::kill_timer(self.hwnd, 1);
        if let Some(path) = self.screenshot.take() {
            if let Err(err) = platform::capture_window_png(self.hwnd, &path) {
                let _ = std::fs::write(format!("{path}.error"), err);
            }
        }
        platform::destroy_window(self.hwnd);
    }

    fn on_focus(&mut self) {}

    fn on_wheel(&mut self, _delta: i16) {}

    fn on_dpi(&mut self, dpi: u32) {
        let dpi = crate::ui::DpiScale::new(dpi).dpi();
        if self.dpi == dpi {
            return;
        }
        // Popup MenuStyle borrows HFONTs. Destroy it before dropping old fonts.
        platform::close_menus();
        self.menu_hits.clear();
        self.hot_menu = None;
        self.pressed_menu = None;
        self.dpi = dpi;
        if let Some(view) = self.manager.active_mut() {
            view.on_exit(&mut self.shared);
        }
        self.fonts = Fonts::new(dpi);
        self.shared.attach(self.hwnd, dpi, self.fonts.body);
        self.shared.client = client_rect(self.hwnd, self.menu_bar_height());
        if let Some(view) = self.manager.active_mut() {
            view.on_enter(&mut self.shared);
        }
        platform::invalidate(self.hwnd);
    }

    fn on_setting_change(&mut self) {
        let system_dark = platform::system_prefers_dark();
        if system_dark == self.system_dark {
            return;
        }
        self.system_dark = system_dark;
        if self.shared.theme == ThemeMode::System {
            let palette = ThemeMode::System.resolve(system_dark);
            self.apply_palette(palette);
            self.sync_theme();
            platform::redraw(self.hwnd);
        }
    }

    fn min_client_size(&self) -> (i32, i32) {
        self.preferred_client_size(self.mode)
    }

    fn control_colors(&self) -> (u32, u32) {
        (self.palette.control_bg, self.palette.control_text)
    }

    fn measure_combo_item(&mut self, item: &mut MEASUREITEMSTRUCT) -> bool {
        item.itemHeight = self.shared.s(crate::ui::metrics::COMBO_ITEM_H).max(1) as u32;
        true
    }

    fn draw_combo_item(&mut self, item: &mut DRAWITEMSTRUCT) -> bool {
        let is_field = item.itemID == u32::MAX;
        let text = if is_field {
            let index = platform::combo_selected(item.hwndItem);
            platform::combo_item_text(item.hwndItem, index)
        } else {
            platform::combo_item_text(item.hwndItem, item.itemID as usize)
        };
        let selected = !is_field && item.itemState & ODS_SELECTED != 0;
        let (bg, fg) = if selected {
            (self.palette.selection_bg, self.palette.selection_text)
        } else {
            (self.palette.control_bg, self.palette.control_text)
        };
        platform::fill_rect(item.hDC, item.rcItem, bg);
        let pad = self.shared.s(crate::ui::metrics::PAD_PANEL);
        let arrow = if is_field {
            platform::combo_arrow_width(item.hwndItem)
        } else {
            0
        };
        let mut rect = item.rcItem;
        rect.left += pad;
        rect.right -= arrow + pad;
        platform::draw_text(
            item.hDC,
            &mut rect,
            &text,
            self.fonts.body,
            fg,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        true
    }

    fn control_brush(&self) -> HBRUSH {
        self.control_brush
    }

    fn on_destroy(&mut self) {
        platform::kill_timer(self.hwnd, 1);
        platform::close_menus();
        self.manager.shutdown(&mut self.shared);
        platform::delete_brush(self.control_brush);
        self.control_brush = std::ptr::null_mut();
    }
}

fn menu_title_selected(
    index: usize,
    hot: Option<usize>,
    pressed: Option<usize>,
    last: Option<usize>,
    open: bool,
) -> bool {
    hot == Some(index) || pressed == Some(index) || (open && last == Some(index))
}

fn dismiss_click_switches(last: Option<usize>, target: Option<usize>) -> bool {
    matches!(target, Some(index) if last != Some(index))
}
