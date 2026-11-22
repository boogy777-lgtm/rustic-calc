//! Calculator engines for the standard, scientific, programmer and statistics views.
//!
//! The original program is the Windows Calculator 6.3 engine.  Its menus and
//! constants were recovered from `calc1.exe` and its en-US MUI.  This is an
//! independent implementation in safe Rust.

pub const PI: f64 = 3.1415926535897932384626433832795;

/// Recovered display limit: the original shows 16 digits.  For values at or
/// above 1 this is 16 significant digits; below 1 it is 16 fractional digits.
/// Input is limited to the same 16-digit window.
pub const DISPLAY_DIGITS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Standard,
    Scientific,
    Programmer,
    Statistics,
    UnitConversion,
    DateCalculation,
    Worksheet(WorksheetKind),
    Coordinates,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorksheetKind {
    Mortgage,
    VehicleLease,
    FuelMpg,
    FuelL100,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleMode {
    Degrees,
    Radians,
    Gradians,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Root,
    Mod,
}

impl Op {
    fn precedence(self) -> u8 {
        match self {
            Op::Add | Op::Sub => 1,
            Op::Mul | Op::Div | Op::Mod => 2,
            Op::Pow | Op::Root => 3,
        }
    }

    fn right_associative(self) -> bool {
        // The original evaluates chained powers/roots left to right:
        // `2 ^ 3 ^ 2 =` -> 64 and `16 yroot 2 yroot 2 =` -> 2.
        let _ = self;
        false
    }

    fn symbol(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "−",
            Op::Mul => "×",
            Op::Div => "÷",
            Op::Pow => "^",
            Op::Root => "√",
            Op::Mod => "Mod",
        }
    }

    fn apply(self, a: f64, b: f64) -> Result<f64, String> {
        let result = match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div => {
                if b == 0.0 {
                    return Err(divide_by_zero_message(a));
                }
                a / b
            }
            Op::Pow => a.powf(b),
            Op::Root => {
                if b == 0.0 {
                    return Err("Result is undefined".to_string());
                }
                a.powf(1.0 / b)
            }
            Op::Mod => {
                if b == 0.0 {
                    return Err(divide_by_zero_message(a));
                }
                a % b
            }
        };
        if result.is_nan() {
            return Err("Invalid input".to_string());
        }
        if result.is_infinite() {
            return Err("Overflow".to_string());
        }
        Ok(result)
    }
}

/// The original distinguishes `0 / 0` from a non-zero numerator divided by
/// zero.  MUI strings 101 and 99 respectively (decompiled error return path in
/// the standard engine, 0x1400058fc and the command router 0x140004dfc).
fn divide_by_zero_message(numerator: f64) -> String {
    if numerator == 0.0 {
        "Result is undefined".to_string()
    } else {
        "Cannot divide by zero".to_string()
    }
}

#[derive(Clone, Debug)]
enum Token {
    Num(f64),
    Op(Op),
    LParen,
    RParen,
}

/// Formats with the comma decimal separator and optional digit grouping.
/// The original keeps more decimal digits internally; its recovered display
/// window is 16 digits (16 significant digits at or above 1, 16 fractional
/// digits below 1).  This implementation uses f64 and therefore can differ in
/// the last displayed digit after a division.
/// Formats one value in exponential notation, e.g. `1,234567890123456e+3`.
pub fn format_scientific(value: f64, _grouping: bool) -> String {
    if value.is_nan() {
        return "Invalid input".to_string();
    }
    if value.is_infinite() {
        return "Overflow".to_string();
    }
    if value == 0.0 {
        return "0e+0".to_string();
    }
    let mut s = format!("{value:.15e}");
    if let Some(e) = s.find('e') {
        let mut mantissa = s[..e].to_string();
        trim_fraction(&mut mantissa);
        let exp: i32 = s[e + 1..].parse().unwrap_or(0);
        s = format!("{mantissa}e{exp:+}");
    }
    s.replace('.', ",").replace('-', "−")
}

pub fn format_number(value: f64, grouping: bool) -> String {
    if value.is_nan() {
        return "Invalid input".to_string();
    }
    if value.is_infinite() {
        return "Overflow".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }

    let abs = value.abs();
    let mut text = if abs >= 1.0e16 || abs < 1.0e-16 {
        let mut s = format!("{value:.15e}");
        if let Some(e) = s.find('e') {
            let mut mantissa = s[..e].to_string();
            trim_fraction(&mut mantissa);
            let exp: i32 = s[e + 1..].parse().unwrap_or(0);
            s = format!("{mantissa}e{exp:+}");
        }
        s
    } else {
        let decimals = if abs >= 1.0 {
            let int_digits = abs.log10().floor() as i32 + 1;
            (DISPLAY_DIGITS as i32 - int_digits).max(0) as usize
        } else {
            DISPLAY_DIGITS
        };
        let mut s = format!("{value:.decimals$}");
        trim_fraction(&mut s);
        s
    };

    text = text.replace('.', ",");
    if grouping {
        text = group_digits(&text);
    }
    text.replace("-", "−")
}

fn trim_fraction(s: &mut String) {
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        *s = "0".to_string();
    }
}

fn group_digits(text: &str) -> String {
    let negative = text.starts_with('−') || text.starts_with('-');
    let body = if negative { &text[1..] } else { text };
    let (mantissa, exponent) = match body.find('e') {
        Some(pos) => (&body[..pos], Some(&body[pos..])),
        None => (body, None),
    };
    let (int_part, frac_part) = match mantissa.find(',') {
        Some(pos) => (&mantissa[..pos], Some(&mantissa[pos..])),
        None => (mantissa, None),
    };
    let mut grouped = String::new();
    let len = int_part.len();
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    if let Some(frac) = frac_part {
        grouped.push_str(frac);
    }
    if let Some(exp) = exponent {
        grouped.push_str(exp);
    }
    if negative {
        format!("−{grouped}")
    } else {
        grouped
    }
}

pub fn parse_number(text: &str) -> Option<f64> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            ',' => '.',
            '−' => '-',
            _ => c,
        })
        .collect();
    if cleaned.is_empty() || cleaned == "-" {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

/// Number of significant digits already typed, ignoring leading zeroes and the
/// decimal separator.  The original accepts at most 16.
fn significant_digits(entry: &str) -> usize {
    let mut seen_non_zero = false;
    let mut count = 0;
    for ch in entry.chars() {
        if let Some(d) = ch.to_digit(10) {
            if d != 0 {
                seen_non_zero = true;
            }
            if seen_non_zero {
                count += 1;
            }
        }
    }
    count
}

/// Digits after the decimal separator.  The original accepts at most 16
/// fractional digits even when the leading zeroes make the value smaller than
/// the significant-digit limit.
fn fractional_digits(entry: &str) -> usize {
    match entry.split_once('.') {
        Some((_, frac)) => frac.chars().filter(|c| c.is_ascii_digit()).count(),
        None => 0,
    }
}

fn entry_has_room(entry: &str) -> bool {
    significant_digits(entry) < DISPLAY_DIGITS && fractional_digits(entry) < DISPLAY_DIGITS
}

fn plain(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if abs >= 1.0e16 || abs < 1.0e-16 {
        let mut s = format!("{value:.15e}");
        if let Some(e) = s.find('e') {
            let mut mantissa = s[..e].to_string();
            trim_fraction(&mut mantissa);
            let exp: i32 = s[e + 1..].parse().unwrap_or(0);
            s = format!("{mantissa}e{exp:+}");
        }
        s
    } else {
        let decimals = if abs >= 1.0 {
            let int_digits = abs.log10().floor() as i32 + 1;
            (DISPLAY_DIGITS as i32 - int_digits).max(0) as usize
        } else {
            DISPLAY_DIGITS
        };
        let mut s = format!("{value:.decimals$}");
        trim_fraction(&mut s);
        s
    }
}

/// The standard/scientific expression engine.
pub struct StandardEngine {
    tokens: Vec<Token>,
    entry: String,
    current: f64,
    entering: bool,
    just_evaluated: bool,
    /// Last binary operation applied by `=`, used by the original's
    /// repeat-equals behaviour (`2 + 3 = =` -> 8 -> 11).
    repeat: Option<(Op, f64)>,
    pub error: Option<String>,
    pub history: Vec<String>,
    pub last_expression: String,
}

impl Default for StandardEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardEngine {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            entry: String::new(),
            current: 0.0,
            entering: false,
            just_evaluated: false,
            repeat: None,
            error: None,
            history: Vec::new(),
            last_expression: String::new(),
        }
    }

    pub fn display(&self, grouping: bool) -> String {
        if let Some(err) = &self.error {
            return err.clone();
        }
        if self.entering {
            if self.entry.is_empty() {
                "0".to_string()
            } else {
                format_entry(&self.entry, grouping)
            }
        } else {
            format_number(self.current, grouping)
        }
    }

    pub fn expression(&self, grouping: bool) -> String {
        if self.error.is_some() {
            return String::new();
        }
        let mut text = String::new();
        for token in &self.tokens {
            match token {
                Token::Num(n) => text.push_str(&format_number(*n, grouping)),
                Token::Op(op) => {
                    text.push(' ');
                    text.push_str(op.symbol());
                    text.push(' ');
                }
                Token::LParen => text.push('('),
                Token::RParen => text.push(')'),
            }
        }
        // The original expression line shows only committed tokens; the
        // number currently being typed appears on the main display alone.
        text.trim().to_string()
    }

    pub fn current_value(&self) -> f64 {
        if self.entering {
            parse_number(&self.entry).unwrap_or(0.0)
        } else {
            self.current
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    fn reset_error(&mut self) {
        if self.error.is_some() {
            self.clear_all();
        }
    }

    pub fn digit(&mut self, ch: char) {
        self.reset_error();
        if self.just_evaluated {
            self.tokens.clear();
            self.current = 0.0;
            self.just_evaluated = false;
        }
        if !self.entering {
            self.entry.clear();
            self.entering = true;
        }
        if self.entry == "0" {
            self.entry.clear();
        }
        if entry_has_room(&self.entry) {
            self.entry.push(ch);
        }
    }

    pub fn decimal(&mut self) {
        self.reset_error();
        if self.just_evaluated {
            self.tokens.clear();
            self.current = 0.0;
            self.just_evaluated = false;
        }
        if !self.entering {
            self.entry.clear();
            self.entering = true;
        }
        if !self.entry.contains('.') && !self.entry.contains('e') {
            if self.entry.is_empty() {
                self.entry.push('0');
            }
            self.entry.push('.');
        }
    }

    pub fn backspace(&mut self) {
        self.reset_error();
        if self.entering {
            self.entry.pop();
            if self.entry.is_empty() || self.entry == "-" {
                self.entry.clear();
                self.entering = false;
                self.current = 0.0;
            }
        }
    }

    pub fn clear_entry(&mut self) {
        // CE clears the current operand to zero but keeps the pending
        // expression and the repeat operation. The original displays `0`
        // and evaluates `2 + 3 CE =` as `2 + 0`.
        self.entry = "0".to_string();
        self.entering = true;
        self.error = None;
        self.just_evaluated = false;
    }

    pub fn clear_all(&mut self) {
        self.tokens.clear();
        self.entry.clear();
        self.current = 0.0;
        self.entering = false;
        self.just_evaluated = false;
        self.repeat = None;
        self.error = None;
        self.last_expression.clear();
    }

    pub fn toggle_sign(&mut self) {
        self.reset_error();
        if self.entering {
            if self.entry.starts_with('-') {
                self.entry.remove(0);
            } else if !self.entry.is_empty() {
                self.entry.insert(0, '-');
            }
        } else {
            self.current = -self.current;
        }
    }

    pub fn push_op(&mut self, op: Op) {
        self.reset_error();
        if self.just_evaluated {
            self.tokens.clear();
            self.tokens.push(Token::Num(self.current));
            self.just_evaluated = false;
        } else if self.entering {
            let value = parse_number(&self.entry).unwrap_or(0.0);
            self.tokens.push(Token::Num(value));
            self.current = value;
            self.entry.clear();
            self.entering = false;
        } else if self.tokens.is_empty() {
            self.tokens.push(Token::Num(self.current));
        }

        if matches!(self.tokens.last(), Some(Token::Op(_))) {
            self.tokens.pop();
        }
        self.tokens.push(Token::Op(op));
        self.last_expression.clear();
    }

    pub fn open_paren(&mut self) {
        self.reset_error();
        if self.just_evaluated {
            self.tokens.clear();
            self.just_evaluated = false;
        }
        if self.entering {
            let value = parse_number(&self.entry).unwrap_or(0.0);
            self.tokens.push(Token::Num(value));
            self.entry.clear();
            self.entering = false;
        } else if !self.tokens.is_empty() {
            if let Some(Token::Num(_)) | Some(Token::RParen) = self.tokens.last() {
                self.tokens.push(Token::Op(Op::Mul));
            }
        }
        self.tokens.push(Token::LParen);
    }

    pub fn close_paren(&mut self) {
        self.reset_error();
        if self.entering {
            let value = parse_number(&self.entry).unwrap_or(0.0);
            self.tokens.push(Token::Num(value));
            self.entry.clear();
            self.entering = false;
        } else if self.tokens.is_empty() {
            self.tokens.push(Token::Num(self.current));
        }
        self.tokens.push(Token::RParen);
    }

    pub fn equals(&mut self) {
        if self.error.is_some() {
            return;
        }
        if self.entering {
            let value = parse_number(&self.entry).unwrap_or(0.0);
            self.tokens.push(Token::Num(value));
            self.entry.clear();
            self.entering = false;
        }
        if self.tokens.is_empty() {
            // A bare `=` repeats the last binary operation:
            // `2 + 3 = =` -> 8 -> 11.
            if let Some((op, operand)) = self.repeat {
                self.apply_repeat(op, operand);
            }
            return;
        }

        // After a completed calculation, a freshly typed number becomes the
        // left side of the repeated operation: `2 + 3 = 4 =` -> 7.
        if self.tokens.len() == 1 {
            if let Some(Token::Num(value)) = self.tokens.first() {
                let value = *value;
                if let Some((op, operand)) = self.repeat {
                    self.current = value;
                    self.apply_repeat(op, operand);
                    return;
                }
            }
        }

        // The original uses the value shown after a trailing operator as the
        // missing right operand: `2 + 3 * =` -> 2 + 3 * 3 = 11.
        if matches!(self.tokens.last(), Some(Token::Op(_))) {
            let missing = self.trailing_operand();
            self.tokens.push(Token::Num(missing));
        }

        let open = self
            .tokens
            .iter()
            .filter(|t| matches!(t, Token::LParen))
            .count();
        let close = self
            .tokens
            .iter()
            .filter(|t| matches!(t, Token::RParen))
            .count();
        for _ in close..open {
            self.tokens.push(Token::RParen);
        }
        let expression = self.expression(false);
        match evaluate(&self.tokens) {
            Ok(result) => {
                if let Some(last) = result.last_op {
                    self.repeat = Some(last);
                }
                let value = result.value;
                self.current = value;
                self.tokens.clear();
                self.just_evaluated = true;
                self.last_expression = format!("{expression} =");
                self.history.push(format!(
                    "{} {}",
                    self.last_expression,
                    format_number(value, false)
                ));
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            Err(err) => {
                self.error = Some(err);
                self.tokens.clear();
                self.entering = false;
            }
        }
    }

    /// Applies the remembered binary operation to `self.current` and records
    /// the result as a normal history entry.
    fn apply_repeat(&mut self, op: Op, operand: f64) {
        let left = self.current;
        match op.apply(left, operand) {
            Ok(value) => {
                self.current = value;
                self.tokens.clear();
                self.entry.clear();
                self.entering = false;
                self.just_evaluated = true;
                self.last_expression = format!(
                    "{} {} {} =",
                    format_number(left, false),
                    op.symbol(),
                    format_number(operand, false)
                );
                self.history.push(format!(
                    "{} {}",
                    self.last_expression,
                    format_number(value, false)
                ));
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            Err(err) => {
                self.error = Some(err);
                self.tokens.clear();
                self.entering = false;
            }
        }
    }

    /// Value that the original displays after the trailing operator: all
    /// pending operators that the new operator would commit are applied,
    /// while lower-precedence operations stay pending.
    fn trailing_operand(&self) -> f64 {
        let Some(Token::Op(op)) = self.tokens.last() else {
            return self.current;
        };
        let op = *op;
        let prefix = &self.tokens[..self.tokens.len() - 1];
        let mut values: Vec<f64> = Vec::new();
        let mut ops: Vec<Token> = Vec::new();
        for token in prefix {
            match token {
                Token::Num(value) => values.push(*value),
                Token::LParen => ops.push(Token::LParen),
                Token::RParen => loop {
                    let top = match ops.last() {
                        Some(Token::LParen) => {
                            ops.pop();
                            break;
                        }
                        Some(Token::Op(_)) => true,
                        _ => false,
                    };
                    if !top || pop_apply(&mut values, &mut ops).is_err() {
                        return self.current;
                    }
                },
                Token::Op(next) => {
                    loop {
                        let flush = match ops.last() {
                            Some(Token::Op(top)) => {
                                top.precedence() > next.precedence()
                                    || (top.precedence() == next.precedence()
                                        && !next.right_associative())
                            }
                            _ => false,
                        };
                        if !flush {
                            break;
                        }
                        if pop_apply(&mut values, &mut ops).is_err() {
                            return self.current;
                        }
                    }
                    ops.push(Token::Op(*next));
                }
            }
        }
        loop {
            let flush = match ops.last() {
                Some(Token::Op(top)) => {
                    top.precedence() > op.precedence()
                        || (top.precedence() == op.precedence() && !op.right_associative())
                }
                _ => false,
            };
            if !flush {
                break;
            }
            if pop_apply(&mut values, &mut ops).is_err() {
                return self.current;
            }
        }
        values.last().copied().unwrap_or(self.current)
    }

    pub fn unary<F>(&mut self, name: &str, func: F)
    where
        F: Fn(f64) -> Result<f64, String>,
    {
        if self.error.is_some() {
            return;
        }
        let value = self.current_value();
        match func(value) {
            Ok(result) => {
                let expression = format!("{name}({})", format_number(value, false));
                self.current = result;
                self.entry.clear();
                self.entering = false;
                self.tokens.clear();
                self.just_evaluated = true;
                self.last_expression = format!("{expression} =");
                self.history
                    .push(format!("{expression} = {}", format_number(result, false)));
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            Err(err) => {
                self.error = Some(err);
                self.entering = false;
            }
        }
    }

    pub fn percent(&mut self) {
        self.reset_error();
        let value = self.current_value();
        // Recovered from the original: `%` uses the accumulated left side as
        // its base for every pending operator, not only for `+`/`-`.
        //   200 + 10 % = 220      (200 * 10 / 100 = 20)
        //   200 * 10 % = 4000     (base 200 -> 20, then 200 * 20)
        //   200 / 10 % = 10       (base 200 -> 20, then 200 / 20)
        //   100 + 50 + 10 % = 165 (base 150 -> 15)
        //   50 % = 0              (empty accumulator is 0)
        let mut prefix = self.tokens.clone();
        while matches!(prefix.last(), Some(Token::Op(_))) {
            prefix.pop();
        }
        let left = evaluate(&prefix).map(|result| result.value).unwrap_or(0.0);
        let percent_value = left * value / 100.0;
        self.entry = plain(percent_value);
        self.entering = true;
        self.just_evaluated = false;
    }

    pub fn set_constant(&mut self, value: f64) {
        self.reset_error();
        self.current = value;
        self.entry.clear();
        self.entering = false;
        self.tokens.clear();
        self.just_evaluated = false;
    }

    pub fn recall(&mut self, value: f64) {
        self.reset_error();
        self.current = value;
        self.entry.clear();
        self.entering = false;
        self.just_evaluated = false;
    }

    pub fn exp_entry(&mut self) {
        self.reset_error();
        if self.just_evaluated {
            self.tokens.clear();
            self.current = 0.0;
            self.just_evaluated = false;
        }
        if !self.entering {
            self.entry = plain(self.current);
            self.entering = true;
        }
        if !self.entry.contains('e') && !self.entry.contains('E') {
            if self.entry.is_empty() {
                self.entry.push('1');
            }
            self.entry.push('e');
        }
    }
}

fn format_entry(entry: &str, grouping: bool) -> String {
    let mut text = entry.replace('.', ",");
    if text.contains('e') {
        text = text.replace('-', "−");
        return text;
    }
    if grouping {
        text = group_digits(&text);
    }
    text.replace('-', "−")
}

struct Evaluation {
    value: f64,
    /// The last binary operation applied while evaluating the expression.
    /// The original repeats this operation when `=` is pressed again.
    last_op: Option<(Op, f64)>,
}

fn pop_apply(values: &mut Vec<f64>, ops: &mut Vec<Token>) -> Result<(), String> {
    let Some(Token::Op(op)) = ops.pop() else {
        return Err("Invalid input".to_string());
    };
    let b = values.pop().ok_or_else(|| "Invalid input".to_string())?;
    let a = values.pop().ok_or_else(|| "Invalid input".to_string())?;
    values.push(op.apply(a, b)?);
    Ok(())
}

fn evaluate(tokens: &[Token]) -> Result<Evaluation, String> {
    let mut output: Vec<Token> = Vec::new();
    let mut ops: Vec<Token> = Vec::new();
    for token in tokens {
        match token {
            Token::Num(_) => output.push(token.clone()),
            Token::Op(op) => {
                while let Some(Token::Op(top)) = ops.last() {
                    if top.precedence() > op.precedence()
                        || (top.precedence() == op.precedence() && !op.right_associative())
                    {
                        output.push(ops.pop().unwrap());
                    } else {
                        break;
                    }
                }
                ops.push(Token::Op(*op));
            }
            Token::LParen => ops.push(Token::LParen),
            Token::RParen => {
                let mut found = false;
                while let Some(top) = ops.pop() {
                    if matches!(top, Token::LParen) {
                        found = true;
                        break;
                    }
                    output.push(top);
                }
                if !found {
                    return Err("Mismatched ')'".to_string());
                }
            }
        }
    }
    while let Some(top) = ops.pop() {
        if matches!(top, Token::LParen) {
            return Err("Mismatched ')'".to_string());
        }
        output.push(top);
    }

    let mut stack: Vec<f64> = Vec::new();
    let mut last_op = None;
    for token in output {
        match token {
            Token::Num(n) => stack.push(n),
            Token::Op(op) => {
                let b = stack.pop().ok_or_else(|| "Invalid input".to_string())?;
                let a = stack.pop().ok_or_else(|| "Invalid input".to_string())?;
                stack.push(op.apply(a, b)?);
                last_op = Some((op, b));
            }
            _ => return Err("Invalid input".to_string()),
        }
    }
    if stack.len() != 1 {
        return Err("Invalid input".to_string());
    }
    Ok(Evaluation {
        value: stack[0],
        last_op,
    })
}

// ---------------------------------------------------------------------------
// Programmer view
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Lsh,
    Rsh,
    Rol,
    Ror,
}

impl ProgOp {
    fn symbol(self) -> &'static str {
        match self {
            ProgOp::Add => "+",
            ProgOp::Sub => "−",
            ProgOp::Mul => "×",
            ProgOp::Div => "÷",
            ProgOp::Mod => "Mod",
            ProgOp::And => "And",
            ProgOp::Or => "Or",
            ProgOp::Xor => "Xor",
            ProgOp::Lsh => "Lsh",
            ProgOp::Rsh => "Rsh",
            ProgOp::Rol => "RoL",
            ProgOp::Ror => "RoR",
        }
    }

    /// Recovered from the original: `Or`/`Xor` bind loosest, then `And`,
    /// then `+`/`-`, then `*`/`/`/`Mod` and the shift/rotate operators.
    fn precedence(self) -> u8 {
        match self {
            ProgOp::Or | ProgOp::Xor => 1,
            ProgOp::And => 2,
            ProgOp::Add | ProgOp::Sub => 3,
            ProgOp::Mul
            | ProgOp::Div
            | ProgOp::Mod
            | ProgOp::Lsh
            | ProgOp::Rsh
            | ProgOp::Rol
            | ProgOp::Ror => 4,
        }
    }

    fn apply(self, a: u64, b: u64, word: u8) -> Result<u64, String> {
        let mask = prog_mask(word);
        let a = a & mask;
        let b = b & mask;
        let result = match self {
            ProgOp::Add => a.wrapping_add(b),
            ProgOp::Sub => a.wrapping_sub(b),
            ProgOp::Mul => a.wrapping_mul(b),
            ProgOp::Div => {
                if b == 0 {
                    return Err("Cannot divide by zero".to_string());
                }
                a / b
            }
            ProgOp::Mod => {
                if b == 0 {
                    return Err("Cannot divide by zero".to_string());
                }
                a % b
            }
            ProgOp::And => a & b,
            ProgOp::Or => a | b,
            ProgOp::Xor => a ^ b,
            ProgOp::Lsh => {
                if b >= word as u64 {
                    0
                } else {
                    a << b
                }
            }
            ProgOp::Rsh => {
                if b >= word as u64 {
                    0
                } else {
                    a >> b
                }
            }
            ProgOp::Rol => {
                let bits = word as u32;
                let shift = (b % bits as u64) as u32;
                if shift == 0 {
                    a
                } else {
                    ((a << shift) | (a >> (bits - shift))) & mask
                }
            }
            ProgOp::Ror => {
                let bits = word as u32;
                let shift = (b % bits as u64) as u32;
                if shift == 0 {
                    a
                } else {
                    ((a >> shift) | (a << (bits - shift))) & mask
                }
            }
        };
        Ok(result & mask)
    }
}

fn prog_mask(word: u8) -> u64 {
    if word >= 64 {
        u64::MAX
    } else {
        (1u64 << word) - 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgToken {
    Value(u64),
    Op(ProgOp),
}

struct ProgEvaluation {
    value: u64,
    last_op: Option<(ProgOp, u64)>,
}

fn prog_pop_apply(values: &mut Vec<u64>, ops: &mut Vec<ProgOp>, word: u8) -> Result<(), String> {
    let Some(op) = ops.pop() else {
        return Err("Invalid input".to_string());
    };
    let b = values.pop().ok_or_else(|| "Invalid input".to_string())?;
    let a = values.pop().ok_or_else(|| "Invalid input".to_string())?;
    values.push(op.apply(a, b, word)?);
    Ok(())
}

fn prog_evaluate(tokens: &[ProgToken], word: u8) -> Result<ProgEvaluation, String> {
    let mut output: Vec<ProgToken> = Vec::new();
    let mut ops: Vec<ProgOp> = Vec::new();
    for token in tokens {
        match token {
            ProgToken::Value(_) => output.push(*token),
            ProgToken::Op(op) => {
                loop {
                    let flush = match ops.last() {
                        Some(top) => top.precedence() >= op.precedence(),
                        None => false,
                    };
                    if !flush {
                        break;
                    }
                    output.push(ProgToken::Op(ops.pop().unwrap()));
                }
                ops.push(*op);
            }
        }
    }
    while let Some(op) = ops.pop() {
        output.push(ProgToken::Op(op));
    }

    let mask = prog_mask(word);
    let mut stack: Vec<u64> = Vec::new();
    let mut last_op = None;
    for token in output {
        match token {
            ProgToken::Value(value) => stack.push(value & mask),
            ProgToken::Op(op) => {
                let b = stack.pop().ok_or_else(|| "Invalid input".to_string())?;
                let a = stack.pop().ok_or_else(|| "Invalid input".to_string())?;
                stack.push(op.apply(a, b, word)?);
                last_op = Some((op, b));
            }
        }
    }
    if stack.len() != 1 {
        return Err("Invalid input".to_string());
    }
    Ok(ProgEvaluation {
        value: stack[0],
        last_op,
    })
}

/// Value the original shows after pressing `op` on a complete prefix.
fn prog_display_after(prefix: &[ProgToken], op: ProgOp, word: u8) -> u64 {
    let mask = prog_mask(word);
    let mut values: Vec<u64> = Vec::new();
    let mut ops: Vec<ProgOp> = Vec::new();
    for token in prefix {
        match token {
            ProgToken::Value(value) => values.push(*value & mask),
            ProgToken::Op(next) => {
                loop {
                    let flush = match ops.last() {
                        Some(top) => top.precedence() >= next.precedence(),
                        None => false,
                    };
                    if !flush {
                        break;
                    }
                    if prog_pop_apply(&mut values, &mut ops, word).is_err() {
                        return values.last().copied().unwrap_or(0);
                    }
                }
                ops.push(*next);
            }
        }
    }
    loop {
        let flush = match ops.last() {
            Some(top) => top.precedence() >= op.precedence(),
            None => false,
        };
        if !flush {
            break;
        }
        if prog_pop_apply(&mut values, &mut ops, word).is_err() {
            break;
        }
    }
    values.last().copied().unwrap_or(0)
}

fn programmer_digit_value(ch: char) -> Option<u8> {
    ch.to_digit(16).map(|value| value as u8)
}

pub fn digit_allowed(base: u8, ch: char) -> bool {
    match programmer_digit_value(ch) {
        Some(value) => value < base,
        None => false,
    }
}

pub struct ProgrammerState {
    pub base: u8,
    pub word: u8,
    pub value: u64,
    entry: String,
    entering: bool,
    tokens: Vec<ProgToken>,
    repeat: Option<(ProgOp, u64)>,
    pub history: Vec<String>,
    pub error: Option<String>,
    pub just_evaluated: bool,
}

impl Default for ProgrammerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgrammerState {
    pub fn new() -> Self {
        Self {
            base: 10,
            word: 64,
            value: 0,
            entry: String::new(),
            entering: false,
            tokens: Vec::new(),
            repeat: None,
            history: Vec::new(),
            error: None,
            just_evaluated: false,
        }
    }

    pub fn mask(&self) -> u64 {
        prog_mask(self.word)
    }

    pub fn display(&self) -> String {
        if let Some(err) = &self.error {
            return err.clone();
        }
        if self.entering && !self.entry.is_empty() {
            group_prog(&self.entry, self.base)
        } else {
            format_prog(self.value, self.base, self.word)
        }
    }

    /// The original programmer view has no expression line above the display.
    pub fn expression(&self) -> String {
        String::new()
    }

    pub fn clear_all(&mut self) {
        self.value = 0;
        self.entry.clear();
        self.entering = false;
        self.tokens.clear();
        self.repeat = None;
        self.error = None;
        self.just_evaluated = false;
    }

    pub fn clear_entry(&mut self) {
        // CE clears the operand to zero but keeps the pending expression and
        // the repeat operation, exactly like the standard engine.
        self.entry = "0".to_string();
        self.entering = true;
        self.value = 0;
        self.error = None;
        self.just_evaluated = false;
    }

    pub fn set_base(&mut self, base: u8) {
        if self.entering {
            self.tokens.push(ProgToken::Value(self.value & self.mask()));
            self.entry.clear();
        }
        self.base = base;
        self.entering = false;
        self.just_evaluated = false;
        self.error = None;
    }

    pub fn set_word(&mut self, word: u8) {
        if self.entering {
            self.tokens.push(ProgToken::Value(self.value & self.mask()));
            self.entry.clear();
        }
        self.word = word;
        let mask = self.mask();
        self.value &= mask;
        for token in &mut self.tokens {
            if let ProgToken::Value(value) = token {
                *value &= mask;
            }
        }
        if let Some((_, operand)) = &mut self.repeat {
            *operand &= mask;
        }
        self.entering = false;
        self.just_evaluated = false;
    }

    pub fn digit(&mut self, ch: char) {
        let digit = ch.to_digit(16).unwrap_or(99);
        if digit >= self.base as u32 {
            return;
        }
        if self.error.is_some() {
            self.clear_all();
        }
        if self.just_evaluated {
            self.tokens.clear();
            self.value = 0;
            self.just_evaluated = false;
        }
        if !self.entering {
            self.entry.clear();
            self.entering = true;
        }
        if self.entry == "0" {
            self.entry.clear();
        }
        // A fixed 16-character cap truncates decimal QWORD input (19 digits)
        // and even binary input (up to 64 bits). Accept a digit only while the
        // resulting value still fits the selected word size.
        let mut candidate = self.entry.clone();
        candidate.push(ch.to_ascii_uppercase());
        // Decimal display is signed, so decimal entry is limited to the
        // positive signed range; hex/octal/binary may use the full bit pattern.
        let max_entry = if self.base == 10 {
            if self.word >= 64 {
                i64::MAX as u64
            } else {
                (1u64 << (self.word - 1)) - 1
            }
        } else {
            self.mask()
        };
        if let Ok(v) = u64::from_str_radix(&candidate, self.base as u32) {
            if v <= max_entry {
                self.entry = candidate;
                self.value = v & self.mask();
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.error.is_some() {
            self.clear_entry();
            return;
        }
        if self.entering {
            self.entry.pop();
            if self.entry.is_empty() {
                self.value = 0;
            } else if let Ok(v) = u64::from_str_radix(&self.entry, self.base as u32) {
                self.value = v & self.mask();
            }
        }
    }

    /// Commits the current display value as the pending operand of an
    /// expression. Used by unary operations so `2 + 3 sqrt =` computes
    /// `2 + 1`, not `2` or `sqrt(2 + 3)`.
    fn commit_unary(&mut self) {
        if self.tokens.is_empty() {
            return;
        }
        let value = self.value & self.mask();
        match self.tokens.last_mut() {
            Some(ProgToken::Op(_)) => self.tokens.push(ProgToken::Value(value)),
            Some(ProgToken::Value(slot)) => *slot = value,
            None => {}
        }
        self.just_evaluated = false;
    }

    pub fn toggle_sign(&mut self) {
        let bits = self.word;
        let masked = self.value & self.mask();
        if bits == 64 {
            self.value = (!masked).wrapping_add(1);
        } else {
            let mask = self.mask();
            self.value = (!masked).wrapping_add(1) & mask;
        }
        self.entry.clear();
        self.entering = false;
        self.commit_unary();
    }

    pub fn not(&mut self) {
        let before = self.value;
        self.value = !self.value & self.mask();
        self.entry.clear();
        self.entering = false;
        self.commit_unary();
        if self.tokens.is_empty() {
            self.just_evaluated = true;
        }
        self.history.push(format!(
            "Not {} = {}",
            format_prog(before, self.base, self.word),
            format_prog(self.value, self.base, self.word)
        ));
    }

    pub fn push_op(&mut self, op: ProgOp) {
        if self.error.is_some() {
            self.clear_all();
        }
        if self.entering {
            let value = self.value & self.mask();
            self.tokens.push(ProgToken::Value(value));
            self.entry.clear();
            self.entering = false;
        }
        if self.just_evaluated {
            self.tokens.clear();
            self.tokens.push(ProgToken::Value(self.value & self.mask()));
            self.just_evaluated = false;
        } else if self.tokens.is_empty() {
            self.tokens.push(ProgToken::Value(self.value & self.mask()));
        }

        let mut replaced = false;
        if matches!(self.tokens.last(), Some(ProgToken::Op(_))) {
            replaced = true;
            let prefix_len = self.tokens.len() - 1;
            let prefix_value = prog_evaluate(&self.tokens[..prefix_len], self.word)
                .ok()
                .map(|result| result.value);
            if prefix_value == Some(self.value & self.mask()) {
                // The display already reflects the whole committed prefix;
                // collapse it so a higher-precedence operator cannot
                // reassociate the part that was already evaluated.
                let display = self.value & self.mask();
                self.tokens.clear();
                self.tokens.push(ProgToken::Value(display));
            } else {
                self.tokens.pop();
            }
        }
        self.tokens.push(ProgToken::Op(op));
        if !replaced {
            let prefix_len = self.tokens.len() - 1;
            self.value = prog_display_after(&self.tokens[..prefix_len], op, self.word);
        }
        self.entering = false;
        self.just_evaluated = false;
    }

    pub fn equals(&mut self) {
        if self.error.is_some() {
            return;
        }
        if self.entering {
            let value = self.value & self.mask();
            self.tokens.push(ProgToken::Value(value));
            self.entry.clear();
            self.entering = false;
        }
        if self.tokens.is_empty() {
            if let Some((op, operand)) = self.repeat {
                self.apply_repeat(op, operand);
            }
            return;
        }
        if self.tokens.len() == 1 {
            if let Some(ProgToken::Value(value)) = self.tokens.first() {
                let value = *value & self.mask();
                if let Some((op, operand)) = self.repeat {
                    self.value = value;
                    self.apply_repeat(op, operand);
                    return;
                }
            }
        }
        if matches!(self.tokens.last(), Some(ProgToken::Op(_))) {
            let missing = self.value & self.mask();
            self.tokens.push(ProgToken::Value(missing));
        }
        let expression = self.history_expression();
        match prog_evaluate(&self.tokens, self.word) {
            Ok(result) => {
                if let Some(last) = result.last_op {
                    self.repeat = Some(last);
                }
                self.value = result.value;
                self.tokens.clear();
                self.just_evaluated = true;
                self.history.push(format!(
                    "{} = {}",
                    expression,
                    format_prog(self.value, self.base, self.word)
                ));
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            Err(err) => {
                self.error = Some(err);
                self.tokens.clear();
                self.entering = false;
            }
        }
    }

    fn history_expression(&self) -> String {
        let mut text = String::new();
        for token in &self.tokens {
            match token {
                ProgToken::Value(value) => {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(&format_prog(*value, self.base, self.word));
                }
                ProgToken::Op(op) => {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(op.symbol());
                }
            }
        }
        text
    }

    fn apply_repeat(&mut self, op: ProgOp, operand: u64) {
        let left = self.value & self.mask();
        match op.apply(left, operand, self.word) {
            Ok(value) => {
                self.value = value;
                self.tokens.clear();
                self.entry.clear();
                self.entering = false;
                self.just_evaluated = true;
                self.history.push(format!(
                    "{} {} {} = {}",
                    format_prog(left, self.base, self.word),
                    op.symbol(),
                    format_prog(operand, self.base, self.word),
                    format_prog(value, self.base, self.word)
                ));
                if self.history.len() > 200 {
                    self.history.remove(0);
                }
            }
            Err(err) => {
                self.error = Some(err);
                self.tokens.clear();
                self.entering = false;
            }
        }
    }

    pub fn toggle_bit(&mut self, bit: u32) {
        if bit < self.word as u32 {
            self.value ^= 1u64 << bit;
            self.value &= self.mask();
            self.entry.clear();
            self.entering = false;
            self.commit_unary();
            self.just_evaluated = false;
        }
    }

    pub fn recall(&mut self, value: u64) {
        self.value = value & self.mask();
        self.entry.clear();
        self.entering = false;
        self.error = None;
        self.commit_unary();
        self.just_evaluated = false;
    }

    pub fn sqrt(&mut self) {
        self.value = (self.value as f64).sqrt() as u64 & self.mask();
        self.entry.clear();
        self.entering = false;
        self.commit_unary();
        if self.tokens.is_empty() {
            self.just_evaluated = true;
        }
    }

    pub fn reciprocal(&mut self) {
        if self.value == 0 {
            self.error = Some("Cannot divide by zero".to_string());
        } else {
            self.value = 1 / self.value;
            self.entry.clear();
            self.entering = false;
            self.commit_unary();
            if self.tokens.is_empty() {
                self.just_evaluated = true;
            }
        }
    }
}

pub fn format_prog(value: u64, base: u8, word: u8) -> String {
    match base {
        16 => group_prog(&format!("{value:X}"), 16),
        8 => group_prog(&format!("{value:o}"), 8),
        2 => group_prog(&format!("{value:b}"), 2),
        _ => {
            let signed = if word >= 64 {
                value as i64
            } else {
                let sign = 1u64 << (word - 1);
                if value & sign != 0 {
                    (value as i64) - (1i64 << word)
                } else {
                    value as i64
                }
            };
            let s = signed.to_string();
            group_signed(&s)
        }
    }
}

fn group_prog(text: &str, base: u8) -> String {
    // Decimal and octal are grouped in threes, hex and binary in nibbles.
    let group = match base {
        10 | 8 => 3,
        _ => 4,
    };
    let mut out = String::new();
    let len = text.len();
    for (i, ch) in text.chars().enumerate() {
        if i > 0 && (len - i) % group == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn group_signed(text: &str) -> String {
    let negative = text.starts_with('-');
    let body = if negative { &text[1..] } else { text };
    let mut out = String::new();
    let len = body.len();
    for (i, ch) in body.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    if negative {
        format!("−{out}")
    } else {
        out
    }
}

// ---------------------------------------------------------------------------
// Statistics view
// ---------------------------------------------------------------------------

pub struct StatsState {
    pub data: Vec<f64>,
    pub history: Vec<String>,
    pub entry: String,
    pub value: f64,
    pub entering: bool,
    pub result_text: String,
    pub error: Option<String>,
}

impl Default for StatsState {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsState {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            history: Vec::new(),
            entry: String::new(),
            value: 0.0,
            entering: false,
            result_text: String::new(),
            error: None,
        }
    }

    pub fn display(&self) -> String {
        if let Some(err) = &self.error {
            return err.clone();
        }
        if !self.result_text.is_empty() {
            return self.result_text.clone();
        }
        if self.entering {
            format_entry(&self.entry, false)
        } else {
            format_number(self.value, false)
        }
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }

    pub fn digit(&mut self, ch: char) {
        self.error = None;
        self.result_text.clear();
        if !self.entering {
            self.entry.clear();
            self.entering = true;
        }
        if self.entry == "0" {
            self.entry.clear();
        }
        if entry_has_room(&self.entry) {
            self.entry.push(ch);
        }
    }

    pub fn decimal(&mut self) {
        self.error = None;
        self.result_text.clear();
        if !self.entering {
            self.entry.clear();
            self.entering = true;
        }
        if !self.entry.contains('.') {
            if self.entry.is_empty() {
                self.entry.push('0');
            }
            self.entry.push('.');
        }
    }

    pub fn backspace(&mut self) {
        if self.entering {
            self.entry.pop();
            if self.entry.is_empty() {
                self.entering = false;
                self.value = 0.0;
            }
        }
    }

    pub fn clear_entry(&mut self) {
        self.entry.clear();
        self.entering = false;
        self.value = 0.0;
        self.result_text.clear();
        self.error = None;
    }

    pub fn toggle_sign(&mut self) {
        if self.entering {
            if self.entry.starts_with('-') {
                self.entry.remove(0);
            } else if !self.entry.is_empty() {
                self.entry.insert(0, '-');
            }
        } else {
            self.value = -self.value;
        }
    }

    pub fn current_value(&self) -> f64 {
        if self.entering {
            parse_number(&self.entry).unwrap_or(0.0)
        } else {
            self.value
        }
    }

    fn entry_value(&self) -> f64 {
        self.current_value()
    }

    pub fn exp_entry(&mut self) {
        self.result_text.clear();
        if !self.entering {
            self.entry = plain(self.value);
            self.entering = true;
        }
        if !self.entry.contains('e') && !self.entry.contains('E') {
            if self.entry.is_empty() {
                self.entry.push('1');
            }
            self.entry.push('e');
        }
    }

    pub fn add_data(&mut self) {
        self.error = None;
        let value = self.entry_value();
        if !value.is_finite() {
            self.error = Some("Invalid input".to_string());
            return;
        }
        self.data.push(value);
        self.history
            .push(format!("Add Data {}", format_number(value, false)));
        self.entry.clear();
        self.entering = false;
        self.value = 0.0;
        self.result_text.clear();
    }

    pub fn clear_dataset(&mut self) {
        self.data.clear();
        self.history.clear();
        self.clear_entry();
    }

    pub fn sum(&self) -> Option<f64> {
        if self.data.is_empty() {
            None
        } else {
            Some(self.data.iter().sum())
        }
    }

    pub fn mean(&self) -> Option<f64> {
        self.sum().map(|s| s / self.data.len() as f64)
    }

    pub fn sum_squares(&self) -> Option<f64> {
        if self.data.is_empty() {
            None
        } else {
            Some(self.data.iter().map(|v| v * v).sum())
        }
    }

    pub fn mean_squares(&self) -> Option<f64> {
        self.sum_squares().map(|s| s / self.data.len() as f64)
    }

    pub fn std_dev_population(&self) -> Option<f64> {
        let mean = self.mean()?;
        let variance = self
            .data
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / self.data.len() as f64;
        Some(variance.sqrt())
    }

    pub fn std_dev_sample(&self) -> Option<f64> {
        if self.data.len() < 2 {
            return None;
        }
        let mean = self.mean()?;
        let variance = self
            .data
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / (self.data.len() - 1) as f64;
        Some(variance.sqrt())
    }

    pub fn show(&mut self, label: &str, value: Option<f64>) {
        self.result_text.clear();
        self.error = None;
        match value {
            Some(v) => {
                let text = format_number(v, false);
                self.result_text = text.clone();
                self.value = v;
                self.entering = false;
                self.history.push(format!("{label} = {text}"));
            }
            None => {
                self.error = Some("Enter data to calculate".to_string());
            }
        }
    }

    pub fn recall(&mut self, value: f64) {
        self.value = value;
        self.entry.clear();
        self.entering = false;
        self.result_text.clear();
        self.error = None;
    }
}

// ---------------------------------------------------------------------------
// Scientific helpers
// ---------------------------------------------------------------------------

pub fn apply_angle(value: f64, angle: AngleMode, inverse: bool, kind: &str) -> Result<f64, String> {
    let radians = match angle {
        AngleMode::Degrees => {
            if inverse {
                value
            } else {
                value.to_radians()
            }
        }
        AngleMode::Radians => value,
        AngleMode::Gradians => {
            if inverse {
                value
            } else {
                value * std::f64::consts::PI / 200.0
            }
        }
    };
    let result = match (kind, inverse) {
        ("sin", false) => radians.sin(),
        ("cos", false) => radians.cos(),
        ("tan", false) => radians.tan(),
        ("sin", true) => radians.asin(),
        ("cos", true) => radians.acos(),
        ("tan", true) => radians.atan(),
        _ => return Err("Invalid input".to_string()),
    };
    if result.is_nan() {
        return Err("Invalid input".to_string());
    }
    let result = match angle {
        AngleMode::Degrees if inverse => result.to_degrees(),
        AngleMode::Gradians if inverse => result * 200.0 / std::f64::consts::PI,
        _ => result,
    };
    Ok(result)
}

pub fn factorial(value: f64) -> Result<f64, String> {
    if value < 0.0 || value.fract() != 0.0 || value > 170.0 {
        return Err("Invalid input".to_string());
    }
    let mut result = 1.0;
    let n = value as u64;
    for i in 1..=n {
        result *= i as f64;
        if result.is_infinite() {
            return Err("Overflow".to_string());
        }
    }
    Ok(result)
}

pub fn dms(value: f64) -> Result<f64, String> {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let abs = value.abs();
    let degrees = abs.floor();
    let minutes_total = (abs - degrees) * 60.0;
    let minutes = minutes_total.floor();
    let seconds = (minutes_total - minutes) * 60.0;
    Ok(sign * (degrees + minutes / 100.0 + seconds / 10000.0))
}
