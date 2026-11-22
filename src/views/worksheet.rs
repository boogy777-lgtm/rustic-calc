use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{CBN_SELCHANGE, EN_CHANGE};

use crate::actions::{Action, KeyStyle};
use crate::engine::{Mode, WorksheetKind};
use crate::platform;
use crate::ui::{metrics, Rect, Stack, Ui};
use rustic_calc_app::worksheet::WorksheetSession;

use super::common::sidebar;
use super::{Shared, View};

const ID_TARGET: u16 = 399;
const ID_FIELD: u16 = 400;

const ROW_GAP: f32 = 8.0;
const LABEL_W: f32 = 240.0;
const FIELD_OFFSET: f32 = 260.0;
const RESULT_OFFSET: f32 = 144.0;
const ERROR_H: f32 = 22.0;

struct Layout {
    title: Rect,
    target: Rect,
    rows: Vec<Rect>,
    calculate: Rect,
    result_label: Rect,
    result: Rect,
    error: Rect,
}

fn layout(shared: &Shared, count: usize, target_index: usize) -> Layout {
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
    let target = stack.take(s(metrics::FIELD_H));
    stack.advance(s(metrics::GAP_SECTION + metrics::GAP_BLOCK));

    let rows = (0..count)
        .map(|i| {
            if i == target_index {
                return Rect::new(content.x, 0, content.w, s(metrics::FIELD_H));
            }
            let row = stack.take(s(metrics::FIELD_H));
            stack.advance(s(ROW_GAP));
            row
        })
        .collect();

    let calc_h = s(metrics::FORM_ACTION_H);
    let label_row = stack.take(s(metrics::FIELD_LABEL_H));
    let result_label = Rect::new(
        content.x + s(RESULT_OFFSET),
        label_row.y,
        content.w - s(RESULT_OFFSET),
        label_row.h,
    );
    stack.advance(s(metrics::GAP_HAIRLINE));
    let buttons_y = stack.take(0).y;
    let calculate = Rect::new(content.x, buttons_y, s(metrics::FORM_ACTION_W), calc_h);
    let result = Rect::new(
        content.x + s(RESULT_OFFSET),
        buttons_y,
        content.w - s(RESULT_OFFSET),
        s(metrics::RESULT_H),
    );
    let error = Rect::new(
        content.x,
        buttons_y + calc_h + s(6.0),
        content.w,
        s(ERROR_H),
    );
    Layout {
        title,
        target,
        rows,
        calculate,
        result_label,
        result,
        error,
    }
}

pub struct WorksheetView {
    session: WorksheetSession,
    target_combo: HWND,
    edits: Vec<HWND>,
}

impl WorksheetView {
    pub fn new(kind: WorksheetKind) -> Self {
        Self {
            session: WorksheetSession::new(kind),
            target_combo: std::ptr::null_mut(),
            edits: Vec::new(),
        }
    }

    fn target_index(&self) -> usize {
        self.session
            .fields()
            .iter()
            .position(|field| *field == self.session.target())
            .unwrap_or(0)
    }

    fn target_option_index(&self) -> usize {
        self.session
            .target_options()
            .iter()
            .position(|field| *field == self.session.target())
            .unwrap_or(0)
    }

    fn create_controls(&mut self, shared: &mut Shared) {
        self.destroy_controls();
        if shared.client.w <= 0 {
            return;
        }
        let l = layout(shared, self.session.fields().len(), self.target_index());
        let options: Vec<&str> = self
            .session
            .target_options()
            .iter()
            .map(|field| field.label(shared.lang))
            .collect();
        self.target_combo = shared.combo(ID_TARGET, l.target, &options, self.target_option_index());
        self.edits = self
            .session
            .fields()
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let r = l.rows[i];
                let rect = Rect::new(
                    r.x + shared.s(FIELD_OFFSET),
                    r.y,
                    r.w - shared.s(FIELD_OFFSET),
                    r.h,
                );
                shared.edit(
                    ID_FIELD + i as u16,
                    rect,
                    self.session.text(*field).unwrap_or(""),
                )
            })
            .collect();
        self.update_visibility();
    }

    fn destroy_controls(&mut self) {
        platform::destroy_control(self.target_combo);
        for edit in self.edits.drain(..) {
            platform::destroy_control(edit);
        }
        self.target_combo = std::ptr::null_mut();
    }

    fn move_controls(&self, shared: &Shared) {
        let l = layout(shared, self.session.fields().len(), self.target_index());
        let combo_h = shared.s(metrics::COMBO_POPUP_H);
        if !self.target_combo.is_null() {
            platform::move_control(
                self.target_combo,
                l.target.x,
                l.target.y,
                l.target.w,
                combo_h,
            );
        }
        for (edit, row) in self.edits.iter().zip(l.rows.iter()) {
            if !edit.is_null() {
                let field_x = row.x + shared.s(FIELD_OFFSET);
                platform::move_control(
                    *edit,
                    field_x,
                    row.y,
                    row.w - shared.s(FIELD_OFFSET),
                    row.h,
                );
            }
        }
    }

    fn update_visibility(&self) {
        let target = self.target_index();
        for (i, edit) in self.edits.iter().enumerate() {
            platform::show_control(*edit, i != target);
        }
    }

    fn sync_values(&mut self) {
        let fields = self.session.fields().to_vec();
        let target = self.target_index();
        for (i, (field, edit)) in fields.iter().zip(self.edits.iter()).enumerate() {
            if i != target && !edit.is_null() {
                self.session.set_text(*field, platform::get_text(*edit));
            }
        }
    }
}

impl View for WorksheetView {
    fn mode(&self) -> Mode {
        Mode::Worksheet(self.session.kind())
    }

    fn on_enter(&mut self, shared: &mut Shared) {
        self.create_controls(shared);
    }

    fn on_exit(&mut self, _shared: &mut Shared) {
        self.destroy_controls();
    }

    fn on_resize(&mut self, shared: &mut Shared) {
        if self.target_combo.is_null() {
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
        let target = self.target_index();
        let l = layout(shared, self.session.fields().len(), target);
        let t = shared.text();
        ui.heading(l.title, t.worksheet_title);
        let label_w = ui.s(LABEL_W);
        for (i, row) in l.rows.iter().enumerate() {
            if i == target {
                continue;
            }
            ui.form_label(
                Rect::new(row.x, row.y, label_w, row.h),
                self.session.fields()[i].label(shared.lang),
            );
        }
        ui.button(
            l.calculate,
            t.calculate,
            KeyStyle::Normal,
            Action::Calculate,
        );
        ui.form_label(
            l.result_label,
            self.session.fields()[target].label(shared.lang),
        );
        let result = self
            .session
            .formatted_result(shared.grouping)
            .unwrap_or_default();
        ui.result_box(l.result, &result, false);
        if let Some(error) = self.session.display_error(shared.lang) {
            ui.error(l.error, &error);
        }
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        if matches!(action, Action::Calculate) {
            self.sync_values();
            let _ = self.session.calculate();
            return;
        }
        let current = shared.sidebar.current_value();
        shared.sidebar_action(action, current);
    }

    fn command(&mut self, id: u16, code: u16, shared: &mut Shared) -> bool {
        if id == ID_TARGET && u32::from(code) == CBN_SELCHANGE {
            self.sync_values();
            let pos = platform::combo_selected(self.target_combo);
            if let Some(target) = self.session.target_options().get(pos).copied() {
                self.session.set_target(target);
            }
            self.update_visibility();
            self.move_controls(shared);
            return true;
        }
        if id >= ID_FIELD
            && id < ID_FIELD + self.session.fields().len() as u16
            && u32::from(code) == EN_CHANGE
        {
            self.sync_values();
            return true;
        }
        false
    }

    fn display(&self, shared: &Shared) -> String {
        self.session
            .formatted_result(shared.grouping)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::units;

    #[test]
    fn every_worksheet_target_has_compact_ordered_rows_and_labelled_result() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let mut shared = Shared::new();
            shared.attach(std::ptr::null_mut(), dpi, std::ptr::null_mut());
            shared.client = Rect::new(0, 0, shared.s(901.0), shared.s(480.0));
            for kind in [
                WorksheetKind::Mortgage,
                WorksheetKind::VehicleLease,
                WorksheetKind::FuelMpg,
                WorksheetKind::FuelL100,
            ] {
                let def = units::worksheet_def(kind);
                for target in 0..def.args.len() {
                    let l = layout(&shared, def.args.len(), target);
                    let rows: Vec<_> = l
                        .rows
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != target)
                        .map(|(_, r)| r)
                        .collect();
                    assert!(rows
                        .windows(2)
                        .all(|w| w[1].y - w[0].bottom() == shared.s(ROW_GAP)));
                    assert!(l.result_label.y >= rows.last().unwrap().bottom());
                    assert_eq!(l.result_label.x, l.result.x);
                    assert!(l.result_label.bottom() <= l.result.y);
                    assert!(l.error.bottom() <= shared.client.bottom() - shared.s(metrics::MARGIN));
                }
            }
        }
    }
}
