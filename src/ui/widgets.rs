use windows_sys::Win32::Graphics::Gdi::HFONT;

use crate::actions::{Action, KeySpec, KeyStyle};

use super::canvas::Canvas;
use super::color::Color;
use super::font::Fonts;
use super::geometry::{Rect, TextAlign};
use super::hit::{self, Hit};
use super::metrics;
use super::palette::Palette;

/// Semantic widget layer. Views never select fonts, colors or paddings by hand.
pub struct Ui<'a> {
    pub canvas: &'a mut Canvas,
    pub fonts: &'a Fonts,
    pub palette: &'a Palette,
    pub scale: f32,
    hot: Option<u64>,
    pressed: Option<u64>,
    hits: Vec<Hit>,
    next_id: u64,
}

impl<'a> Ui<'a> {
    pub fn new(
        canvas: &'a mut Canvas,
        fonts: &'a Fonts,
        palette: &'a Palette,
        scale: f32,
        hot: Option<u64>,
        pressed: Option<u64>,
    ) -> Self {
        Self {
            canvas,
            fonts,
            palette,
            scale,
            hot,
            pressed,
            hits: Vec::new(),
            next_id: 1,
        }
    }

    /// Scales a logical pixel value to the current DPI.
    pub fn s(&self, logical: f32) -> i32 {
        super::dpi::dp_to_px(logical, self.scale)
    }

    pub fn finish(self) -> Vec<Hit> {
        self.hits
    }

    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn hit(&mut self, id: u64, rect: Rect, action: Action) {
        if !matches!(action, Action::None) {
            self.hits.push(Hit { id, rect, action });
        }
    }

    pub fn bit(&mut self, rect: Rect, on: bool, bit: u32) {
        let id = self.id();
        self.hit(id, rect, Action::ProgBit(bit));
        if self.hot == Some(id) || self.pressed == Some(id) {
            self.surface(
                rect,
                self.s(2.0),
                self.palette.selection_bg,
                self.palette.selection_bg,
            );
        }
        self.text(
            rect,
            if on { "1" } else { "0" },
            self.fonts.small,
            if self.hot == Some(id) || self.pressed == Some(id) {
                self.palette.selection_text
            } else if on {
                self.palette.bit_on
            } else {
                self.palette.bit_off
            },
            TextAlign::Center,
        );
    }

    pub fn hit_at(hits: &[Hit], x: i32, y: i32) -> Option<Hit> {
        hit::find(hits, x, y)
    }

    // Surfaces --------------------------------------------------------------

    pub fn surface(&self, rect: Rect, radius: i32, fill: Color, border: Color) {
        self.canvas.round_rect(rect, radius, fill, border);
    }

    pub fn panel(&self, rect: Rect) {
        self.surface(
            rect,
            self.s(metrics::RADIUS_PANEL),
            self.palette.panel,
            self.palette.panel_border,
        );
    }

    // Text ------------------------------------------------------------------

    pub fn text(&self, rect: Rect, text: &str, font: HFONT, color: Color, align: TextAlign) {
        self.canvas.text(rect, text, font, color, align);
    }

    pub fn heading(&self, rect: Rect, text: &str) {
        self.text(
            rect,
            text,
            self.fonts.title,
            self.palette.text,
            TextAlign::Left,
        );
    }

    pub fn form_label(&self, rect: Rect, text: &str) {
        self.text(
            rect,
            text,
            self.fonts.body,
            self.palette.muted,
            TextAlign::Left,
        );
    }

    pub fn caption(&self, rect: Rect, text: &str) {
        self.text(
            rect,
            text,
            self.fonts.small,
            self.palette.muted,
            TextAlign::Left,
        );
    }

    pub fn error(&self, rect: Rect, text: &str) {
        self.text(
            rect,
            text,
            self.fonts.body,
            self.palette.error,
            TextAlign::Left,
        );
    }

    // Interactive -----------------------------------------------------------

    pub fn button(&mut self, rect: Rect, label: &str, style: KeyStyle, action: Action) {
        self.key(rect, label, style, action, false, true);
    }

    fn key(
        &mut self,
        rect: Rect,
        label: &str,
        style: KeyStyle,
        action: Action,
        active: bool,
        enabled: bool,
    ) {
        if matches!(style, KeyStyle::Blank) {
            return;
        }
        let id = self.id();
        if enabled {
            self.hit(id, rect, action);
        }
        let hot = enabled && self.hot == Some(id);
        let pressed = enabled && (active || self.pressed == Some(id));
        let radius = self.s(metrics::RADIUS_CONTROL);
        if !enabled {
            self.surface(
                rect,
                radius,
                self.palette.disabled_bg,
                self.palette.disabled_border,
            );
            let font = if label.chars().count() > metrics::BUTTON_SMALL_LABEL {
                self.fonts.button_small
            } else {
                self.fonts.button
            };
            let pad = self.s(metrics::PAD_BUTTON);
            self.text(
                Rect::new(rect.x + pad, rect.y, rect.w - pad * 2, rect.h),
                label,
                font,
                self.palette.disabled_text,
                TextAlign::Center,
            );
            return;
        }
        let (base, border, text_color) = match style {
            KeyStyle::Normal => (
                self.palette.key_bottom,
                self.palette.border,
                self.palette.text,
            ),
            KeyStyle::Operator => (
                self.palette.op_bottom,
                self.palette.border,
                self.palette.text,
            ),
            KeyStyle::Equals => (
                self.palette.eq_bottom,
                self.palette.eq_border,
                self.palette.text,
            ),
            KeyStyle::Memory => (
                self.palette.key_bottom,
                self.palette.border,
                self.palette.muted,
            ),
            KeyStyle::Blank => return,
        };
        // Form actions share the selection palette; '=' keeps its amber accent.
        let primary = matches!(action, Action::Calculate);
        let fill = if pressed {
            self.palette.key_pressed
        } else if hot {
            self.palette.key_hover
        } else if primary {
            self.palette.selection_bg
        } else {
            base
        };
        self.surface(rect, radius, fill, border);
        self.gloss(rect, radius, style, pressed || hot || primary);

        let font = if label.chars().count() > metrics::BUTTON_SMALL_LABEL {
            self.fonts.button_small
        } else {
            self.fonts.button
        };
        let pad = self.s(metrics::PAD_BUTTON);
        self.text(
            Rect::new(rect.x + pad, rect.y, rect.w - pad * 2, rect.h),
            label,
            font,
            text_color,
            TextAlign::Center,
        );
    }

    /// The thin highlight along the top edge that makes keys look raised.
    fn gloss(&self, rect: Rect, radius: i32, style: KeyStyle, muted: bool) {
        if muted {
            return;
        }
        let inset = self.s(1.0);
        let top = Rect::new(
            rect.x + inset,
            rect.y + inset,
            rect.w - inset * 2,
            self.s(metrics::GLOSS_H),
        );
        let top_radius = self
            .s(metrics::RADIUS_GLOSS)
            .min(radius.saturating_sub(inset));
        let color = if style == KeyStyle::Equals {
            self.palette.eq_top
        } else if style == KeyStyle::Operator {
            self.palette.op_top
        } else {
            self.palette.key_top
        };
        self.surface(top, top_radius, color, color);
    }

    pub fn keypad(&mut self, rect: Rect, keys: &[KeySpec], cols: usize, rows: usize, gap: i32) {
        self.keypad_active(rect, keys, cols, rows, gap, None);
    }

    /// Keypad variant that can render one key in a latched (pressed) state.
    pub fn keypad_active(
        &mut self,
        rect: Rect,
        keys: &[KeySpec],
        cols: usize,
        rows: usize,
        gap: i32,
        active: Option<usize>,
    ) {
        for (index, (cell, key)) in key_cells(rect, keys, cols, rows, gap)
            .into_iter()
            .enumerate()
        {
            self.key(
                cell,
                key.label,
                key.style,
                key.action,
                active == Some(index),
                key.enabled,
            );
        }
    }

    /// Shared choice item, independent of horizontal or vertical group layout.
    pub fn selector_item(&mut self, rect: Rect, label: &str, action: Action, selected: bool) {
        let id = self.id();
        self.hit(id, rect, action);
        let hot = self.hot == Some(id);
        let pressed = hot && self.pressed == Some(id);
        if selected || hot || pressed {
            let fill = if pressed {
                self.palette.key_pressed
            } else {
                self.palette.selection_bg
            };
            self.surface(
                rect,
                self.s(metrics::RADIUS_ITEM),
                fill,
                if selected { self.palette.border } else { fill },
            );
        }
        self.text(
            rect,
            label,
            self.fonts.body,
            if selected || hot {
                self.palette.selection_text
            } else {
                self.palette.text
            },
            TextAlign::Center,
        );
    }

    // Composite readouts ----------------------------------------------------

    pub fn display(&mut self, rect: Rect, text: &str, expression: &str, error: bool) {
        self.surface(
            rect,
            self.s(metrics::RADIUS_CONTROL),
            self.palette.display_bg,
            self.palette.border,
        );
        let inner = rect.inset(self.s(metrics::PAD_DISPLAY));
        if !expression.is_empty() {
            self.caption(
                Rect::new(inner.x, inner.y, inner.w, self.s(metrics::EXPRESSION_H)),
                expression,
            );
        }
        self.readout(
            inner,
            text,
            error,
            &[
                self.fonts.display,
                self.fonts.display_mid,
                self.fonts.display_small,
            ],
        );
    }

    pub fn result_box(&mut self, rect: Rect, text: &str, error: bool) {
        self.surface(
            rect,
            self.s(metrics::RADIUS_CONTROL),
            self.palette.display_bg,
            self.palette.border,
        );
        let inner = rect.pad(self.s(metrics::PAD_RESULT), 0);
        self.readout(
            inner,
            text,
            error,
            &[self.fonts.display_mid, self.fonts.display_small],
        );
    }

    fn readout(&self, rect: Rect, text: &str, error: bool, fonts: &[HFONT]) {
        let color = if error {
            self.palette.error
        } else {
            self.palette.text
        };
        let font = self.fit_font(text, rect.w, fonts);
        self.text(rect, text, font, color, TextAlign::Right);
    }

    pub fn history(&mut self, rect: Rect, lines: &[String], title: &str, footer: Option<&str>) {
        self.panel(rect);
        let pad = self.s(metrics::PAD_PANEL);
        let inner = rect.pad(pad, pad);
        self.caption(
            Rect::new(inner.x, inner.y, inner.w, self.s(metrics::HISTORY_TITLE_H)),
            title,
        );
        if let Some(footer) = footer {
            let footer_h = self.s(metrics::HISTORY_FOOTER_H);
            self.caption(
                Rect::new(inner.x, inner.bottom() - footer_h, inner.w, footer_h),
                footer,
            );
        }

        let mut y = rect.y + self.s(metrics::HISTORY_TOP);
        let bottom = rect.bottom() - self.s(metrics::HISTORY_BOTTOM);
        let line_h = self.s(metrics::HISTORY_LINE_H);
        for line in lines.iter().rev() {
            if y + line_h > bottom {
                break;
            }
            self.text(
                Rect::new(inner.x, y, inner.w, line_h),
                line,
                self.fonts.small,
                self.palette.text,
                TextAlign::Left,
            );
            y += self.s(metrics::HISTORY_LINE_STEP);
        }
    }

    pub fn fit_font(&self, text: &str, max_width: i32, fonts: &[HFONT]) -> HFONT {
        fonts
            .iter()
            .copied()
            .find(|font| self.canvas.measure(text, *font) <= max_width)
            .unwrap_or(self.fonts.display_small)
    }
}

/// Shared keypad geometry used by every view and by tests.
pub fn key_cells<'a>(
    rect: Rect,
    keys: &'a [KeySpec],
    cols: usize,
    rows: usize,
    gap: i32,
) -> Vec<(Rect, &'a KeySpec)> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let columns = super::geometry::axis_cells(rect.x, rect.w, cols, gap);
    let row_edges = super::geometry::axis_cells(rect.y, rect.h, rows, gap);
    let mut occupied = vec![false; cols * rows];
    let mut cells = Vec::with_capacity(keys.len());
    for key in keys {
        let Some(slot) = occupied.iter().position(|used| !used) else {
            break;
        };
        if slot >= occupied.len() {
            break;
        }
        let col = slot % cols;
        let row = slot / cols;
        let span_x = key.span_x.max(1).min(cols - col);
        let span_y = key.span_y.max(1).min(rows - row);
        let cell = Rect::new(
            columns[col].0,
            row_edges[row].0,
            columns[col + span_x - 1].1 - columns[col].0,
            row_edges[row + span_y - 1].1 - row_edges[row].0,
        );
        for r in row..row + span_y {
            for c in col..col + span_x {
                occupied[r * cols + c] = true;
            }
        }
        cells.push((cell, key));
    }
    cells
}

#[cfg(test)]
mod dpi_grid_tests {
    use super::*;
    use crate::actions::{programmer_keys, standard_keys};
    use crate::ui::DpiScale;

    #[test]
    fn fractional_dpi_fills_the_grid_and_preserves_spanning_keys() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let scale = DpiScale::new(dpi);
            for (keys, cols, logical_w) in [
                (standard_keys(), metrics::KEYPAD_COLS, metrics::KEYPAD_W),
                (
                    programmer_keys(),
                    metrics::KEYPAD_COLS_WIDE,
                    metrics::KEYPAD_WIDE_W,
                ),
            ] {
                let rect = Rect::new(13, 21, scale.px(logical_w), scale.px(metrics::KEYPAD_H));
                let gap = scale.px(metrics::GAP_KEYPAD);
                let cells = key_cells(rect, &keys, cols, metrics::KEYPAD_ROWS, gap);
                assert_eq!(
                    cells.iter().map(|(r, _)| r.right()).max(),
                    Some(rect.right())
                );
                assert_eq!(
                    cells.iter().map(|(r, _)| r.bottom()).max(),
                    Some(rect.bottom())
                );
                for (i, (r, key)) in cells.iter().enumerate() {
                    assert!(
                        r.x >= rect.x
                            && r.y >= rect.y
                            && r.right() <= rect.right()
                            && r.bottom() <= rect.bottom()
                    );
                    for (other, _) in cells.iter().skip(i + 1) {
                        assert!(
                            r.right() <= other.x
                                || other.right() <= r.x
                                || r.bottom() <= other.y
                                || other.bottom() <= r.y
                        );
                    }
                    if key.span_y > 1 {
                        assert_eq!(r.bottom(), rect.bottom());
                    }
                }
                let widths: Vec<_> = cells
                    .iter()
                    .filter(|(_, k)| k.span_x == 1)
                    .map(|(r, _)| r.w)
                    .collect();
                assert!(widths.iter().max().unwrap() - widths.iter().min().unwrap() <= 1);
                let heights: Vec<_> = cells
                    .iter()
                    .filter(|(_, k)| k.span_y == 1)
                    .map(|(r, _)| r.h)
                    .collect();
                assert!(heights.iter().max().unwrap() - heights.iter().min().unwrap() <= 1);
            }
        }
    }
}
