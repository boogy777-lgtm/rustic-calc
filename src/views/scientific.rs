use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use crate::actions::{scientific_keys, standard_keys, Action, MemAction, SciFunc};
use crate::engine::{
    apply_angle, dms, factorial, format_scientific, parse_number, AngleMode, Mode, StandardEngine,
};
use crate::ui::{metrics, Rect, Ui};

use super::common::angle_selector;
use super::{Shared, View};

const GRID_ROWS: usize = metrics::SCIENTIFIC_ROWS;
const GRID_H: f32 = metrics::SCIENTIFIC_GRID_H;
const COLUMN_GAP: f32 = metrics::COLUMN_GAP;

pub struct ScientificView {
    engine: StandardEngine,
    angle: AngleMode,
    inv: bool,
    scientific_notation: bool,
}

impl ScientificView {
    pub fn new() -> Self {
        Self {
            engine: StandardEngine::new(),
            angle: AngleMode::Degrees,
            inv: false,
            scientific_notation: false,
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
            Action::ToggleExp => self.scientific_notation = !self.scientific_notation,
            Action::ParenOpen => self.engine.open_paren(),
            Action::ParenClose => self.engine.close_paren(),
            Action::Inv => self.inv = !self.inv,
            Action::SetAngle(mode) => self.angle = mode,
            Action::Func(func) => {
                let angle = self.angle;
                let inv = self.inv;
                match func {
                    SciFunc::Sin => self
                        .engine
                        .unary("sin", |v| apply_angle(v, angle, inv, "sin")),
                    SciFunc::Cos => self
                        .engine
                        .unary("cos", |v| apply_angle(v, angle, inv, "cos")),
                    SciFunc::Tan => self
                        .engine
                        .unary("tan", |v| apply_angle(v, angle, inv, "tan")),
                    SciFunc::Sinh => {
                        if inv {
                            self.engine.unary("sinh⁻¹", |v| Ok(v.asinh()))
                        } else {
                            self.engine.unary("sinh", |v| Ok(v.sinh()))
                        }
                    }
                    SciFunc::Cosh => {
                        if inv {
                            self.engine.unary("cosh⁻¹", |v| Ok(v.acosh()))
                        } else {
                            self.engine.unary("cosh", |v| Ok(v.cosh()))
                        }
                    }
                    SciFunc::Tanh => {
                        if inv {
                            self.engine.unary("tanh⁻¹", |v| Ok(v.atanh()))
                        } else {
                            self.engine.unary("tanh", |v| Ok(v.tanh()))
                        }
                    }
                    SciFunc::Ln => self.engine.unary("ln", |v| {
                        if v <= 0.0 {
                            Err(t.error_invalid_input.to_string())
                        } else {
                            Ok(v.ln())
                        }
                    }),
                    SciFunc::Log => self.engine.unary("log", |v| {
                        if v <= 0.0 {
                            Err(t.error_invalid_input.to_string())
                        } else {
                            Ok(v.log10())
                        }
                    }),
                    SciFunc::Int => self.engine.unary("Int", |v| Ok(v.trunc())),
                    SciFunc::Dms => self.engine.unary("dms", dms),
                    SciFunc::Cbrt => self.engine.unary("cuberoot", |v| Ok(v.cbrt())),
                    SciFunc::TenPow => self.engine.unary("10^", |v| Ok(10f64.powf(v))),
                }
            }
            Action::Mem(MemAction::Clear) => shared.memory = 0.0,
            Action::Mem(MemAction::Recall) => self.engine.recall(shared.memory),
            Action::Mem(MemAction::Store) => shared.memory = self.engine.current_value(),
            Action::Mem(MemAction::Add) => shared.memory += self.engine.current_value(),
            Action::Mem(MemAction::Sub) => shared.memory -= self.engine.current_value(),
            _ => {}
        }
    }

    fn key_action(vk: u16) -> Option<Action> {
        match vk {
            VK_RETURN => Some(Action::Equals),
            VK_BACK => Some(Action::Backspace),
            VK_ESCAPE => Some(Action::Clear),
            VK_DELETE => Some(Action::ClearEntry),
            _ => None,
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
            &self.engine.expression(shared.grouping),
            self.engine.has_error(),
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
        angle_selector(ui, angle, self.angle, shared.text());

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
            if self.inv { inv } else { None },
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
        if let Some(action) = Self::key_action(vk) {
            self.apply(action, shared);
            true
        } else {
            false
        }
    }

    fn display(&self, shared: &Shared) -> String {
        let text = crate::i18n::runtime_error(shared.lang, &self.engine.display(shared.grouping));
        if self.scientific_notation {
            parse_number(&text)
                .map(|v| format_scientific(v, shared.grouping))
                .unwrap_or(text)
        } else {
            text
        }
    }
}
