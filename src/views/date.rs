use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{CBN_SELCHANGE, EN_CHANGE};

use crate::actions::{Action, KeyStyle};
use crate::engine::Mode;
use crate::i18n;
use crate::platform;
use crate::ui::{metrics, Rect, Stack, Ui};
use crate::units;

use super::common::sidebar;
use super::{Shared, View};

const GAP_ROW: f32 = 18.0;
const FIELD_W: f32 = 70.0;
const FIELD_STEP: f32 = 82.0;
const ERROR_H: f32 = 20.0;

const ID_MODE: u16 = 300;
const ID_FROM: u16 = 301;
const ID_TO: u16 = 302;
const ID_OP: u16 = 303;
const ID_YEARS: u16 = 304;
const ID_MONTHS: u16 = 305;
const ID_DAYS: u16 = 306;

struct Layout {
    title: Rect,
    mode: Rect,
    from_label: Rect,
    from_edit: Rect,
    to_label: Rect,
    to_edit: Rect,
    op_label: Rect,
    op_combo: Rect,
    years_label: Rect,
    years_edit: Rect,
    months_label: Rect,
    months_edit: Rect,
    days_label: Rect,
    days_edit: Rect,
    result_a_label: Rect,
    result_a: Rect,
    result_b_label: Rect,
    result_b: Rect,
    calculate: Rect,
    error: Rect,
}

fn layout(shared: &Shared, difference: bool) -> Layout {
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
    let mode = stack.take(s(metrics::FIELD_H));
    stack.advance(s(metrics::GAP_SECTION));

    let value_label = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_HAIRLINE));
    let value_edit = stack.take(s(metrics::FIELD_H));
    stack.advance(s(GAP_ROW));

    let (years_label, years_edit) = if difference {
        (Rect::new(content.x, 0, 0, 0), Rect::new(content.x, 0, 0, 0))
    } else {
        let label = stack.take(s(metrics::FIELD_LABEL_H));
        stack.advance(s(metrics::GAP_HAIRLINE));
        let edit = stack.take(s(metrics::FIELD_H));
        stack.advance(s(GAP_ROW));
        (label, edit)
    };

    let action_row = stack.take(s(metrics::FORM_ACTION_H));
    let calculate = Rect::new(
        content.x,
        action_row.y,
        s(metrics::FORM_ACTION_W),
        action_row.h,
    );
    let error = Rect::new(
        calculate.right() + s(metrics::GAP_FORM),
        action_row.y,
        content.w - calculate.w - s(metrics::GAP_FORM),
        s(ERROR_H),
    );
    stack.advance(s(metrics::GAP_SECTION));

    let (result_a_label, result_a) = if difference {
        let label = stack.take(s(metrics::FIELD_LABEL_H));
        stack.advance(s(metrics::GAP_HAIRLINE));
        let result = stack.take(s(metrics::RESULT_SMALL_H));
        stack.advance(s(metrics::GAP_FORM));
        (label, result)
    } else {
        (Rect::new(content.x, 0, 0, 0), Rect::new(content.x, 0, 0, 0))
    };

    let result_b_label = stack.take(s(metrics::FIELD_LABEL_H));
    stack.advance(s(metrics::GAP_HAIRLINE));
    let result_b = stack.take(s(metrics::RESULT_SMALL_H));

    let col_w = s(metrics::FORM_COL_W);
    let second_x = content.x + col_w + s(metrics::FORM_COL_GAP);
    let field_w = s(FIELD_W);
    let years_x = content.x;
    let months_x = content.x + s(FIELD_STEP);
    let days_x = content.x + s(FIELD_STEP * 2.0);
    Layout {
        title,
        mode,
        from_label: Rect::new(content.x, value_label.y, col_w, value_label.h),
        from_edit: Rect::new(content.x, value_edit.y, col_w, value_edit.h),
        to_label: Rect::new(second_x, value_label.y, col_w, value_label.h),
        to_edit: Rect::new(second_x, value_edit.y, col_w, value_edit.h),
        op_label: Rect::new(second_x, value_label.y, col_w, value_label.h),
        op_combo: Rect::new(second_x, value_edit.y, col_w, value_edit.h),
        years_label: Rect::new(years_x, years_label.y, field_w, years_label.h),
        years_edit: Rect::new(years_x, years_edit.y, field_w, years_edit.h),
        months_label: Rect::new(months_x, years_label.y, field_w, years_label.h),
        months_edit: Rect::new(months_x, years_edit.y, field_w, years_edit.h),
        days_label: Rect::new(days_x, years_label.y, field_w, years_label.h),
        days_edit: Rect::new(days_x, years_edit.y, field_w, years_edit.h),
        result_a_label,
        result_a,
        result_b_label,
        result_b,
        calculate,
        error,
    }
}

pub struct DateView {
    difference: bool,
    from: String,
    to: String,
    add_subtract: bool,
    years: String,
    months: String,
    days: String,
    result_a: String,
    result_b: String,
    error: String,
    mode_combo: HWND,
    from_edit: HWND,
    to_edit: HWND,
    op_combo: HWND,
    years_edit: HWND,
    months_edit: HWND,
    days_edit: HWND,
}

impl DateView {
    pub fn new() -> Self {
        let today = units::today();
        let text = units::format_date(today);
        Self {
            difference: true,
            from: text.clone(),
            to: text,
            add_subtract: true,
            years: "0".to_string(),
            months: "0".to_string(),
            days: "0".to_string(),
            result_a: String::new(),
            result_b: String::new(),
            error: String::new(),
            mode_combo: std::ptr::null_mut(),
            from_edit: std::ptr::null_mut(),
            to_edit: std::ptr::null_mut(),
            op_combo: std::ptr::null_mut(),
            years_edit: std::ptr::null_mut(),
            months_edit: std::ptr::null_mut(),
            days_edit: std::ptr::null_mut(),
        }
    }

    fn create_controls(&mut self, shared: &mut Shared) {
        self.destroy_controls();
        if shared.client.w <= 0 {
            return;
        }
        let l = layout(shared, self.difference);
        let t = shared.text();
        let modes = [t.date_diff_mode, t.date_shift_mode];
        self.mode_combo =
            shared.combo(ID_MODE, l.mode, &modes, if self.difference { 0 } else { 1 });
        self.from_edit = shared.edit(ID_FROM, l.from_edit, &self.from);
        self.to_edit = shared.edit(ID_TO, l.to_edit, &self.to);
        let ops = [t.add, t.subtract];
        self.op_combo = shared.combo(
            ID_OP,
            l.op_combo,
            &ops,
            if self.add_subtract { 0 } else { 1 },
        );
        self.years_edit = shared.edit(ID_YEARS, l.years_edit, &self.years);
        self.months_edit = shared.edit(ID_MONTHS, l.months_edit, &self.months);
        self.days_edit = shared.edit(ID_DAYS, l.days_edit, &self.days);
        self.update_visibility();
        platform::set_focus(self.from_edit);
    }

    fn destroy_controls(&mut self) {
        for c in [
            self.mode_combo,
            self.from_edit,
            self.to_edit,
            self.op_combo,
            self.years_edit,
            self.months_edit,
            self.days_edit,
        ] {
            platform::destroy_control(c);
        }
        self.mode_combo = std::ptr::null_mut();
        self.from_edit = std::ptr::null_mut();
        self.to_edit = std::ptr::null_mut();
        self.op_combo = std::ptr::null_mut();
        self.years_edit = std::ptr::null_mut();
        self.months_edit = std::ptr::null_mut();
        self.days_edit = std::ptr::null_mut();
    }

    fn update_visibility(&self) {
        platform::show_control(self.to_edit, self.difference);
        platform::show_control(self.op_combo, !self.difference);
        for c in [self.years_edit, self.months_edit, self.days_edit] {
            platform::show_control(c, !self.difference);
        }
    }

    fn move_controls(&self, shared: &Shared) {
        let l = layout(shared, self.difference);
        let combo_h = shared.s(metrics::COMBO_POPUP_H);
        for (c, r, h) in [
            (self.mode_combo, l.mode, combo_h),
            (self.from_edit, l.from_edit, l.from_edit.h),
            (self.to_edit, l.to_edit, l.to_edit.h),
            (self.op_combo, l.op_combo, combo_h),
            (self.years_edit, l.years_edit, l.years_edit.h),
            (self.months_edit, l.months_edit, l.months_edit.h),
            (self.days_edit, l.days_edit, l.days_edit.h),
        ] {
            if !c.is_null() {
                platform::move_control(c, r.x, r.y, r.w, h);
            }
        }
    }

    fn calculate(&mut self, shared: &Shared) {
        let t = shared.text();
        self.error.clear();
        self.result_a.clear();
        self.result_b.clear();
        self.from = platform::get_text(self.from_edit);
        self.to = platform::get_text(self.to_edit);
        let from = units::parse_date(&self.from);
        let Some(from) = from else {
            self.error = t.invalid_date.to_string();
            return;
        };
        if self.difference {
            let Some(to) = units::parse_date(&self.to) else {
                self.error = t.invalid_date.to_string();
                return;
            };
            let (years, months, weeks, days, total) = units::date_difference(from, to);
            self.result_a = i18n::date_parts(shared.lang, years, months, weeks, days);
            self.result_b = format!("{} {}", total, t.days_suffix);
        } else {
            self.add_subtract = platform::combo_selected(self.op_combo) == 0;
            self.years = platform::get_text(self.years_edit);
            self.months = platform::get_text(self.months_edit);
            self.days = platform::get_text(self.days_edit);
            let parse_offset = |text: &str| text.trim().parse::<i32>().ok().filter(|v| *v >= 0);
            let offsets = [
                (&self.years, t.years),
                (&self.months, t.months),
                (&self.days, t.days),
            ];
            let mut parsed = [0; 3];
            for (i, (value, label)) in offsets.iter().enumerate() {
                let Some(number) = parse_offset(value) else {
                    let reason = t.field_nonnegative_integer;
                    self.error = format!("{label}: {reason}");
                    return;
                };
                parsed[i] = number;
            }
            let [years, months, days] = parsed;
            match units::shift_date(from, years, months, days, self.add_subtract) {
                Some(date) => self.result_b = units::format_date(date),
                None => self.error = t.date_out_of_bound.to_string(),
            }
        }
    }
}

impl View for DateView {
    fn mode(&self) -> Mode {
        Mode::DateCalculation
    }

    fn on_enter(&mut self, shared: &mut Shared) {
        self.create_controls(shared);
    }

    fn on_exit(&mut self, _shared: &mut Shared) {
        self.destroy_controls();
    }

    fn on_resize(&mut self, shared: &mut Shared) {
        if self.from_edit.is_null() {
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
        let l = layout(shared, self.difference);
        let t = shared.text();
        ui.heading(l.title, t.date_title);
        ui.form_label(l.from_label, t.from);
        if self.difference {
            ui.form_label(l.to_label, t.to);
            ui.form_label(l.result_a_label, t.diff_ymwd);
            ui.result_box(l.result_a, &self.result_a, false);
            ui.form_label(l.result_b_label, t.diff_days);
            ui.result_box(l.result_b, &self.result_b, false);
        } else {
            ui.form_label(l.op_label, t.operation);
            ui.form_label(l.years_label, t.years);
            ui.form_label(l.months_label, t.months);
            ui.form_label(l.days_label, t.days);
            ui.form_label(l.result_b_label, t.result_date);
            ui.result_box(l.result_b, &self.result_b, false);
        }
        ui.button(
            l.calculate,
            t.calculate,
            KeyStyle::Normal,
            Action::Calculate,
        );
        if !self.error.is_empty() {
            ui.error(l.error, &self.error);
        }
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        if matches!(action, Action::Calculate) {
            self.calculate(shared);
            return;
        }
        let current = shared.sidebar.current_value();
        shared.sidebar_action(action, current);
    }

    fn command(&mut self, id: u16, code: u16, shared: &mut Shared) -> bool {
        match id {
            ID_MODE if u32::from(code) == CBN_SELCHANGE => {
                self.difference = platform::combo_selected(self.mode_combo) == 0;
                self.result_a.clear();
                self.result_b.clear();
                self.error.clear();
                self.update_visibility();
                self.move_controls(shared);
                true
            }
            ID_FROM | ID_TO | ID_YEARS | ID_MONTHS | ID_DAYS if u32::from(code) == EN_CHANGE => {
                let (control, value) = match id {
                    ID_FROM => (self.from_edit, &mut self.from),
                    ID_TO => (self.to_edit, &mut self.to),
                    ID_YEARS => (self.years_edit, &mut self.years),
                    ID_MONTHS => (self.months_edit, &mut self.months),
                    _ => (self.days_edit, &mut self.days),
                };
                if !control.is_null() {
                    *value = platform::get_text(control);
                }
                self.result_a.clear();
                self.result_b.clear();
                self.error.clear();
                true
            }
            ID_OP if u32::from(code) == CBN_SELCHANGE => {
                self.add_subtract = platform::combo_selected(self.op_combo) == 0;
                self.result_a.clear();
                self.result_b.clear();
                self.error.clear();
                true
            }
            _ => false,
        }
    }

    fn display(&self, _shared: &Shared) -> String {
        self.result_b.clone()
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn date_modes_keep_input_action_and_results_together() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let mut shared = Shared::new();
            shared.attach(std::ptr::null_mut(), dpi, std::ptr::null_mut());
            shared.client = Rect::new(0, 0, shared.s(901.0), shared.s(480.0));
            for difference in [true, false] {
                let l = layout(&shared, difference);
                let last_input = if difference { l.to_edit } else { l.days_edit };
                assert!(l.calculate.y >= last_input.bottom());
                assert!(l.calculate.y - last_input.bottom() <= shared.s(GAP_ROW));
                assert!(l.result_b_label.y >= l.calculate.bottom());
                assert!(l.result_b.bottom() <= shared.client.bottom() - shared.s(metrics::MARGIN));
                assert!(l.error.x >= l.calculate.right());
                if difference {
                    assert!(l.result_a.bottom() <= l.result_b_label.y);
                }
            }
        }
    }
}
