use std::collections::BTreeMap;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_F2};
use windows_sys::Win32::UI::WindowsAndMessaging::{CBN_SELCHANGE, EN_CHANGE};

use super::{Shared, View};
use crate::actions::{Action, KeyStyle};
use crate::coordinates::{catalogue, Crs, Error, LocalPoint, Wgs84};
use crate::engine::Mode;
use crate::i18n::coordinates as strings;
use crate::platform;
use crate::ui::{metrics, Rect, Stack, Ui};

const ID_BASE: u16 = 500;
const REGION: usize = 0;
const TEMPLATE: usize = 1;
const DEFINITION: usize = 2;
const LATITUDE: usize = 3;
const LONGITUDE: usize = 4;
const HEIGHT: usize = 5;
const NORTHING: usize = 6;
const EASTING: usize = 7;
const INFO: usize = 8;

struct Layout {
    title: Rect,
    info_button: Rect,
    choice_labels: [Rect; 2],
    definition_label: Rect,
    reset: Rect,
    source: Rect,
    input_labels: [Rect; 3],
    calculate: Rect,
    error: Rect,
    result_labels: [Rect; 2],
    accuracy: Rect,
    session: Rect,
    controls: [Rect; 9],
}

fn columns<const N: usize>(rect: Rect, gap: i32) -> [Rect; N] {
    let usable = rect.w - gap * (N as i32 - 1);
    std::array::from_fn(|i| {
        let left = usable * i as i32 / N as i32;
        let right = usable * (i + 1) as i32 / N as i32;
        Rect::new(rect.x + left + gap * i as i32, rect.y, right - left, rect.h)
    })
}

fn layout(shared: &Shared) -> Layout {
    let s = |dp| shared.s(dp);
    let m = s(metrics::MARGIN);
    let content = Rect::new(
        shared.client.x + m,
        shared.client.y + m,
        shared.client.w - m * 2,
        shared.client.h - m * 2,
    );
    let mut stack = Stack::new(content, 0);
    let title_row = stack.take(s(metrics::FORM_TITLE_H));
    let info_button = Rect::new(
        content.right() - s(metrics::COORDINATE_TOOL_W),
        title_row.y,
        s(metrics::COORDINATE_TOOL_W),
        title_row.h,
    );
    let title = Rect::new(
        content.x,
        title_row.y,
        info_button.x - content.x - s(metrics::GAP_FORM),
        title_row.h,
    );
    stack.advance(s(metrics::GAP_SECTION));
    let details_top = stack.take(0).y;

    let choice_labels = columns(
        stack.take(s(metrics::FIELD_LABEL_H)),
        s(metrics::GAP_SECTION),
    );
    stack.advance(s(metrics::GAP_HAIRLINE));
    let choice = columns::<2>(stack.take(s(metrics::FIELD_H)), s(metrics::GAP_SECTION));
    stack.advance(s(metrics::GAP_SECTION));

    let definition_row = stack.take(s(metrics::FIELD_H));
    let reset = Rect::new(
        content.right() - s(metrics::COORDINATE_TOOL_W),
        definition_row.y,
        s(metrics::COORDINATE_TOOL_W),
        definition_row.h,
    );
    let definition_label = Rect::new(
        content.x,
        definition_row.y,
        reset.x - content.x - s(metrics::GAP_FORM),
        definition_row.h,
    );
    stack.advance(s(metrics::GAP_HAIRLINE));
    let definition = stack.take(s(metrics::COORDINATE_EDITOR_H));
    stack.advance(s(metrics::GAP_HAIRLINE));
    let source = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_SECTION));

    let input_labels = columns(
        stack.take(s(metrics::FIELD_LABEL_H)),
        s(metrics::GAP_SECTION),
    );
    stack.advance(s(metrics::GAP_HAIRLINE));
    let inputs = columns::<3>(stack.take(s(metrics::FIELD_H)), s(metrics::GAP_SECTION));
    stack.advance(s(metrics::GAP_SECTION));
    let action = stack.take(s(metrics::FORM_ACTION_H));
    let calculate = Rect::new(action.x, action.y, s(metrics::FORM_ACTION_W), action.h);
    stack.advance(s(metrics::GAP_TIGHT));
    let error = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_SECTION));
    let result_labels = columns(
        stack.take(s(metrics::FIELD_LABEL_H)),
        s(metrics::GAP_SECTION),
    );
    stack.advance(s(metrics::GAP_HAIRLINE));
    let results = columns::<2>(stack.take(s(metrics::FIELD_H)), s(metrics::GAP_SECTION));
    stack.advance(s(metrics::GAP_SECTION));
    let accuracy = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_HAIRLINE));
    let session = stack.take(s(metrics::FIELD_LABEL_H));
    let details = Rect::new(
        content.x,
        details_top,
        content.w,
        content.bottom() - details_top,
    );
    Layout {
        title,
        info_button,
        choice_labels,
        definition_label,
        reset,
        source,
        input_labels,
        calculate,
        error,
        result_labels,
        accuracy,
        session,
        controls: [
            choice[0], choice[1], definition, inputs[0], inputs[1], inputs[2], results[0],
            results[1], details,
        ],
    }
}

pub struct CoordinateView {
    region: usize,
    template: usize, // 0 = custom; otherwise index + 1 in the region catalogue.
    definition: String,
    values: [String; 3],
    drafts: BTreeMap<String, String>,
    selections: BTreeMap<&'static str, usize>,
    result: Option<LocalPoint>,
    error: Option<Error>,
    show_info: bool,
    controls: [HWND; 9],
}

impl CoordinateView {
    pub fn new() -> Self {
        let region = strings::REGIONS.iter().position(|r| r.id == "50").unwrap();
        let templates = catalogue::for_region("50");
        Self {
            region,
            template: 1,
            definition: templates[0].definition.into(),
            values: [String::new(), String::new(), "0".into()],
            drafts: BTreeMap::new(),
            selections: BTreeMap::new(),
            result: None,
            error: None,
            show_info: false,
            controls: [std::ptr::null_mut(); 9],
        }
    }

    fn templates(&self) -> Vec<catalogue::Template> {
        catalogue::for_region(strings::REGIONS[self.region].id)
    }

    fn selected_template(&self) -> Option<catalogue::Template> {
        self.template
            .checked_sub(1)
            .and_then(|i| self.templates().get(i).copied())
    }

    fn draft_key(&self) -> String {
        self.selected_template()
            .map(|t| t.id.into())
            .unwrap_or_else(|| format!("custom:{}", strings::REGIONS[self.region].id))
    }

    fn remember(&mut self) {
        self.drafts
            .insert(self.draft_key(), self.definition.clone());
        self.selections
            .insert(strings::REGIONS[self.region].id, self.template);
    }

    fn load_definition(&mut self) {
        self.definition = self
            .drafts
            .get(&self.draft_key())
            .cloned()
            .unwrap_or_else(|| {
                self.selected_template()
                    .map(|t| t.definition.into())
                    .unwrap_or_default()
            });
        platform::set_text(self.controls[DEFINITION], &self.definition);
        self.clear_result();
    }

    fn select_region(&mut self, index: usize) {
        self.remember();
        self.region = index.min(strings::REGIONS.len() - 1);
        let count = self.templates().len();
        self.template = self
            .selections
            .get(strings::REGIONS[self.region].id)
            .copied()
            .unwrap_or(usize::from(count > 0))
            .min(count);
        self.load_definition();
    }

    fn select_template(&mut self, index: usize) {
        self.remember();
        self.template = index.min(self.templates().len());
        self.load_definition();
    }

    fn sync_inputs(&mut self) {
        if !self.controls[DEFINITION].is_null() {
            self.definition = platform::get_text(self.controls[DEFINITION]);
        }
        for (i, field) in [LATITUDE, LONGITUDE, HEIGHT].into_iter().enumerate() {
            if !self.controls[field].is_null() {
                self.values[i] = platform::get_text(self.controls[field]);
            }
        }
    }

    fn clear_result(&mut self) {
        self.result = None;
        self.error = None;
        for field in [NORTHING, EASTING] {
            platform::set_text(self.controls[field], "");
        }
    }

    fn calculate(&mut self) {
        if self.show_info {
            return;
        }
        self.sync_inputs();
        self.clear_result();
        let converted = Crs::parse(&self.definition).and_then(|crs| {
            let point = Wgs84::parse(&self.values[0], &self.values[1], &self.values[2])?;
            crs.convert(point)
        });
        match converted {
            Ok(point) => {
                self.result = Some(point);
                platform::set_text(self.controls[NORTHING], &format!("{:.3}", point.northing_m));
                platform::set_text(self.controls[EASTING], &format!("{:.3}", point.easting_m));
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn info_text(shared: &Shared) -> String {
        let text = strings::text(shared.lang);
        format!(
            "{}{}\r\n{}: {}\r\n\r\n{}",
            text.help,
            catalogue::SOURCE,
            text.revision,
            catalogue::REVISION,
            catalogue::LICENSE.replace('\n', "\r\n")
        )
    }

    fn create_controls(&mut self, shared: &Shared) {
        self.destroy_controls();
        if shared.client.w <= 0 {
            return;
        }
        let l = layout(shared);
        let t = strings::text(shared.lang);
        let regions: Vec<_> = strings::REGIONS
            .iter()
            .map(|r| r.name(shared.lang))
            .collect();
        let mut names = vec![t.custom.to_string()];
        names.extend(
            self.templates()
                .iter()
                .map(|t| strings::template_name(shared.lang, t.name)),
        );
        let options: Vec<_> = names.iter().map(String::as_str).collect();
        self.controls[REGION] = shared.combo(ID_BASE, l.controls[REGION], &regions, self.region);
        self.controls[TEMPLATE] =
            shared.combo(ID_BASE + 1, l.controls[TEMPLATE], &options, self.template);
        self.controls[DEFINITION] =
            shared.text_area(ID_BASE + 2, l.controls[DEFINITION], &self.definition, false);
        for (i, field) in [LATITUDE, LONGITUDE, HEIGHT].into_iter().enumerate() {
            self.controls[field] =
                shared.edit(ID_BASE + field as u16, l.controls[field], &self.values[i]);
        }
        for (field, value) in [
            (
                NORTHING,
                self.result
                    .map(|p| format!("{:.3}", p.northing_m))
                    .unwrap_or_default(),
            ),
            (
                EASTING,
                self.result
                    .map(|p| format!("{:.3}", p.easting_m))
                    .unwrap_or_default(),
            ),
        ] {
            self.controls[field] = shared.edit(ID_BASE + field as u16, l.controls[field], &value);
            platform::set_read_only(self.controls[field], true);
        }
        self.controls[INFO] = shared.text_area(
            ID_BASE + INFO as u16,
            l.controls[INFO],
            &Self::info_text(shared),
            true,
        );
        self.update_visibility();
        platform::set_focus(self.controls[if self.show_info { INFO } else { LATITUDE }]);
    }

    fn destroy_controls(&mut self) {
        for control in std::mem::replace(&mut self.controls, [std::ptr::null_mut(); 9]) {
            platform::destroy_control(control);
        }
    }

    fn update_visibility(&self) {
        for (index, control) in self.controls.iter().enumerate() {
            platform::show_control(*control, (index == INFO) == self.show_info);
        }
    }

    fn toggle_info(&mut self) {
        self.show_info = !self.show_info;
        self.update_visibility();
        platform::set_focus(self.controls[if self.show_info { INFO } else { LATITUDE }]);
    }

    fn source_note(&self, shared: &Shared) -> String {
        let t = strings::text(shared.lang);
        if let Some(template) = self.selected_template() {
            let status = if self.definition.trim() == template.definition {
                t.original
            } else {
                t.modified
            };
            format!("{}: geo-ts/msk · {} · {status}", t.source, template.id)
        } else if self.templates().is_empty() && self.definition.is_empty() {
            t.no_templates.into()
        } else {
            t.custom_status.into()
        }
    }
}

impl View for CoordinateView {
    fn mode(&self) -> Mode {
        Mode::Coordinates
    }

    fn on_enter(&mut self, shared: &mut Shared) {
        self.create_controls(shared);
    }

    fn on_exit(&mut self, _shared: &mut Shared) {
        self.sync_inputs();
        self.remember();
        self.destroy_controls();
    }

    fn on_resize(&mut self, shared: &mut Shared) {
        if self.controls[REGION].is_null() {
            return;
        }
        for (index, rect) in layout(shared).controls.into_iter().enumerate() {
            let height = rect.h
                + if index <= TEMPLATE {
                    shared.s(metrics::COMBO_LIST_H)
                } else {
                    0
                };
            platform::move_control(self.controls[index], rect.x, rect.y, rect.w, height);
        }
    }

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let l = layout(shared);
        let t = strings::text(shared.lang);
        ui.heading(l.title, t.title);
        ui.button(
            l.info_button,
            if self.show_info { t.back } else { t.info },
            KeyStyle::Normal,
            Action::CoordinateInfo,
        );
        if self.show_info {
            return;
        }
        for (rect, label) in l.choice_labels.into_iter().zip([t.region, t.template]) {
            ui.form_label(rect, label);
        }
        ui.form_label(l.definition_label, t.definition);
        ui.button(l.reset, t.reset, KeyStyle::Normal, Action::CoordinateReset);
        ui.caption(l.source, &self.source_note(shared));
        for (rect, label) in l
            .input_labels
            .into_iter()
            .zip([t.latitude, t.longitude, t.height])
        {
            ui.form_label(rect, label);
        }
        ui.button(
            l.calculate,
            shared.text().calculate,
            KeyStyle::Normal,
            Action::Calculate,
        );
        if let Some(error) = &self.error {
            ui.error(l.error, &strings::error(shared.lang, error));
        }
        for (rect, label) in l.result_labels.into_iter().zip([t.northing, t.easting]) {
            ui.form_label(rect, label);
        }
        ui.caption(l.accuracy, t.accuracy);
        ui.caption(l.session, t.session);
    }

    fn action(&mut self, action: Action, _shared: &mut Shared) {
        match action {
            Action::Calculate => self.calculate(),
            Action::CoordinateInfo => self.toggle_info(),
            Action::CoordinateReset if !self.show_info => {
                self.definition = self
                    .selected_template()
                    .map(|t| t.definition.into())
                    .unwrap_or_default();
                platform::set_text(self.controls[DEFINITION], &self.definition);
                self.remember();
                self.clear_result();
            }
            _ => {}
        }
    }

    fn command(&mut self, id: u16, code: u16, shared: &mut Shared) -> bool {
        if !(ID_BASE..ID_BASE + 9).contains(&id) {
            return false;
        }
        let index = (id - ID_BASE) as usize;
        if u32::from(code) == EN_CHANGE && (DEFINITION..=HEIGHT).contains(&index) {
            self.sync_inputs();
            self.clear_result();
            return true;
        }
        if u32::from(code) == CBN_SELCHANGE {
            self.sync_inputs();
            if index == REGION {
                self.select_region(platform::combo_selected(self.controls[REGION]));
                let combo = self.controls[TEMPLATE];
                platform::combo_reset(combo);
                platform::combo_add(combo, strings::text(shared.lang).custom);
                for template in self.templates() {
                    platform::combo_add(combo, &strings::template_name(shared.lang, template.name));
                }
                platform::combo_select(combo, self.template);
                return true;
            }
            if index == TEMPLATE {
                self.select_template(platform::combo_selected(self.controls[TEMPLATE]));
                return true;
            }
        }
        false
    }

    fn key(&mut self, vk: u16, ctrl: bool, _shared: &mut Shared) -> bool {
        if !ctrl && (vk == VK_F2 || (vk == VK_ESCAPE && self.show_info)) {
            self.toggle_info();
            return true;
        }
        false
    }

    fn display(&self, shared: &Shared) -> String {
        let text = strings::text(shared.lang);
        self.result
            .map(|p| {
                format!(
                    "{}: {:.3}\r\n{}: {:.3}",
                    text.northing, p.northing_m, text.easting, p.easting_m
                )
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::ui::Fonts;

    #[test]
    fn all_catalogue_regions_exist_but_missing_definitions_are_not_fabricated() {
        assert_eq!(strings::REGIONS.len(), 89);
        let ids: std::collections::BTreeSet<_> = strings::REGIONS.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), 89);
        assert!(catalogue::all().all(|t| ids.contains(t.region)));
        let mut view = CoordinateView::new();
        view.select_region(strings::REGIONS.iter().position(|r| r.id == "17").unwrap());
        assert_eq!(view.template, 0);
        assert!(view.definition.is_empty());
        view.calculate();
        assert_eq!(view.error, Some(Error::EmptyDefinition));
    }

    #[test]
    fn each_template_and_custom_region_retains_its_own_unsaved_draft() {
        let mut view = CoordinateView::new();
        let region = view.region;
        view.definition = "original draft".into();
        view.select_template(0);
        assert!(view.definition.is_empty());
        view.definition = "custom draft".into();
        view.select_region(0);
        view.select_region(region);
        assert_eq!(view.template, 0);
        assert_eq!(view.definition, "custom draft");
        view.select_template(1);
        assert_eq!(view.definition, "original draft");
    }

    #[test]
    fn native_fields_results_license_and_unsaved_edits_survive_recreation() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        // Hidden, test-owned parent; no real clipboard or global input events.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                windows_sys::w!("STATIC"),
                windows_sys::w!("Coordinate lifecycle test"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                1000,
                800,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        struct Parent(HWND);
        impl Drop for Parent {
            fn drop(&mut self) {
                platform::destroy_window(self.0);
            }
        }
        let _parent = Parent(hwnd);
        let mut shared = Shared::new();
        let mut fonts = Fonts::new(96);
        shared.attach(hwnd, 96, fonts.body);
        shared.client = Rect::new(
            0,
            0,
            shared.s(metrics::COORDINATE_CONTENT_W),
            shared.s(metrics::COORDINATE_CONTENT_H),
        );
        let mut view = CoordinateView::new();
        view.on_enter(&mut shared);
        assert!(view.controls.iter().all(|c| !c.is_null()));
        assert_eq!(
            unsafe { SendMessageW(view.controls[REGION], CB_GETCOUNT, 0, 0) },
            89
        );
        platform::set_text(view.controls[LATITUDE], "55,7558");
        platform::set_text(view.controls[LONGITUDE], "37.6173");
        platform::set_text(view.controls[HEIGHT], "150");
        view.command(ID_BASE + LATITUDE as u16, EN_CHANGE as u16, &mut shared);
        view.action(Action::Calculate, &mut shared);
        assert!(view.result.is_some());
        assert!(!platform::get_text(view.controls[NORTHING]).is_empty());
        let result = platform::get_text(view.controls[NORTHING]);
        unsafe {
            SendMessageW(view.controls[NORTHING], WM_CHAR, '9' as usize, 0);
        }
        assert_eq!(
            platform::get_text(view.controls[NORTHING]),
            result,
            "results must remain read-only"
        );

        let edited = format!("{}\r\n+nadgrids=unsupported.gsb", view.definition);
        platform::set_text(view.controls[DEFINITION], &edited);
        view.command(ID_BASE + DEFINITION as u16, EN_CHANGE as u16, &mut shared);
        assert!(view.result.is_none());
        assert!(platform::get_text(view.controls[NORTHING]).is_empty());
        view.calculate();
        assert!(matches!(view.error, Some(Error::Unsupported(_))));

        for (dpi, lang) in [
            (144, Lang::En),
            (192, Lang::Ru),
            (72, Lang::En),
            (96, Lang::Ru),
        ] {
            view.on_exit(&mut shared);
            assert!(view.controls.iter().all(|c| c.is_null()));
            fonts = Fonts::new(dpi);
            shared.attach(hwnd, dpi, fonts.body);
            shared.lang = lang;
            shared.client = Rect::new(
                0,
                0,
                shared.s(metrics::COORDINATE_CONTENT_W),
                shared.s(metrics::COORDINATE_CONTENT_H),
            );
            view.on_enter(&mut shared);
            assert_eq!(platform::get_text(view.controls[DEFINITION]), edited);
            assert_eq!(platform::get_text(view.controls[LATITUDE]), "55,7558");
            assert_eq!(platform::get_text(view.controls[HEIGHT]), "150");
            let help = platform::get_text(view.controls[INFO]);
            assert!(help.contains(catalogue::REVISION));
            assert!(help.contains("END OF TERMS AND CONDITIONS"));
            assert!(
                help.ends_with(&catalogue::LICENSE.replace('\n', "\r\n")),
                "full license must be accessible without truncation"
            );
        }
        view.key(VK_F2, false, &mut shared);
        assert!(view.show_info);
        view.key(VK_ESCAPE, false, &mut shared);
        assert!(!view.show_info);
        view.action(Action::CoordinateReset, &mut shared);
        assert!(!view.definition.contains("nadgrids"));
        assert!(view.error.is_none());
        view.on_exit(&mut shared);
    }

    #[test]
    fn coordinate_layout_and_actual_localized_labels_fit_all_dpi_scales() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let fonts = Fonts::new(dpi);
            let mut shared = Shared::new();
            shared.attach(std::ptr::null_mut(), dpi, fonts.body);
            shared.client = Rect::new(
                0,
                shared.s(32.0),
                shared.s(metrics::COORDINATE_CONTENT_W),
                shared.s(metrics::COORDINATE_CONTENT_H),
            );
            let l = layout(&shared);
            for pair in [
                l.controls[0..2].to_vec(),
                l.controls[3..6].to_vec(),
                l.controls[6..8].to_vec(),
            ] {
                assert!(pair.windows(2).all(|r| r[0].right() < r[1].x));
                assert_eq!(
                    pair.last().unwrap().right(),
                    shared.client.right() - shared.s(metrics::MARGIN)
                );
            }
            assert!(l.controls[DEFINITION].y > l.reset.bottom());
            assert!(l.controls[HEIGHT].bottom() < l.calculate.y);
            assert!(l.error.bottom() < l.result_labels[0].y);
            assert!(
                l.session.bottom() <= shared.client.bottom() - shared.s(metrics::MARGIN),
                "DPI {dpi}: {:?}",
                l.session
            );
            for lang in [Lang::Ru, Lang::En] {
                let t = strings::text(lang);
                for (rect, label, font) in [
                    (l.input_labels[0], t.latitude, fonts.body),
                    (l.input_labels[1], t.longitude, fonts.body),
                    (l.input_labels[2], t.height, fonts.body),
                    (l.accuracy, t.accuracy, fonts.small),
                    (l.session, t.session, fonts.small),
                ] {
                    assert!(
                        platform::text_width(std::ptr::null_mut(), font, label) <= rect.w,
                        "{dpi}: {label}"
                    );
                }
                for region in strings::REGIONS {
                    assert!(
                        platform::text_width(std::ptr::null_mut(), fonts.body, region.name(lang))
                            + shared.s(metrics::PAD_PANEL * 2.0 + 24.0)
                            <= l.controls[REGION].w,
                        "{dpi}: {}",
                        region.name(lang)
                    );
                }
            }
        }
    }
}
