use crate::actions::Action;
use crate::engine::{Mode, StatsState};
use crate::ui::{metrics, Rect, Ui};
use rustic_calc_app::keypad::{action_for_char, action_for_key, apply_statistics};

use super::common::sidebar;
use super::{keypad_key, Shared, View};

pub struct StatisticsView {
    stats: StatsState,
}

impl StatisticsView {
    pub fn new() -> Self {
        Self {
            stats: StatsState::new(),
        }
    }
}

impl View for StatisticsView {
    fn mode(&self) -> Mode {
        Mode::Statistics
    }

    fn on_enter(&mut self, _shared: &mut Shared) {}

    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let client = shared.client;
        let m = ui.s(metrics::MARGIN);
        let sidebar_w = ui.s(metrics::SIDEBAR_W);
        let rect = Rect::new(
            client.x + (client.w - sidebar_w) / 2,
            client.y + m,
            sidebar_w,
            client.h - m * 2,
        );
        sidebar(
            ui,
            rect,
            &self.stats,
            shared.text(),
            Some(self.stats.count()),
        );
    }

    fn action(&mut self, action: Action, shared: &mut Shared) {
        let current = self.stats.current_value();
        apply_statistics(
            action,
            shared.lang,
            &mut self.stats,
            &mut shared.memory,
            current,
        );
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
        crate::i18n::runtime_error(shared.lang, &self.stats.display())
    }
}
