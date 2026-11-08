mod common;
mod date;
mod programmer;
mod scientific;
mod standard;
mod statistics;
mod unit;
mod worksheet;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::HFONT;

use crate::actions::{Action, MemAction, StatsFunc};
use crate::engine::{Mode, StatsState, WorksheetKind};
use crate::i18n::{Lang, Text};
use crate::platform;
use crate::ui::palette::ThemeMode;
use crate::ui::{metrics, Rect, Ui};

pub struct Shared {
    pub hwnd: HWND,
    pub dpi: u32,
    pub scale: f32,
    pub hfont: HFONT,
    pub memory: f64,
    pub grouping: bool,
    pub show_history: bool,
    pub sidebar: StatsState,
    pub client: Rect,
    pub lang: Lang,
    pub theme: ThemeMode,
    pub dark: bool,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            hwnd: std::ptr::null_mut(),
            dpi: crate::ui::dpi::BASE_DPI,
            scale: 1.0,
            hfont: std::ptr::null_mut(),
            memory: 0.0,
            grouping: true,
            show_history: false,
            sidebar: StatsState::new(),
            client: Rect::new(0, 0, 0, 0),
            lang: Lang::default(),
            theme: ThemeMode::System,
            dark: false,
        }
    }

    pub fn text(&self) -> &'static Text {
        self.lang.text()
    }

    pub fn attach(&mut self, hwnd: HWND, dpi: u32, hfont: HFONT) {
        self.hwnd = hwnd;
        let scale = crate::ui::DpiScale::new(dpi);
        self.dpi = scale.dpi();
        self.scale = scale.factor();
        self.hfont = hfont;
    }

    pub fn s(&self, logical: f32) -> i32 {
        crate::ui::dpi::dp_to_px(logical, self.scale)
    }

    pub fn edit(&self, id: u16, rect: Rect, text: &str) -> HWND {
        let hwnd = platform::create_edit(self.hwnd, id, rect.x, rect.y, rect.w, rect.h, text);
        platform::set_font(hwnd, self.hfont);
        platform::theme_control(hwnd, self.dark);
        platform::install_edit_paint(hwnd);
        hwnd
    }

    pub fn combo(&self, id: u16, rect: Rect, items: &[&str], selected: usize) -> HWND {
        let hwnd = platform::create_combo(
            self.hwnd,
            id,
            rect.x,
            rect.y,
            rect.w,
            rect.h + self.s(metrics::COMBO_LIST_H),
            items,
            selected,
        );
        platform::set_font(hwnd, self.hfont);
        platform::theme_control(hwnd, self.dark);
        hwnd
    }

    pub fn sidebar_action(&mut self, action: Action, current: f64) -> bool {
        match action {
            Action::Mem(MemAction::Clear) => self.memory = 0.0,
            Action::Mem(MemAction::Recall) => self.sidebar.recall(self.memory),
            Action::Mem(MemAction::Store) => self.memory = current,
            Action::Mem(MemAction::Add) => self.memory += current,
            Action::Mem(MemAction::Sub) => self.memory -= current,
            Action::StatsDigit(ch) => self.sidebar.digit(ch),
            Action::StatsDecimal => self.sidebar.decimal(),
            Action::StatsAdd => self.sidebar.add_data(),
            Action::StatsClearAll => self.sidebar.clear_dataset(),
            Action::StatsClear => self.sidebar.clear_entry(),
            Action::StatsBackspace => self.sidebar.backspace(),
            Action::StatsSign => self.sidebar.toggle_sign(),
            Action::StatsExp => self.sidebar.exp_entry(),
            Action::StatsCalc(func) => {
                let t = self.lang.text();
                match func {
                    StatsFunc::Mean => self.sidebar.show(t.stats_average, self.sidebar.mean()),
                    StatsFunc::MeanSq => self
                        .sidebar
                        .show(t.stats_average_squares, self.sidebar.mean_squares()),
                    StatsFunc::Sum => self.sidebar.show(t.stats_sum, self.sidebar.sum()),
                    StatsFunc::SumSq => self
                        .sidebar
                        .show(t.stats_sum_squares, self.sidebar.sum_squares()),
                    StatsFunc::SdPop => self
                        .sidebar
                        .show(t.stats_sd, self.sidebar.std_dev_population()),
                    StatsFunc::SdSample => self
                        .sidebar
                        .show(t.stats_sd_sample, self.sidebar.std_dev_sample()),
                }
            }
            _ => return false,
        }
        true
    }
}

pub trait View {
    fn mode(&self) -> Mode;
    fn on_enter(&mut self, shared: &mut Shared);
    fn on_exit(&mut self, _shared: &mut Shared) {}
    fn on_resize(&mut self, _shared: &mut Shared) {}
    fn paint(&mut self, ui: &mut Ui, shared: &mut Shared);
    fn action(&mut self, action: Action, shared: &mut Shared);
    fn command(&mut self, _id: u16, _code: u16, _shared: &mut Shared) -> bool {
        false
    }
    /// Client rectangle that must be repainted after `id` was handled.
    /// `None` asks the host to invalidate the whole window.
    fn command_rect(&self, _id: u16, _shared: &Shared) -> Option<Rect> {
        None
    }
    fn key(&mut self, _vk: u16, _ctrl: bool, _shared: &mut Shared) -> bool {
        false
    }
    fn char(&mut self, _ch: char, _shared: &mut Shared) -> bool {
        false
    }
    fn display(&self, _shared: &Shared) -> String {
        String::new()
    }
}

fn index(mode: Mode) -> usize {
    match mode {
        Mode::Standard => 0,
        Mode::Scientific => 1,
        Mode::Programmer => 2,
        Mode::Statistics => 3,
        Mode::UnitConversion => 4,
        Mode::DateCalculation => 5,
        Mode::Worksheet(WorksheetKind::Mortgage) => 6,
        Mode::Worksheet(WorksheetKind::VehicleLease) => 7,
        Mode::Worksheet(WorksheetKind::FuelMpg) => 8,
        Mode::Worksheet(WorksheetKind::FuelL100) => 9,
    }
}

fn create(mode: Mode) -> Box<dyn View> {
    match mode {
        Mode::Standard => Box::new(standard::StandardView::new()),
        Mode::Scientific => Box::new(scientific::ScientificView::new()),
        Mode::Programmer => Box::new(programmer::ProgrammerView::new()),
        Mode::Statistics => Box::new(statistics::StatisticsView::new()),
        Mode::UnitConversion => Box::new(unit::UnitView::new()),
        Mode::DateCalculation => Box::new(date::DateView::new()),
        Mode::Worksheet(kind) => Box::new(worksheet::WorksheetView::new(kind)),
    }
}

pub struct ViewManager {
    slots: Vec<Option<Box<dyn View>>>,
    active: Option<Mode>,
}

impl ViewManager {
    pub fn new() -> Self {
        Self {
            slots: (0..10).map(|_| None).collect(),
            active: None,
        }
    }

    pub fn active_mode(&self) -> Option<Mode> {
        self.active_ref().map(|view| view.mode())
    }

    pub fn activate(&mut self, mode: Mode, shared: &mut Shared) {
        if self.active == Some(mode) {
            return;
        }
        if let Some(index) = self.active.map(index) {
            if let Some(view) = self.slots[index].as_mut() {
                view.on_exit(shared);
            }
        }
        self.active = Some(mode);
        let index = index(mode);
        if self.slots[index].is_none() {
            self.slots[index] = Some(create(mode));
        }
        if let Some(view) = self.slots[index].as_mut() {
            view.on_enter(shared);
        }
    }

    pub fn active_mut(&mut self) -> Option<&mut Box<dyn View>> {
        self.active
            .map(index)
            .and_then(|index| self.slots[index].as_mut())
    }

    pub fn active_ref(&self) -> Option<&Box<dyn View>> {
        self.active
            .map(index)
            .and_then(|index| self.slots[index].as_ref())
    }

    pub fn shutdown(&mut self, shared: &mut Shared) {
        if let Some(index) = self.active.map(index) {
            if let Some(view) = self.slots[index].as_mut() {
                view.on_exit(shared);
            }
        }
        self.slots.iter_mut().for_each(|slot| {
            slot.take();
        });
        self.active = None;
    }
}

pub fn mode_client_size(mode: Mode, menu_height: i32) -> (i32, i32) {
    let (width, height) = match mode {
        Mode::Standard => (
            metrics::MARGIN * 2.0 + metrics::KEYPAD_W,
            metrics::COMPACT_CONTENT_H,
        ),
        Mode::Scientific => (
            metrics::MARGIN * 2.0 + metrics::KEYPAD_W * 2.0 + metrics::COLUMN_GAP,
            metrics::COMPACT_CONTENT_H,
        ),
        Mode::Programmer => (
            metrics::MARGIN * 2.0
                + metrics::PROGRAMMER_BASE_W
                + metrics::COLUMN_GAP
                + metrics::KEYPAD_WIDE_W,
            metrics::PROGRAMMER_CONTENT_H,
        ),
        Mode::Statistics => (
            metrics::MARGIN * 2.0 + metrics::SIDEBAR_W,
            metrics::FORM_CONTENT_H,
        ),
        Mode::UnitConversion | Mode::DateCalculation | Mode::Worksheet(_) => (
            metrics::MARGIN * 3.0
                + metrics::SIDEBAR_W
                + metrics::KEYPAD_W * 2.0
                + metrics::COLUMN_GAP,
            metrics::FORM_CONTENT_H,
        ),
    };
    // Reserve content independently from the font-measured menu bar.
    (width as i32, height as i32 + menu_height)
}
