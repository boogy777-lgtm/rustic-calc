use crate::actions::{stats_keys, Action};
use crate::engine::{AngleMode, StatsState};
use crate::i18n::Text;
use crate::ui::{metrics, Rect, Ui};

pub fn sidebar(ui: &mut Ui, rect: Rect, stats: &StatsState, text: &Text, count: Option<usize>) {
    let gap = ui.s(metrics::GAP);
    let display_h = ui.s(metrics::SIDEBAR_DISPLAY_H);
    let keypad_h = ui.s(metrics::KEYPAD_H);
    let history_h = (rect.h - display_h - keypad_h - gap * 2).max(0);
    let history = Rect::new(rect.x, rect.y, rect.w, history_h);
    let display = Rect::new(rect.x, history.bottom() + gap, rect.w, display_h);
    let keypad = Rect::new(rect.x, display.bottom() + gap, rect.w, keypad_h);

    let mut lines: Vec<String> = stats
        .history
        .iter()
        .rev()
        .take(metrics::HISTORY_VISIBLE)
        .cloned()
        .collect();
    if !stats.result_text.is_empty() {
        lines.insert(0, stats.result_text.clone());
    }
    let footer = count.map(|count| format!("{} = {count}", text.count));
    ui.history(history, &lines, text.running_history, footer.as_deref());

    ui.display(display, &stats.display(), "", stats.error.is_some());
    ui.keypad(
        keypad,
        &stats_keys(text.stats_add),
        metrics::KEYPAD_COLS,
        metrics::KEYPAD_ROWS,
        ui.s(metrics::GAP_KEYPAD),
    );
}

fn angle_cells(rect: Rect, scale: f32, count: usize) -> Vec<Rect> {
    let s = |v: f32| crate::ui::dpi::dp_to_px(v, scale);
    rect.inset(s(metrics::PAD_PANEL))
        .columns(count, s(metrics::SELECTOR_GAP))
}

pub fn angle_selector(ui: &mut Ui, rect: Rect, angle: AngleMode, text: &Text) {
    ui.panel(rect);
    let modes = [
        (AngleMode::Degrees, text.angle_degrees),
        (AngleMode::Radians, text.angle_radians),
        (AngleMode::Gradians, text.angle_grads),
    ];
    for (cell, (mode, label)) in angle_cells(rect, ui.scale, modes.len())
        .into_iter()
        .zip(modes)
    {
        let selected = angle == mode;
        ui.selector_item(cell, label, Action::SetAngle(mode), selected);
    }
}

#[cfg(test)]
mod selector_tests {
    use super::*;

    #[test]
    fn angle_selection_uses_panel_padding_on_every_edge() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let scale = crate::ui::DpiScale::new(dpi);
            let s = |v: f32| scale.px(v);
            let height =
                (s(metrics::KEYPAD_H) - s(metrics::SCIENTIFIC_GRID_H) - s(metrics::GAP_TIGHT))
                    .max(s(metrics::SELECTOR_ITEM_H + metrics::PAD_PANEL * 2.0));
            let panel = Rect::new(17, 29, s(metrics::KEYPAD_W), height);
            let cells = angle_cells(panel, scale.factor(), 3);
            let pad = s(metrics::PAD_PANEL);
            assert_eq!(cells[0].x - panel.x, pad);
            assert_eq!(panel.right() - cells[2].right(), pad);
            for cell in &cells {
                assert_eq!(cell.y - panel.y, pad);
                assert_eq!(panel.bottom() - cell.bottom(), pad);
                assert!(cell.h >= s(metrics::SELECTOR_ITEM_H));
            }
            assert!(cells
                .windows(2)
                .all(|c| c[1].x - c[0].right() == s(metrics::SELECTOR_GAP)));
            assert!(
                cells.iter().map(|c| c.w).max().unwrap() - cells.iter().map(|c| c.w).min().unwrap()
                    <= 1
            );
        }
    }
}
