use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{CBN_SELCHANGE, EN_CHANGE};

use crate::actions::Action;
use crate::engine::{format_number, Mode};
use crate::i18n;
use crate::platform;
use crate::ui::{metrics, Rect, Stack, TextAlign, Ui};
use crate::units::{self, Category, CategoryId};
use rustic_calc_app::unit_session::{ConversionResult, UnitField, UnitIndex, UnitSession};

use super::common::sidebar;
use super::{Shared, View};

const ID_CATEGORY: u16 = 200;
const ID_FROM: u16 = 201;
const ID_TO: u16 = 202;
const ID_INPUT: u16 = 203;
const ID_LOOP_VOLTAGE: u16 = 204;
const ID_LOOP_CURRENT: u16 = 205;

struct Layout {
    title: Rect,
    category: Rect,
    from_label: Rect,
    input: Rect,
    from_combo: Rect,
    to_label: Rect,
    result: Rect,
    to_combo: Rect,
}

fn layout(shared: &Shared) -> Layout {
    let s = |v: f32| shared.s(v);
    let client = shared.client;
    let margin = s(metrics::MARGIN);
    let content_x = client.x + margin + s(metrics::SIDEBAR_W) + margin;
    let content = Rect::new(
        content_x,
        client.y + margin,
        client.right() - margin - content_x,
        client.h - margin * 2,
    );
    let mut stack = Stack::new(content, 0);
    let title = stack.take(s(metrics::FORM_TITLE_H));
    stack.advance(s(metrics::GAP));
    let category = stack.take(s(metrics::FIELD_H));
    stack.advance(s(metrics::GAP_SECTION));
    let from_label = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_HAIRLINE));
    let input = stack.take(s(metrics::FIELD_H));
    stack.advance(s(metrics::GAP));
    let from_combo = stack.take(s(metrics::FIELD_H));

    let to_y = from_combo.bottom() + s(metrics::GAP_BLOCK);
    let to_label = Rect::new(
        content.x,
        to_y,
        s(metrics::FORM_COL_W),
        s(metrics::FIELD_LABEL_H),
    );
    let result = Rect::new(
        content.x,
        to_y + s(metrics::FIELD_LABEL_H + metrics::GAP_HAIRLINE),
        content.w,
        s(metrics::RESULT_H),
    );
    let to_combo = Rect::new(
        content.x,
        result.bottom() + s(metrics::GAP),
        content.w,
        s(metrics::FIELD_H),
    );
    Layout {
        title,
        category,
        from_label,
        input,
        from_combo,
        to_label,
        result,
        to_combo,
    }
}

#[derive(Clone, Copy)]
struct LoopLayout {
    panel: Rect,
    title: Rect,
    voltage_label: Rect,
    voltage_edit: Rect,
    current_label: Rect,
    current_edit: Rect,
    resistance: Rect,
    power: Rect,
    standard: Rect,
}

fn loop_layout(shared: &Shared, l: &Layout) -> LoopLayout {
    let s = |v: f32| shared.s(v);
    let pad = s(metrics::PAD_PANEL);
    // The optional helper follows the entire conversion group; it must not
    // separate the input from its result or overlap either combo box.
    let top = l.to_combo.bottom() + s(metrics::GAP_SECTION);
    let panel = Rect::new(l.from_combo.x, top, l.from_combo.w, 0);
    let inner_x = panel.x + pad;
    let inner_w = (panel.w - pad * 2).max(0);
    let title = Rect::new(inner_x, panel.y + pad, inner_w, s(metrics::FIELD_LABEL_H));
    let label_y = title.bottom() + s(metrics::GAP);
    let column_gap = s(metrics::GAP_FORM);
    let column_w = (inner_w - column_gap) / 2;
    let voltage_label = Rect::new(inner_x, label_y, column_w, s(metrics::FIELD_LABEL_H));
    let current_label = Rect::new(
        inner_x + column_w + column_gap,
        label_y,
        column_w,
        s(metrics::FIELD_LABEL_H),
    );
    let edit_y = voltage_label.bottom() + s(metrics::GAP_HAIRLINE);
    let voltage_edit = Rect::new(inner_x, edit_y, column_w, s(metrics::FIELD_H));
    let current_edit = Rect::new(
        inner_x + column_w + column_gap,
        edit_y,
        column_w,
        s(metrics::FIELD_H),
    );
    let resistance = Rect::new(
        inner_x,
        voltage_edit.bottom() + s(metrics::GAP_BLOCK),
        inner_w,
        s(metrics::SELECTOR_ITEM_H),
    );
    let power = Rect::new(
        inner_x,
        resistance.bottom() + s(metrics::GAP_HAIRLINE),
        inner_w,
        s(metrics::FIELD_LABEL_H),
    );
    let standard = Rect::new(
        inner_x,
        power.bottom() + s(metrics::GAP_HAIRLINE),
        inner_w,
        s(metrics::FIELD_LABEL_H),
    );
    let panel = Rect::new(panel.x, panel.y, panel.w, standard.bottom() + pad - panel.y);
    LoopLayout {
        panel,
        title,
        voltage_label,
        voltage_edit,
        current_label,
        current_edit,
        resistance,
        power,
        standard,
    }
}

pub struct UnitView {
    session: UnitSession,
    category_combo: HWND,
    from_combo: HWND,
    to_combo: HWND,
    input_edit: HWND,
    voltage_edit: HWND,
    current_edit: HWND,
}

impl UnitView {
    pub fn new() -> Self {
        Self {
            session: UnitSession::new(),
            category_combo: std::ptr::null_mut(),
            from_combo: std::ptr::null_mut(),
            to_combo: std::ptr::null_mut(),
            input_edit: std::ptr::null_mut(),
            voltage_edit: std::ptr::null_mut(),
            current_edit: std::ptr::null_mut(),
        }
    }

    fn category(&self) -> &Category {
        self.session.category()
    }

    fn is_dcs(&self) -> bool {
        self.session.category_id() == CategoryId::DcsSignals
    }

    fn sync_loop_visibility(&self) {
        let show = self.is_dcs();
        platform::show_control(self.voltage_edit, show);
        platform::show_control(self.current_edit, show);
    }

    fn create_controls(&mut self, shared: &mut Shared) {
        self.destroy_controls();
        if shared.client.w <= 0 {
            return;
        }
        let l = layout(shared);
        let ll = loop_layout(shared, &l);
        let names: Vec<&str> = self
            .session
            .categories()
            .iter()
            .map(|c| i18n::category(shared.lang, c.name))
            .collect();
        self.category_combo = shared.combo(
            ID_CATEGORY,
            l.category,
            &names,
            self.session.category_index(),
        );
        let unit_count = self.category().units.len();
        let cat = self.category();
        let from_items: Vec<&str> = cat
            .units
            .iter()
            .map(|u| i18n::unit(shared.lang, u.name))
            .collect();
        self.from_combo = shared.combo(
            ID_FROM,
            l.from_combo,
            &from_items,
            self.session.from_index().get().min(unit_count - 1),
        );
        self.to_combo = shared.combo(
            ID_TO,
            l.to_combo,
            &from_items,
            self.session.to_index().get().min(unit_count - 1),
        );
        self.input_edit = shared.edit(ID_INPUT, l.input, self.session.text(UnitField::Value));
        self.voltage_edit = shared.edit(
            ID_LOOP_VOLTAGE,
            ll.voltage_edit,
            self.session.text(UnitField::LoopVoltage),
        );
        self.current_edit = shared.edit(
            ID_LOOP_CURRENT,
            ll.current_edit,
            self.session.text(UnitField::LoopCurrentMa),
        );
        self.session.set_text(
            UnitField::LoopVoltage,
            platform::get_text(self.voltage_edit),
        );
        self.session.set_text(
            UnitField::LoopCurrentMa,
            platform::get_text(self.current_edit),
        );
        self.sync_loop_visibility();
        platform::set_focus(self.input_edit);
    }

    fn destroy_controls(&mut self) {
        for control in [
            self.category_combo,
            self.from_combo,
            self.to_combo,
            self.input_edit,
            self.voltage_edit,
            self.current_edit,
        ] {
            platform::destroy_control(control);
        }
        self.category_combo = std::ptr::null_mut();
        self.from_combo = std::ptr::null_mut();
        self.to_combo = std::ptr::null_mut();
        self.input_edit = std::ptr::null_mut();
        self.voltage_edit = std::ptr::null_mut();
        self.current_edit = std::ptr::null_mut();
    }

    fn refill_units(&self, shared: &Shared) {
        let cat = self.category();
        let items: Vec<&str> = cat
            .units
            .iter()
            .map(|u| i18n::unit(shared.lang, u.name))
            .collect();
        for combo in [self.from_combo, self.to_combo] {
            platform::combo_reset(combo);
            for item in &items {
                platform::combo_add(combo, item);
            }
        }
        platform::combo_select(self.from_combo, self.session.from_index().get());
        platform::combo_select(self.to_combo, self.session.to_index().get());
    }

    fn move_controls(&self, shared: &Shared) {
        let l = layout(shared);
        let ll = loop_layout(shared, &l);
        let combo_h = shared.s(metrics::COMBO_POPUP_H);
        for (control, rect, h) in [
            (self.category_combo, l.category, combo_h),
            (self.from_combo, l.from_combo, combo_h),
            (self.to_combo, l.to_combo, combo_h),
            (self.input_edit, l.input, l.input.h),
            (self.voltage_edit, ll.voltage_edit, ll.voltage_edit.h),
            (self.current_edit, ll.current_edit, ll.current_edit.h),
        ] {
            if !control.is_null() {
                platform::move_control(control, rect.x, rect.y, rect.w, h);
            }
        }
    }
}

impl View for UnitView {
    fn mode(&self) -> Mode {
        Mode::UnitConversion
    }

    fn on_enter(&mut self, shared: &mut Shared) {
        self.create_controls(shared);
    }

    fn on_exit(&mut self, _shared: &mut Shared) {
        self.destroy_controls();
    }

    fn on_resize(&mut self, shared: &mut Shared) {
        if self.input_edit.is_null() {
            self.create_controls(shared);
        } else {
            self.move_controls(shared);
        }
    }

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let client = shared.client;
        let m = ui.s(metrics::MARGIN);
        sidebar(
            ui,
            Rect::new(
                client.x + m,
                client.y + m,
                ui.s(metrics::SIDEBAR_W),
                client.h - m * 2,
            ),
            &shared.sidebar,
            shared.text(),
            None,
        );
        let l = layout(shared);
        let t = shared.text();
        ui.heading(l.title, t.unit_title);
        ui.form_label(l.from_label, t.from);
        ui.form_label(l.to_label, t.to);
        let conversion = self.session.conversion_result();
        let invalid_input = conversion == ConversionResult::InvalidInput;
        let result = match conversion {
            ConversionResult::Value(value) => format_number(value, shared.grouping),
            ConversionResult::Empty | ConversionResult::InvalidInput => String::new(),
        };
        let result_text = if invalid_input {
            t.error_invalid_input
        } else {
            &result
        };
        ui.result_box(l.result, result_text, invalid_input);

        if self.is_dcs() {
            let ll = loop_layout(shared, &l);
            ui.panel(ll.panel);
            ui.heading(ll.title, t.dcs_loop_title);
            ui.form_label(ll.voltage_label, t.dcs_loop_voltage);
            ui.form_label(ll.current_label, t.dcs_loop_current);
            let (resistance, power, standard) = match self.session.dcs_loop_result() {
                Some(Ok(loop_value)) => (
                    format!(
                        "R = {}",
                        units::format_resistance(loop_value.resistance, shared.grouping)
                    ),
                    format!(
                        "P = {} W  ·  ≥ {} W",
                        format_number(loop_value.power, shared.grouping),
                        format_number(loop_value.recommended_wattage, shared.grouping)
                    ),
                    format!(
                        "E24: {} → {} mA",
                        units::format_resistance(loop_value.standard_resistance, shared.grouping),
                        format_number(loop_value.actual_current_ma, shared.grouping)
                    ),
                ),
                Some(Err(_)) => (
                    "R = —".to_string(),
                    "P = —".to_string(),
                    t.loop_positive_values.to_string(),
                ),
                None => unreachable!("DCS helper is visible only in the DCS category"),
            };
            ui.text(
                ll.resistance,
                &resistance,
                ui.fonts.title,
                ui.palette.text,
                TextAlign::Left,
            );
            ui.text(
                ll.power,
                &power,
                ui.fonts.body,
                ui.palette.muted,
                TextAlign::Left,
            );
            ui.text(
                ll.standard,
                &standard,
                ui.fonts.body,
                ui.palette.muted,
                TextAlign::Left,
            );
        }
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        let current = shared.sidebar.current_value();
        shared.sidebar_action(action, current);
    }

    fn command(&mut self, id: u16, code: u16, shared: &mut Shared) -> bool {
        match id {
            ID_CATEGORY if u32::from(code) == CBN_SELCHANGE => {
                if let Some(category) = self
                    .session
                    .categories()
                    .get(platform::combo_selected(self.category_combo))
                    .map(|category| category.id)
                {
                    let _ = self.session.select_category(category);
                }
                self.refill_units(shared);
                self.sync_loop_visibility();
                true
            }
            ID_FROM if u32::from(code) == CBN_SELCHANGE => {
                let _ = self
                    .session
                    .select_from(UnitIndex::new(platform::combo_selected(self.from_combo)));
                true
            }
            ID_TO if u32::from(code) == CBN_SELCHANGE => {
                let _ = self
                    .session
                    .select_to(UnitIndex::new(platform::combo_selected(self.to_combo)));
                true
            }
            ID_INPUT if !self.input_edit.is_null() && u32::from(code) == EN_CHANGE => {
                self.session
                    .set_text(UnitField::Value, platform::get_text(self.input_edit));
                true
            }
            ID_LOOP_VOLTAGE if !self.voltage_edit.is_null() && u32::from(code) == EN_CHANGE => {
                self.session.set_text(
                    UnitField::LoopVoltage,
                    platform::get_text(self.voltage_edit),
                );
                true
            }
            ID_LOOP_CURRENT if !self.current_edit.is_null() && u32::from(code) == EN_CHANGE => {
                self.session.set_text(
                    UnitField::LoopCurrentMa,
                    platform::get_text(self.current_edit),
                );
                true
            }
            _ => false,
        }
    }

    fn command_rect(&self, id: u16, shared: &Shared) -> Option<Rect> {
        let l = layout(shared);
        match id {
            ID_INPUT | ID_FROM | ID_TO => Some(l.result),
            ID_LOOP_VOLTAGE | ID_LOOP_CURRENT if self.is_dcs() => {
                let ll = loop_layout(shared, &l);
                Some(Rect::new(
                    ll.panel.x,
                    ll.resistance.y,
                    ll.panel.w,
                    ll.standard.bottom() - ll.resistance.y,
                ))
            }
            _ => None,
        }
    }

    fn display(&self, _shared: &Shared) -> String {
        match self.session.conversion_result() {
            ConversionResult::Value(value) => format_number(value, _shared.grouping),
            ConversionResult::Empty | ConversionResult::InvalidInput => String::new(),
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn category_identity_does_not_depend_on_name_or_position() {
        let mut view = UnitView::new();
        assert_eq!(view.category().id, CategoryId::Angle);
        let dcs = view
            .session
            .categories()
            .iter()
            .position(|c| c.id == CategoryId::DcsSignals)
            .unwrap();
        let dcs_id = view.session.categories()[dcs].id;
        view.session.select_category(dcs_id).unwrap();
        assert!(view.is_dcs());
    }

    #[test]
    fn conversion_and_dcs_helper_are_compact_and_disjoint_at_all_scales() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let mut shared = Shared::new();
            shared.attach(std::ptr::null_mut(), dpi, std::ptr::null_mut());
            shared.client = Rect::new(0, 0, shared.s(901.0), shared.s(480.0));
            let l = layout(&shared);
            let dcs = loop_layout(&shared, &l);
            assert_eq!(
                l.to_label.y - l.from_combo.bottom(),
                shared.s(metrics::GAP_BLOCK)
            );
            assert!(dcs.panel.y >= l.to_combo.bottom());
            assert!(dcs.standard.bottom() <= dcs.panel.bottom());
            assert_eq!(
                dcs.panel.bottom() - dcs.standard.bottom(),
                shared.s(metrics::PAD_PANEL)
            );
            assert!(dcs.panel.bottom() <= shared.client.bottom() - shared.s(metrics::MARGIN));
        }
    }
}
