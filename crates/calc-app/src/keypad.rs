//! Backend-neutral commands and state for the keyboard-driven calculators.
use crate::actions::{Action, MemAction, SciFunc, StatsFunc};
use crate::engine::{
    apply_angle, digit_allowed, dms, factorial, format_scientific, parse_number, AngleMode, Mode,
    Op, ProgOp, ProgrammerState, StandardEngine, StatsState, PI,
};
use crate::i18n::{self, Lang};

pub struct ArithmeticSession {
    pub engine: StandardEngine,
    pub angle: AngleMode,
    pub inv: bool,
    pub scientific_notation: bool,
    scientific: bool,
}

impl ArithmeticSession {
    pub fn new(scientific: bool) -> Self {
        Self {
            engine: StandardEngine::new(),
            angle: AngleMode::Degrees,
            inv: false,
            scientific_notation: false,
            scientific,
        }
    }

    pub fn apply(&mut self, action: Action, lang: Lang, memory: &mut f64) {
        let t = lang.text();
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
            Action::Pi => self.engine.set_constant(PI),
            Action::Exp => self.engine.exp_entry(),
            Action::ParenOpen => self.engine.open_paren(),
            Action::ParenClose => self.engine.close_paren(),
            Action::ToggleExp if self.scientific => {
                self.scientific_notation = !self.scientific_notation;
            }
            Action::Inv if self.scientific => self.inv = !self.inv,
            Action::SetAngle(mode) if self.scientific => self.angle = mode,
            Action::Func(func) if self.scientific => {
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
                    SciFunc::Sinh if inv => self.engine.unary("sinh⁻¹", |v| Ok(v.asinh())),
                    SciFunc::Sinh => self.engine.unary("sinh", |v| Ok(v.sinh())),
                    SciFunc::Cosh if inv => self.engine.unary("cosh⁻¹", |v| Ok(v.acosh())),
                    SciFunc::Cosh => self.engine.unary("cosh", |v| Ok(v.cosh())),
                    SciFunc::Tanh if inv => self.engine.unary("tanh⁻¹", |v| Ok(v.atanh())),
                    SciFunc::Tanh => self.engine.unary("tanh", |v| Ok(v.tanh())),
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
            Action::Mem(MemAction::Clear) => *memory = 0.0,
            Action::Mem(MemAction::Recall) => self.engine.recall(*memory),
            Action::Mem(MemAction::Store) => *memory = self.engine.current_value(),
            Action::Mem(MemAction::Add) => *memory += self.engine.current_value(),
            Action::Mem(MemAction::Sub) => *memory -= self.engine.current_value(),
            _ => {}
        }
    }

    pub fn display(&self, lang: Lang, grouping: bool) -> String {
        let text = i18n::runtime_error(lang, &self.engine.display(grouping));
        if self.scientific_notation {
            parse_number(&text)
                .map(|value| format_scientific(value, grouping))
                .unwrap_or(text)
        } else {
            text
        }
    }

    pub fn expression(&self, grouping: bool) -> String {
        self.engine.expression(grouping)
    }

    pub fn history(&self) -> &[String] {
        &self.engine.history
    }
}

pub struct ProgrammerSession {
    pub state: ProgrammerState,
}

impl ProgrammerSession {
    pub fn new() -> Self {
        Self {
            state: ProgrammerState::new(),
        }
    }

    pub fn apply(&mut self, action: Action, memory: &mut f64) {
        let state = &mut self.state;
        match action {
            Action::ProgDigit(ch) if digit_allowed(state.base, ch) => state.digit(ch),
            Action::ProgBin(op) => state.push_op(op),
            Action::ProgEquals => state.equals(),
            Action::ProgNot => state.not(),
            Action::ProgSign => state.toggle_sign(),
            Action::ProgBackspace => state.backspace(),
            Action::ProgClearEntry => state.clear_entry(),
            Action::ProgClear => state.clear_all(),
            Action::ProgSqrt => state.sqrt(),
            Action::ProgRecip => state.reciprocal(),
            Action::ProgBase(base) => state.set_base(base),
            Action::ProgWord(word) => state.set_word(word),
            Action::ProgBit(bit) => state.toggle_bit(bit),
            Action::Mem(MemAction::Clear) => *memory = 0.0,
            Action::Mem(MemAction::Recall) => state.recall(*memory as u64),
            Action::Mem(MemAction::Store) => *memory = state.value as f64,
            Action::Mem(MemAction::Add) => *memory += state.value as f64,
            Action::Mem(MemAction::Sub) => *memory -= state.value as f64,
            _ => {}
        }
    }

    pub fn display(&self, lang: Lang) -> String {
        i18n::runtime_error(lang, &self.state.display())
    }
}

/// Applies the same statistics commands to the standalone mode or its sidebar.
/// `current` can be supplied by a different display for the memory commands.
pub fn apply_statistics(
    action: Action,
    lang: Lang,
    stats: &mut StatsState,
    memory: &mut f64,
    current: f64,
) -> bool {
    let t = lang.text();
    match action {
        Action::Mem(MemAction::Clear) => *memory = 0.0,
        Action::Mem(MemAction::Recall) => stats.recall(*memory),
        Action::Mem(MemAction::Store) => *memory = current,
        Action::Mem(MemAction::Add) => *memory += current,
        Action::Mem(MemAction::Sub) => *memory -= current,
        Action::StatsDigit(ch) => stats.digit(ch),
        Action::StatsDecimal => stats.decimal(),
        Action::StatsAdd => stats.add_data(),
        Action::StatsClearAll => stats.clear_dataset(),
        Action::StatsClear => stats.clear_entry(),
        Action::StatsBackspace => stats.backspace(),
        Action::StatsSign => stats.toggle_sign(),
        Action::StatsExp => stats.exp_entry(),
        Action::StatsCalc(func) => {
            let (label, value) = match func {
                StatsFunc::Mean => (t.stats_average, stats.mean()),
                StatsFunc::MeanSq => (t.stats_average_squares, stats.mean_squares()),
                StatsFunc::Sum => (t.stats_sum, stats.sum()),
                StatsFunc::SumSq => (t.stats_sum_squares, stats.sum_squares()),
                StatsFunc::SdPop => (t.stats_sd, stats.std_dev_population()),
                StatsFunc::SdSample => (t.stats_sd_sample, stats.std_dev_sample()),
            };
            stats.show(label, value);
        }
        _ => return false,
    }
    true
}

#[derive(Clone, Copy, Debug)]
pub enum KeyInput {
    Enter,
    Backspace,
    Escape,
    Delete,
}

pub fn action_for_char(mode: Mode, ch: char) -> Option<Action> {
    use Action::*;
    match mode {
        Mode::Standard | Mode::Scientific => match ch {
            '0'..='9' => Some(Digit(ch)),
            '.' | ',' => Some(Decimal),
            '+' => Some(BinOp(Op::Add)),
            '-' => Some(BinOp(Op::Sub)),
            '*' => Some(BinOp(Op::Mul)),
            '/' => Some(BinOp(Op::Div)),
            '=' => Some(Equals),
            _ => Option::None,
        },
        Mode::Programmer => match ch {
            '0'..='9' | 'a'..='f' | 'A'..='F' => Some(ProgDigit(ch)),
            '+' => Some(ProgBin(ProgOp::Add)),
            '-' => Some(ProgBin(ProgOp::Sub)),
            '*' => Some(ProgBin(ProgOp::Mul)),
            '/' => Some(ProgBin(ProgOp::Div)),
            '%' => Some(ProgBin(ProgOp::Mod)),
            '=' => Some(ProgEquals),
            _ => Option::None,
        },
        Mode::Statistics => match ch {
            '0'..='9' => Some(StatsDigit(ch)),
            '.' | ',' => Some(StatsDecimal),
            _ => Option::None,
        },
        _ => Option::None,
    }
}

pub fn action_for_key(mode: Mode, key: KeyInput) -> Option<Action> {
    use Action::*;
    use KeyInput::{Delete, Enter, Escape};
    match (mode, key) {
        (Mode::Standard | Mode::Scientific, Enter) => Some(Equals),
        (Mode::Standard | Mode::Scientific, KeyInput::Backspace) => Some(Backspace),
        (Mode::Standard | Mode::Scientific, Escape) => Some(Clear),
        (Mode::Standard | Mode::Scientific, Delete) => Some(ClearEntry),
        (Mode::Programmer, Enter) => Some(ProgEquals),
        (Mode::Programmer, KeyInput::Backspace) => Some(ProgBackspace),
        (Mode::Programmer, Escape) => Some(ProgClear),
        (Mode::Programmer, Delete) => Some(ProgClearEntry),
        (Mode::Statistics, Enter) => Some(StatsAdd),
        (Mode::Statistics, KeyInput::Backspace) => Some(StatsBackspace),
        (Mode::Statistics, Escape) => Some(StatsClear),
        _ => Option::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_commands_keep_separate_modes_and_one_memory() {
        let mut memory = 0.0;
        let mut standard = ArithmeticSession::new(false);
        for ch in "7+3=".chars() {
            standard.apply(
                action_for_char(Mode::Standard, ch).unwrap(),
                Lang::En,
                &mut memory,
            );
        }
        standard.apply(Action::Mem(MemAction::Store), Lang::En, &mut memory);
        let mut scientific = ArithmeticSession::new(true);
        scientific.apply(Action::Mem(MemAction::Recall), Lang::Ru, &mut memory);
        scientific.apply(Action::Square, Lang::Ru, &mut memory);
        assert_eq!(standard.display(Lang::En, false), "10");
        assert_eq!(scientific.display(Lang::Ru, false), "100");
        assert_eq!(memory, 10.0);
    }

    #[test]
    fn programmer_filters_digits_after_base_switch() {
        let mut memory = 0.0;
        let mut programmer = ProgrammerSession::new();
        programmer.apply(Action::ProgBase(2), &mut memory);
        for ch in "1021".chars() {
            programmer.apply(action_for_char(Mode::Programmer, ch).unwrap(), &mut memory);
        }
        assert_eq!(programmer.state.value, 5);
    }
}
