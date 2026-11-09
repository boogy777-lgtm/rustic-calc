use crate::engine::{AngleMode, Op, ProgOp};

#[derive(Clone, Copy, Debug)]
pub enum MemAction {
    Clear,
    Recall,
    Store,
    Add,
    Sub,
}

#[derive(Clone, Copy, Debug)]
pub enum SciFunc {
    Sin,
    Cos,
    Tan,
    Sinh,
    Cosh,
    Tanh,
    Ln,
    Log,
    Int,
    Dms,
    Cbrt,
    TenPow,
}

#[derive(Clone, Copy, Debug)]
pub enum StatsFunc {
    Mean,
    MeanSq,
    Sum,
    SumSq,
    SdPop,
    SdSample,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Digit(char),
    Decimal,
    BinOp(Op),
    Equals,
    Clear,
    ClearEntry,
    Backspace,
    Sign,
    Percent,
    Sqrt,
    Reciprocal,
    Square,
    Cube,
    Factorial,
    Pi,
    Exp,
    ToggleExp,
    ParenOpen,
    ParenClose,
    Func(SciFunc),
    Inv,
    ProgDigit(char),
    ProgBin(ProgOp),
    ProgEquals,
    ProgNot,
    ProgSign,
    ProgBackspace,
    ProgClearEntry,
    ProgClear,
    ProgSqrt,
    ProgRecip,
    ProgBase(u8),
    ProgWord(u8),
    ProgBit(u32),
    Mem(MemAction),
    StatsDigit(char),
    StatsDecimal,
    StatsAdd,
    StatsClearAll,
    StatsClear,
    StatsBackspace,
    StatsSign,
    StatsExp,
    StatsCalc(StatsFunc),
    SetAngle(AngleMode),
    Calculate,
    CoordinateInfo,
    CoordinateReset,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyStyle {
    Normal,
    Operator,
    Equals,
    Memory,
    Blank,
}

#[derive(Clone)]
pub struct KeySpec {
    pub label: &'static str,
    pub action: Action,
    pub style: KeyStyle,
    pub span_x: usize,
    pub span_y: usize,
    pub enabled: bool,
}

impl KeySpec {
    pub const fn new(label: &'static str, action: Action) -> Self {
        Self {
            label,
            action,
            style: KeyStyle::Normal,
            span_x: 1,
            span_y: 1,
            enabled: true,
        }
    }
    pub const fn op(label: &'static str, action: Action) -> Self {
        Self {
            label,
            action,
            style: KeyStyle::Operator,
            span_x: 1,
            span_y: 1,
            enabled: true,
        }
    }
    pub const fn mem(label: &'static str, action: Action) -> Self {
        Self {
            label,
            action,
            style: KeyStyle::Memory,
            span_x: 1,
            span_y: 1,
            enabled: true,
        }
    }
    pub const fn blank() -> Self {
        Self {
            label: "",
            action: Action::None,
            style: KeyStyle::Blank,
            span_x: 1,
            span_y: 1,
            enabled: false,
        }
    }
    pub const fn equals() -> Self {
        Self {
            label: "=",
            action: Action::Equals,
            style: KeyStyle::Equals,
            span_x: 1,
            span_y: 2,
            enabled: true,
        }
    }
    pub const fn span(mut self, x: usize, y: usize) -> Self {
        self.span_x = x;
        self.span_y = y;
        self
    }
    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

pub fn standard_keys() -> Vec<KeySpec> {
    use Action::*;
    vec![
        KeySpec::mem("MC", Mem(MemAction::Clear)),
        KeySpec::mem("MR", Mem(MemAction::Recall)),
        KeySpec::mem("MS", Mem(MemAction::Store)),
        KeySpec::mem("M+", Mem(MemAction::Add)),
        KeySpec::mem("M−", Mem(MemAction::Sub)),
        KeySpec::op("←", Backspace),
        KeySpec::op("CE", ClearEntry),
        KeySpec::op("C", Clear),
        KeySpec::op("±", Sign),
        KeySpec::op("√", Sqrt),
        KeySpec::new("7", Digit('7')),
        KeySpec::new("8", Digit('8')),
        KeySpec::new("9", Digit('9')),
        KeySpec::op("÷", BinOp(Op::Div)),
        KeySpec::op("%", Percent),
        KeySpec::new("4", Digit('4')),
        KeySpec::new("5", Digit('5')),
        KeySpec::new("6", Digit('6')),
        KeySpec::op("×", BinOp(Op::Mul)),
        KeySpec::op("1/x", Reciprocal),
        KeySpec::new("1", Digit('1')),
        KeySpec::new("2", Digit('2')),
        KeySpec::new("3", Digit('3')),
        KeySpec::op("−", BinOp(Op::Sub)),
        KeySpec::equals(),
        KeySpec::new("0", Digit('0')).span(2, 1),
        KeySpec::new(".", Decimal),
        KeySpec::op("+", BinOp(Op::Add)),
    ]
}

pub fn scientific_keys() -> Vec<KeySpec> {
    use Action::*;
    vec![
        KeySpec::blank(),
        KeySpec::op("Inv", Inv),
        KeySpec::new("ln", Func(SciFunc::Ln)),
        KeySpec::new("(", ParenOpen),
        KeySpec::new(")", ParenClose),
        KeySpec::new("Int", Func(SciFunc::Int)),
        KeySpec::new("sinh", Func(SciFunc::Sinh)),
        KeySpec::new("sin", Func(SciFunc::Sin)),
        KeySpec::new("x²", Square),
        KeySpec::new("n!", Factorial),
        KeySpec::new("dms", Func(SciFunc::Dms)),
        KeySpec::new("cosh", Func(SciFunc::Cosh)),
        KeySpec::new("cos", Func(SciFunc::Cos)),
        KeySpec::new("xʸ", BinOp(Op::Pow)),
        KeySpec::new("y√x", BinOp(Op::Root)),
        KeySpec::new("π", Pi),
        KeySpec::new("tanh", Func(SciFunc::Tanh)),
        KeySpec::new("tan", Func(SciFunc::Tan)),
        KeySpec::new("x³", Cube),
        KeySpec::new("³√x", Func(SciFunc::Cbrt)),
        KeySpec::new("F-E", ToggleExp),
        KeySpec::new("Exp", Exp),
        KeySpec::new("Mod", BinOp(Op::Mod)),
        KeySpec::new("log", Func(SciFunc::Log)),
        KeySpec::new("10ˣ", Func(SciFunc::TenPow)),
    ]
}

pub fn programmer_keys() -> Vec<KeySpec> {
    use Action::*;
    vec![
        KeySpec::blank(),
        KeySpec::new("Mod", ProgBin(ProgOp::Mod)),
        KeySpec::new("A", ProgDigit('A')),
        KeySpec::mem("MC", Mem(MemAction::Clear)),
        KeySpec::mem("MR", Mem(MemAction::Recall)),
        KeySpec::mem("MS", Mem(MemAction::Store)),
        KeySpec::mem("M+", Mem(MemAction::Add)),
        KeySpec::mem("M−", Mem(MemAction::Sub)),
        KeySpec::new("(", ParenOpen),
        KeySpec::new(")", ParenClose),
        KeySpec::new("B", ProgDigit('B')),
        KeySpec::op("←", ProgBackspace),
        KeySpec::op("CE", ProgClearEntry),
        KeySpec::op("C", ProgClear),
        KeySpec::op("±", ProgSign),
        KeySpec::op("√", ProgSqrt),
        KeySpec::new("RoL", ProgBin(ProgOp::Rol)),
        KeySpec::new("RoR", ProgBin(ProgOp::Ror)),
        KeySpec::new("C", ProgDigit('C')),
        KeySpec::new("7", ProgDigit('7')),
        KeySpec::new("8", ProgDigit('8')),
        KeySpec::new("9", ProgDigit('9')),
        KeySpec::op("÷", ProgBin(ProgOp::Div)),
        KeySpec::op("%", ProgBin(ProgOp::Mod)),
        KeySpec::new("Or", ProgBin(ProgOp::Or)),
        KeySpec::new("Xor", ProgBin(ProgOp::Xor)),
        KeySpec::new("D", ProgDigit('D')),
        KeySpec::new("4", ProgDigit('4')),
        KeySpec::new("5", ProgDigit('5')),
        KeySpec::new("6", ProgDigit('6')),
        KeySpec::op("×", ProgBin(ProgOp::Mul)),
        KeySpec::op("1/x", ProgRecip),
        KeySpec::new("Lsh", ProgBin(ProgOp::Lsh)),
        KeySpec::new("Rsh", ProgBin(ProgOp::Rsh)),
        KeySpec::new("E", ProgDigit('E')),
        KeySpec::new("1", ProgDigit('1')),
        KeySpec::new("2", ProgDigit('2')),
        KeySpec::new("3", ProgDigit('3')),
        KeySpec::op("−", ProgBin(ProgOp::Sub)),
        KeySpec {
            label: "=",
            action: ProgEquals,
            style: KeyStyle::Equals,
            span_x: 1,
            span_y: 2,
            enabled: true,
        },
        KeySpec::new("Not", ProgNot),
        KeySpec::new("And", ProgBin(ProgOp::And)),
        KeySpec::new("F", ProgDigit('F')),
        KeySpec::new("0", ProgDigit('0')).span(2, 1),
        KeySpec::new(".", None).disabled(),
        KeySpec::op("+", ProgBin(ProgOp::Add)),
    ]
}

pub fn stats_keys(add_label: &'static str) -> Vec<KeySpec> {
    use Action::*;
    vec![
        KeySpec::mem("MC", Mem(MemAction::Clear)),
        KeySpec::mem("MR", Mem(MemAction::Recall)),
        KeySpec::mem("MS", Mem(MemAction::Store)),
        KeySpec::mem("M+", Mem(MemAction::Add)),
        KeySpec::mem("M−", Mem(MemAction::Sub)),
        KeySpec::op("←", StatsBackspace),
        KeySpec::op("CAD", StatsClearAll),
        KeySpec::op("C", StatsClear),
        KeySpec::op("F-E", StatsExp),
        KeySpec::op("Exp", StatsExp),
        KeySpec::new("7", StatsDigit('7')),
        KeySpec::new("8", StatsDigit('8')),
        KeySpec::new("9", StatsDigit('9')),
        KeySpec::new("x̄", StatsCalc(StatsFunc::Mean)),
        KeySpec::new("x̄²", StatsCalc(StatsFunc::MeanSq)),
        KeySpec::new("4", StatsDigit('4')),
        KeySpec::new("5", StatsDigit('5')),
        KeySpec::new("6", StatsDigit('6')),
        KeySpec::new("Σx", StatsCalc(StatsFunc::Sum)),
        KeySpec::new("Σx²", StatsCalc(StatsFunc::SumSq)),
        KeySpec::new("1", StatsDigit('1')),
        KeySpec::new("2", StatsDigit('2')),
        KeySpec::new("3", StatsDigit('3')),
        KeySpec::new("σn", StatsCalc(StatsFunc::SdPop)),
        KeySpec::new("σn-1", StatsCalc(StatsFunc::SdSample)),
        KeySpec::new("0", StatsDigit('0')).span(2, 1),
        KeySpec::new(".", StatsDecimal),
        KeySpec::op("±", StatsSign),
        KeySpec::new(add_label, StatsAdd),
    ]
}
