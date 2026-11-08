use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use crate::actions::{programmer_keys, Action, MemAction};
use crate::engine::{digit_allowed, Mode, ProgrammerState};
use crate::ui::{metrics, Rect, TextAlign, Ui};

use super::{Shared, View};

const BIT_PANEL_H: f32 = metrics::PROGRAMMER_BITS_H;
const BIT_GLYPH_INSET: f32 = 12.0;
const BIT_CELL_H: f32 = 20.0;
const BIT_ROW_GAP: f32 = 6.0;
const BIT_EDGE_LABEL_W: f32 = 24.0;
const BIT_EDGE_PAD: f32 = 4.0;
const BIT_GROUP_GAP: f32 = 14.0;

const BASE_PANEL_W: f32 = metrics::PROGRAMMER_BASE_W;
const COLUMN_GAP: f32 = metrics::COLUMN_GAP;

fn selector_layout(panel: Rect, scale: f32) -> (Rect, Vec<Rect>) {
    let s = |v: f32| crate::ui::dpi::dp_to_px(v, scale);
    let inner = panel.inset(s(metrics::PAD_PANEL));
    let title = Rect::new(inner.x, inner.y, inner.w, s(metrics::FIELD_LABEL_H));
    let gap = s(metrics::SELECTOR_GAP);
    let top = title.bottom() + gap;
    let available = inner.bottom() - top - gap * 3;
    let rows = (0..4)
        .map(|i| {
            let start = available * i / 4;
            let end = available * (i + 1) / 4;
            Rect::new(inner.x, top + start + gap * i, inner.w, end - start)
        })
        .collect();
    (title, rows)
}

pub struct ProgrammerView {
    prog: ProgrammerState,
}

impl ProgrammerView {
    pub fn new() -> Self {
        Self {
            prog: ProgrammerState::new(),
        }
    }

    fn apply(&mut self, action: Action, shared: &mut Shared) {
        match action {
            Action::ProgDigit(ch) if digit_allowed(self.prog.base, ch) => self.prog.digit(ch),
            Action::ProgDigit(_) => {}
            Action::ProgBin(op) => self.prog.push_op(op),
            Action::ProgEquals => self.prog.equals(),
            Action::ProgNot => self.prog.not(),
            Action::ProgSign => self.prog.toggle_sign(),
            Action::ProgBackspace => self.prog.backspace(),
            Action::ProgClearEntry => self.prog.clear_entry(),
            Action::ProgClear => self.prog.clear_all(),
            Action::ProgSqrt => self.prog.sqrt(),
            Action::ProgRecip => self.prog.reciprocal(),
            Action::ProgBase(base) => self.prog.set_base(base),
            Action::ProgWord(word) => self.prog.set_word(word),
            Action::ProgBit(bit) => self.prog.toggle_bit(bit),
            Action::Mem(MemAction::Clear) => shared.memory = 0.0,
            Action::Mem(MemAction::Recall) => self.prog.recall(shared.memory as u64),
            Action::Mem(MemAction::Store) => shared.memory = self.prog.value as f64,
            Action::Mem(MemAction::Add) => shared.memory += self.prog.value as f64,
            Action::Mem(MemAction::Sub) => shared.memory -= self.prog.value as f64,
            Action::ParenOpen | Action::ParenClose => {}
            _ => {}
        }
    }

    fn bit_panel(ui: &mut Ui, rect: Rect, prog: &ProgrammerState) {
        ui.surface(
            rect,
            ui.s(metrics::RADIUS_CONTROL),
            ui.palette.display_bg,
            ui.palette.border,
        );

        let rows = 2u32;
        let per_row = 32u32;
        let groups = (per_row / 8) as i32;

        let cell_h = ui.s(BIT_CELL_H);
        let row_gap = ui.s(BIT_ROW_GAP);
        let block_h = rows as i32 * cell_h + (rows as i32 - 1) * row_gap;
        let top = rect.y + (rect.h - block_h) / 2;

        let pad = ui.s(BIT_GLYPH_INSET);
        let edge_w = ui.s(BIT_EDGE_LABEL_W);
        let edge_pad = ui.s(BIT_EDGE_PAD);
        let group_gap = ui.s(BIT_GROUP_GAP);
        let bit_x = rect.x + pad + edge_w;
        let bit_w = rect.w - (pad + edge_w) * 2;
        let full_cell = (bit_w - (groups - 1) * group_gap) / per_row as i32;
        let cell_w = full_cell.max(1);
        let row_w = per_row as i32 * cell_w + (groups - 1) * group_gap;
        let start_x = bit_x + (bit_w - row_w) / 2;
        let active_bits = prog.word as u32;

        for row in 0..rows {
            let high = 63 - row * per_row;
            let low = high + 1 - per_row;
            let y = top + row as i32 * (cell_h + row_gap);
            let label_w = edge_w - edge_pad;
            ui.text(
                Rect::new(rect.x + pad, y, label_w, cell_h),
                &high.to_string(),
                ui.fonts.small,
                ui.palette.muted,
                TextAlign::Left,
            );
            ui.text(
                Rect::new(rect.right() - pad - label_w, y, label_w, cell_h),
                &low.to_string(),
                ui.fonts.small,
                ui.palette.muted,
                TextAlign::Right,
            );

            for i in 0..per_row {
                let group = (i / 8) as i32;
                let in_group = (i % 8) as i32;
                let x = start_x + group * (8 * cell_w + group_gap) + in_group * cell_w;
                let bit = high - i;
                let cell = Rect::new(x, y, cell_w, cell_h);
                if bit >= active_bits {
                    continue;
                }
                let on = (prog.value >> bit) & 1 == 1;
                ui.bit(cell, on, bit);
            }

            for group in 1..groups {
                let x = start_x + group * 8 * cell_w + (group - 1) * group_gap + group_gap / 2;
                let inset = ui.s(2.0);
                ui.canvas
                    .vline(x, y + inset, y + cell_h - inset, ui.palette.panel_border);
            }
        }
    }
}

impl View for ProgrammerView {
    fn mode(&self) -> Mode {
        Mode::Programmer
    }

    fn on_enter(&mut self, _shared: &mut Shared) {}

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let t = shared.text();
        let client = shared.client;
        let m = ui.s(metrics::MARGIN);
        let gap = ui.s(metrics::GAP);
        let display = Rect::new(
            client.x + m,
            client.y + m,
            client.w - m * 2,
            ui.s(metrics::DISPLAY_H),
        );
        let text = self.display(shared);
        let expression = self.prog.expression();
        let inner = display.inset(ui.s(metrics::PAD_DISPLAY));
        if self.prog.base == 2 && ui.canvas.measure(&text, ui.fonts.display_small) > inner.w {
            ui.surface(
                display,
                ui.s(metrics::RADIUS_CONTROL),
                ui.palette.display_bg,
                ui.palette.border,
            );
            if !expression.is_empty() {
                ui.caption(
                    Rect::new(inner.x, inner.y, inner.w, ui.s(metrics::EXPRESSION_H)),
                    &expression,
                );
            }
            let groups: Vec<&str> = text.split_whitespace().collect();
            let split = groups.len().saturating_sub(8);
            let first = groups[..split].join(" ");
            let second = groups[split..].join(" ");
            let h = (inner.h - ui.s(metrics::EXPRESSION_H)).max(2) / 2;
            let top = inner.bottom() - h * 2;
            let color = if self.prog.error.is_some() {
                ui.palette.error
            } else {
                ui.palette.text
            };
            ui.text(
                Rect::new(inner.x, top, inner.w, h),
                &first,
                ui.fonts.display_small,
                color,
                TextAlign::Right,
            );
            ui.text(
                Rect::new(inner.x, top + h, inner.w, h),
                &second,
                ui.fonts.display_small,
                color,
                TextAlign::Right,
            );
        } else {
            ui.display(display, &text, &expression, self.prog.error.is_some());
        }
        let bits = Rect::new(
            client.x + m,
            display.bottom() + gap,
            client.w - m * 2,
            ui.s(BIT_PANEL_H),
        );
        Self::bit_panel(ui, bits, &self.prog);

        let top = bits.bottom() + gap;
        let group_gap = ui.s(metrics::GAP_BLOCK);
        let group_h = (ui.s(metrics::KEYPAD_H) - group_gap) / 2;
        let base = Rect::new(client.x + m, top, ui.s(BASE_PANEL_W), group_h);
        ui.panel(base);
        let (title, rows) = selector_layout(base, shared.scale);
        ui.form_label(title, t.base);
        for (cell, (base_value, label)) in
            rows.into_iter()
                .zip([(16, t.hex), (10, t.dec), (8, t.oct), (2, t.bin)])
        {
            ui.selector_item(
                cell,
                label,
                Action::ProgBase(base_value),
                self.prog.base == base_value,
            );
        }

        let word_panel = Rect::new(
            base.x,
            base.bottom() + group_gap,
            base.w,
            top + ui.s(metrics::KEYPAD_H) - base.bottom() - group_gap,
        );
        ui.panel(word_panel);
        let (title, rows) = selector_layout(word_panel, shared.scale);
        ui.form_label(title, t.word_size);
        for (cell, (word, label)) in
            rows.into_iter()
                .zip([(64, t.qword), (32, t.dword), (16, t.word), (8, t.byte)])
        {
            ui.selector_item(cell, label, Action::ProgWord(word), self.prog.word == word);
        }

        let key_x = base.right() + ui.s(COLUMN_GAP);
        let key_w = ui.s(metrics::KEYPAD_WIDE_W).min(client.right() - m - key_x);
        let mut keys = programmer_keys();
        for key in &mut keys {
            key.enabled = match key.action {
                Action::ProgDigit(ch) => digit_allowed(self.prog.base, ch),
                Action::None => false,
                _ => true,
            };
        }
        ui.keypad(
            Rect::new(key_x, top, key_w, ui.s(metrics::KEYPAD_H)),
            &keys,
            metrics::KEYPAD_COLS_WIDE,
            metrics::KEYPAD_ROWS,
            ui.s(metrics::GAP_KEYPAD),
        );
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        self.apply(action, shared);
    }

    fn char(&mut self, ch: char, shared: &mut Shared) -> bool {
        let action = match ch {
            '0'..='9' | 'a'..='f' | 'A'..='F' => {
                if !digit_allowed(self.prog.base, ch) {
                    return true;
                }
                Action::ProgDigit(ch)
            }
            '+' => Action::ProgBin(crate::engine::ProgOp::Add),
            '-' => Action::ProgBin(crate::engine::ProgOp::Sub),
            '*' => Action::ProgBin(crate::engine::ProgOp::Mul),
            '/' => Action::ProgBin(crate::engine::ProgOp::Div),
            '%' => Action::ProgBin(crate::engine::ProgOp::Mod),
            '=' => Action::ProgEquals,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn key(&mut self, vk: u16, _ctrl: bool, shared: &mut Shared) -> bool {
        let action = match vk {
            VK_RETURN => Action::ProgEquals,
            VK_BACK => Action::ProgBackspace,
            VK_ESCAPE => Action::ProgClear,
            VK_DELETE => Action::ProgClearEntry,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn display(&self, shared: &Shared) -> String {
        crate::i18n::runtime_error(shared.lang, &self.prog.display())
    }
}

#[cfg(test)]
mod selector_tests {
    use super::*;

    #[test]
    fn independent_groups_keep_equal_padding_at_supported_dpis() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let scale = crate::ui::DpiScale::new(dpi).factor();
            let s = |v: f32| (v * scale).round() as i32;
            let height = s(metrics::KEYPAD_H);
            let gap = s(metrics::GAP_BLOCK);
            let first_h = (height - gap) / 2;
            let first = Rect::new(0, 0, s(BASE_PANEL_W), first_h);
            let second = Rect::new(0, first.bottom() + gap, first.w, height - first_h - gap);
            assert_eq!(second.bottom(), height);
            assert_eq!(second.y - first.bottom(), gap);
            assert!((first.h - second.h).abs() <= 1);
            for panel in [first, second] {
                let (title, rows) = selector_layout(panel, scale);
                assert_eq!(title.y - panel.y, s(metrics::PAD_PANEL));
                assert_eq!(
                    panel.bottom() - rows.last().unwrap().bottom(),
                    s(metrics::PAD_PANEL)
                );
                assert!(rows.iter().all(|r| r.h >= s(metrics::SELECTOR_ITEM_H) - 1));
                assert!(rows
                    .windows(2)
                    .all(|r| r[1].y - r[0].bottom() == s(metrics::SELECTOR_GAP)));
            }
        }
    }
}
