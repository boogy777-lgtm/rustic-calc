use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use crate::actions::{Action, MemAction, StatsFunc};
use crate::engine::{Mode, StatsState};
use crate::ui::{metrics, Rect, Ui};

use super::common::sidebar;
use super::{Shared, View};

pub struct StatisticsView {
    stats: StatsState,
}

impl StatisticsView {
    pub fn new() -> Self {
        Self {
            stats: StatsState::new(),
        }
    }

    fn apply(&mut self, action: Action, shared: &mut Shared) {
        let t = shared.text();
        match action {
            Action::StatsDigit(ch) => self.stats.digit(ch),
            Action::StatsDecimal => self.stats.decimal(),
            Action::StatsAdd => self.stats.add_data(),
            Action::StatsClearAll => self.stats.clear_dataset(),
            Action::StatsClear => self.stats.clear_entry(),
            Action::StatsBackspace => self.stats.backspace(),
            Action::StatsSign => self.stats.toggle_sign(),
            Action::StatsExp => self.stats.exp_entry(),
            Action::StatsCalc(func) => match func {
                StatsFunc::Mean => self.stats.show(t.stats_average, self.stats.mean()),
                StatsFunc::MeanSq => self
                    .stats
                    .show(t.stats_average_squares, self.stats.mean_squares()),
                StatsFunc::Sum => self.stats.show(t.stats_sum, self.stats.sum()),
                StatsFunc::SumSq => self
                    .stats
                    .show(t.stats_sum_squares, self.stats.sum_squares()),
                StatsFunc::SdPop => self.stats.show(t.stats_sd, self.stats.std_dev_population()),
                StatsFunc::SdSample => self
                    .stats
                    .show(t.stats_sd_sample, self.stats.std_dev_sample()),
            },
            Action::Mem(MemAction::Clear) => shared.memory = 0.0,
            Action::Mem(MemAction::Recall) => self.stats.recall(shared.memory),
            Action::Mem(MemAction::Store) => shared.memory = self.stats.current_value(),
            Action::Mem(MemAction::Add) => shared.memory += self.stats.current_value(),
            Action::Mem(MemAction::Sub) => shared.memory -= self.stats.current_value(),
            _ => {}
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
        self.apply(action, shared);
    }

    fn char(&mut self, ch: char, shared: &mut Shared) -> bool {
        let action = match ch {
            '0'..='9' => Action::StatsDigit(ch),
            '.' | ',' => Action::StatsDecimal,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn key(&mut self, vk: u16, _ctrl: bool, shared: &mut Shared) -> bool {
        let action = match vk {
            VK_RETURN => Action::StatsAdd,
            VK_BACK => Action::StatsBackspace,
            VK_ESCAPE => Action::StatsClear,
            _ => return false,
        };
        self.apply(action, shared);
        true
    }

    fn display(&self, shared: &Shared) -> String {
        crate::i18n::runtime_error(shared.lang, &self.stats.display())
    }
}
