use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use crate::actions::{standard_keys, Action, MemAction};
use crate::engine::{factorial, Mode, StandardEngine};
use crate::ui::{metrics, Rect, Ui};

use super::{Shared, View};

pub struct StandardView {
    engine: StandardEngine,
}

impl StandardView {
    pub fn new() -> Self {
        Self {
            engine: StandardEngine::new(),
        }
    }

    fn apply(&mut self, action: Action, shared: &mut Shared) {
        let t = shared.text();
        match action {
            Action::Digit(ch) => self.engine.digit(ch),
            Action::Decimal => self.engine.decimal(),
            Action::BinOp(op) => self.engine.push_op(op),
            Action::Equals => self.engine.equals(),
            Action::Clear => self.engine.clear_all(),
            Action::ClearEntry => self.engine.clear_entry(),
            Action::Backspace => self.engine.backspace(),
            Action::Sign => self.engine.toggle_sign(),
            Action::Percent => self.engine.percent(),
            Action::Sqrt => self.engine.unary("sqrt", |v| {
                if v < 0.0 {
                    Err(t.error_invalid_input.to_string())
                } else {
                    Ok(v.sqrt())
                }
            }),
            Action::Reciprocal => self.engine.unary("1/x", |v| {
                if v == 0.0 {
                    Err(t.error_undefined.to_string())
                } else {
                    Ok(1.0 / v)
                }
            }),
            Action::Square => self.engine.unary("x²", |v| Ok(v * v)),
            Action::Cube => self.engine.unary("x³", |v| Ok(v * v * v)),
            Action::Factorial => self.engine.unary("n!", factorial),
            Action::Pi => self.engine.set_constant(crate::engine::PI),
            Action::Exp => self.engine.exp_entry(),
            Action::ParenOpen => self.engine.open_paren(),
            Action::ParenClose => self.engine.close_paren(),
            Action::Mem(MemAction::Clear) => shared.memory = 0.0,
            Action::Mem(MemAction::Recall) => self.engine.recall(shared.memory),
            Action::Mem(MemAction::Store) => shared.memory = self.engine.current_value(),
            Action::Mem(MemAction::Add) => shared.memory += self.engine.current_value(),
            Action::Mem(MemAction::Sub) => shared.memory -= self.engine.current_value(),
            _ => {}
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
            &self.engine.expression(shared.grouping),
            self.engine.has_error(),
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
        self.apply(action, shared);
    }

    fn char(&mut self, ch: char, shared: &mut Shared) -> bool {
        let action = match ch {
            '0'..='9' => Action::Digit(ch),
            '.' | ',' => Action::Decimal,
            '+' => Action::BinOp(crate::engine::Op::Add),
            '-' => Action::BinOp(crate::engine::Op::Sub),
            '*' => Action::BinOp(crate::engine::Op::Mul),
            '/' => Action::BinOp(crate::engine::Op::Div),
            '=' => Action::Equals,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn key(&mut self, vk: u16, _ctrl: bool, shared: &mut Shared) -> bool {
        let action = match vk {
            VK_RETURN => Action::Equals,
            VK_BACK => Action::Backspace,
            VK_ESCAPE => Action::Clear,
            VK_DELETE => Action::ClearEntry,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn display(&self, shared: &Shared) -> String {
        crate::i18n::runtime_error(shared.lang, &self.engine.display(shared.grouping))
    }
}
