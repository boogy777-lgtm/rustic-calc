use crate::actions::{standard_keys, Action};
use crate::engine::Mode;
use crate::ui::{metrics, Rect, Ui};
use rustic_calc_app::keypad::{action_for_char, action_for_key, ArithmeticSession};

use super::{keypad_key, Shared, View};

pub struct StandardView {
    session: ArithmeticSession,
}

impl StandardView {
    pub fn new() -> Self {
        Self {
            session: ArithmeticSession::new(false),
        }
    }
}

impl View for StandardView {
    fn mode(&self) -> Mode {
        Mode::Standard
    }

    fn on_enter(&mut self, _shared: &mut Shared) {}

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let client = shared.client;
        let m = ui.s(metrics::MARGIN);
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
        let keypad_w = ui.s(metrics::KEYPAD_W);
        let keypad = Rect::new(
            client.x + (client.w - keypad_w) / 2,
            display.bottom() + ui.s(metrics::GAP),
            keypad_w,
            ui.s(metrics::KEYPAD_H),
        );
        ui.keypad(
            keypad,
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
        let Some(action) = keypad_key(vk).and_then(|key| action_for_key(self.mode(), key)) else {
            return false;
        };
        self.action(action, shared);
        true
    }

    fn display(&self, shared: &Shared) -> String {
        self.session.display(shared.lang, shared.grouping)
    }
}
