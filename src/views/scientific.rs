use crate::actions::{scientific_keys, standard_keys, Action};
use crate::engine::Mode;
use crate::ui::{metrics, Rect, Ui};
use rustic_calc_app::keypad::{action_for_char, action_for_key, ArithmeticSession};

use super::common::angle_selector;
use super::{keypad_key, Shared, View};

const GRID_ROWS: usize = metrics::SCIENTIFIC_ROWS;
const GRID_H: f32 = metrics::SCIENTIFIC_GRID_H;
const COLUMN_GAP: f32 = metrics::COLUMN_GAP;

pub struct ScientificView {
    session: ArithmeticSession,
}

impl ScientificView {
    pub fn new() -> Self {
        Self {
            session: ArithmeticSession::new(true),
        }
    }
}

impl View for ScientificView {
    fn mode(&self) -> Mode {
        Mode::Scientific
    }

    fn on_enter(&mut self, _shared: &mut Shared) {}

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let client = shared.client;
        let m = ui.s(metrics::MARGIN);
        let gap = ui.s(metrics::GAP);
        let display = Rect::new(
            client.x + m,
            client.y + m,
            client.w - m * 2,
            ui.s(metrics::DISPLAY_H),
        );
        ui.display(
            display,
            &self.display(shared),
            &self.session.expression(shared.grouping),
            self.session.engine.has_error(),
        );

        let column_w = ui.s(metrics::KEYPAD_W);
        let between = ui.s(COLUMN_GAP);
        let total = column_w * 2 + between;
        let left_x = client.x + (client.w - total) / 2;
        let top = display.bottom() + gap;
        // Make the segmented angle selector as tall as the shortfall between
        // the scientific grid and the standard keypad so both columns end on
        // the same baseline.
        let angle_h = (ui.s(metrics::KEYPAD_H) - ui.s(GRID_H) - ui.s(metrics::GAP_TIGHT))
            .max(ui.s(metrics::SELECTOR_ITEM_H + metrics::PAD_PANEL * 2.0));
        let angle = Rect::new(left_x, top, column_w, angle_h);
        angle_selector(ui, angle, self.session.angle, shared.text());

        let keys = scientific_keys();
        let inv = keys
            .iter()
            .position(|key| matches!(key.action, Action::Inv));
        ui.keypad_active(
            Rect::new(
                left_x,
                angle.bottom() + ui.s(metrics::GAP_TIGHT),
                column_w,
                ui.s(GRID_H),
            ),
            &keys,
            metrics::KEYPAD_COLS,
            GRID_ROWS,
            ui.s(metrics::GAP_KEYPAD),
            if self.session.inv { inv } else { None },
        );

        let right = Rect::new(
            left_x + column_w + between,
            top,
            column_w,
            ui.s(metrics::KEYPAD_H),
        );
        ui.keypad(
            right,
            &standard_keys(),
            metrics::KEYPAD_COLS,
            metrics::KEYPAD_ROWS,
            ui.s(metrics::GAP_KEYPAD),
        );
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        self.session.apply(action, shared.lang, &mut shared.memory);
    }

    fn char(&mut self, ch: char, shared: &mut Shared) -> bool {
        let Some(action) = action_for_char(self.mode(), ch) else {
            return false;
        };
        self.action(action, shared);
        true
    }

    fn key(&mut self, vk: u16, _ctrl: bool, shared: &mut Shared) -> bool {
        if let Some(action) = keypad_key(vk).and_then(|key| action_for_key(self.mode(), key)) {
            self.action(action, shared);
            true
        } else {
            false
        }
    }

    fn display(&self, shared: &Shared) -> String {
        self.session.display(shared.lang, shared.grouping)
    }
}
