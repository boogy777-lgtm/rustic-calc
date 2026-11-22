#![cfg_attr(
    all(target_os = "windows", target_arch = "x86_64", not(debug_assertions)),
    windows_subsystem = "windows"
)]

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
compile_error!("Rustic Calc GPUI is currently supported only on Windows x64.");

use std::ops::Range;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use gpui::{
    App, Application, Bounds, Context, DispatchPhase, ElementInputHandler, EntityInputHandler,
    FocusHandle, Focusable, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, TextRun, UTF16Selection, UnderlineStyle, Window, WindowAppearance,
    WindowBounds, WindowOptions, canvas, div, fill, point, prelude::*, px, rgb, size,
};
use rustic_calc_app::actions::{self, Action, KeySpec, KeyStyle};
use rustic_calc_app::coordinate_session::{
    CoordinateInputField, CoordinateSession, CoordinateTemplateSelection,
};
use rustic_calc_app::date::{
    DateError, DateField, DateMode, DateOperation, DateOutcome, DateSession,
};
use rustic_calc_app::engine::{Mode, StatsState, WorksheetKind};
use rustic_calc_app::i18n::Lang;
use rustic_calc_app::keypad::{
    ArithmeticSession, KeyInput, ProgrammerSession, action_for_char, action_for_key,
    apply_statistics,
};
use rustic_calc_app::startup::{self, LaunchOptions, ThemeMode};
use rustic_calc_app::unit_session::{
    ConversionResult, DcsLoopError, UnitField, UnitIndex, UnitSession,
};
use rustic_calc_app::units::{CategoryId, Date};
use rustic_calc_app::worksheet::{WorksheetField, WorksheetSession};
use text_input::{InputEdit, TextEditState};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

mod text_input;
mod windows_capture;

const WINDOW_WIDTH: f32 = 820.0;
const WINDOW_HEIGHT: f32 = 930.0;

struct Selection {
    range: Range<usize>,
    reversed: bool,
}

struct InputLayout {
    bounds: [Option<Bounds<Pixels>>; 28],
    lines: [Option<gpui::ShapedLine>; 28],
}

impl Default for InputLayout {
    fn default() -> Self {
        Self {
            bounds: std::array::from_fn(|_| None),
            lines: std::array::from_fn(|_| None),
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            range: 0..0,
            reversed: false,
        }
    }
}

impl Selection {
    fn cursor(&self) -> usize {
        if self.reversed {
            self.range.start
        } else {
            self.range.end
        }
    }

    fn move_to(&mut self, offset: usize) {
        self.range = offset..offset;
        self.reversed = false;
    }

    fn select_to(&mut self, offset: usize) {
        if self.reversed {
            self.range.start = offset;
        } else {
            self.range.end = offset;
        }
        if self.range.end < self.range.start {
            std::mem::swap(&mut self.range.start, &mut self.range.end);
            self.reversed = !self.reversed;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputTarget {
    Date(DateField),
    Unit(UnitField),
    Worksheet(WorksheetField),
    Coordinate(CoordinateInputField),
    CoordinateDefinition,
}

struct GpuiCalc {
    mode: Mode,
    lang: Lang,
    theme: ThemeMode,
    grouping: bool,
    arithmetic: [ArithmeticSession; 2],
    programmer: ProgrammerSession,
    stats: StatsState,
    date: DateSession,
    units: UnitSession,
    worksheets: [WorksheetSession; 4],
    coordinates: CoordinateSession,
    memory: f64,
    focus_handle: FocusHandle,
    focused_input: Option<InputTarget>,
    selections: [Selection; 28],
    marked_ranges: [Option<Range<usize>>; 28],
    input_layout: Rc<RefCell<InputLayout>>,
    selecting_input: Option<InputTarget>,
    show_coordinate_help: bool,
}

impl GpuiCalc {
    fn new(options: LaunchOptions, cx: &mut Context<Self>) -> Self {
        Self {
            mode: options.mode,
            lang: options.lang,
            theme: options.theme,
            grouping: false,
            arithmetic: [ArithmeticSession::new(false), ArithmeticSession::new(true)],
            programmer: ProgrammerSession::new(),
            stats: StatsState::new(),
            date: DateSession::new(local_today()),
            units: UnitSession::new(),
            worksheets: [
                WorksheetSession::new(WorksheetKind::Mortgage),
                WorksheetSession::new(WorksheetKind::VehicleLease),
                WorksheetSession::new(WorksheetKind::FuelMpg),
                WorksheetSession::new(WorksheetKind::FuelL100),
            ],
            coordinates: CoordinateSession::new(),
            memory: 0.0,
            focus_handle: cx.focus_handle(),
            focused_input: None,
            selections: std::array::from_fn(|_| Selection::default()),
            marked_ranges: std::array::from_fn(|_| None),
            input_layout: Rc::new(RefCell::new(InputLayout::default())),
            selecting_input: None,
            show_coordinate_help: false,
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        if let Some(target) = self.focused_input {
            self.marked_ranges[input_index(target)] = None;
        }
        self.mode = mode;
        self.focused_input = None;
        self.selecting_input = None;
        self.show_coordinate_help = false;
    }

    fn toggle_coordinate_help(&mut self) {
        self.show_coordinate_help = !self.show_coordinate_help;
        self.selecting_input = None;
        if self.show_coordinate_help {
            if let Some(target) = self.focused_input {
                self.marked_ranges[input_index(target)] = None;
            }
            self.focused_input = None;
        } else if self.mode == Mode::Coordinates {
            let target = InputTarget::Coordinate(CoordinateInputField::Latitude);
            let cursor = self.input_text(target).map(str::len).unwrap_or(0);
            self.focused_input = Some(target);
            self.selections[input_index(target)].move_to(cursor);
        }
    }

    fn active_worksheet(&self) -> Option<&WorksheetSession> {
        let index = worksheet_index(self.mode)?;
        self.worksheets.get(index)
    }

    fn active_worksheet_mut(&mut self) -> Option<&mut WorksheetSession> {
        let index = worksheet_index(self.mode)?;
        self.worksheets.get_mut(index)
    }

    fn input_text(&self, target: InputTarget) -> Option<&str> {
        match target {
            InputTarget::Date(field) => Some(self.date.text(field)),
            InputTarget::Unit(field) => Some(self.units.text(field)),
            InputTarget::Worksheet(field) => self
                .active_worksheet()
                .and_then(|session| session.text(field)),
            InputTarget::Coordinate(field) => Some(self.coordinates.text(field)),
            InputTarget::CoordinateDefinition => Some(self.coordinates.definition()),
        }
    }

    fn set_input_text(&mut self, target: InputTarget, value: String) {
        match target {
            InputTarget::Date(field) => self.date.set_text(field, value),
            InputTarget::Unit(field) => self.units.set_text(field, value),
            InputTarget::Worksheet(field) => {
                if let Some(session) = self.active_worksheet_mut() {
                    session.set_text(field, value);
                }
            }
            InputTarget::Coordinate(field) => self.coordinates.set_text(field, value),
            InputTarget::CoordinateDefinition => self.coordinates.set_definition(value),
        }
    }

    fn input_edit_state(&self, target: InputTarget) -> TextEditState {
        let index = input_index(target);
        let selection = &self.selections[index];
        TextEditState::new(
            self.input_text(target).unwrap_or_default().to_owned(),
            selection.range.clone(),
            selection.reversed,
            self.marked_ranges[index].clone(),
        )
    }

    fn apply_input_edit(&mut self, edit: InputEdit, cx: &mut Context<Self>) {
        let Some(target) = self.focused_input else {
            return;
        };
        let index = input_index(target);
        let mut state = self.input_edit_state(target);
        state.apply(edit);
        self.set_input_text(target, state.text);
        self.selections[index] = Selection {
            range: state.selection,
            reversed: state.reversed,
        };
        self.marked_ranges[index] = state.marked_range;
        cx.notify();
    }

    fn input_byte_index_for_point(
        &self,
        target: InputTarget,
        point: Point<Pixels>,
    ) -> Option<usize> {
        let index = input_index(target);
        let layout = self.input_layout.borrow();
        let bounds = layout.bounds[index]?;
        let line = layout.lines[index].as_ref()?;
        let text = self.input_text(target)?;
        let x = if point.x < bounds.left() {
            px(0.)
        } else if point.x > bounds.right() {
            bounds.size.width
        } else {
            point.x - bounds.left()
        };
        Some(text_input::clamp_utf8_offset(
            text,
            line.closest_index_for_x(x),
            false,
        ))
    }

    fn visible_inputs(&self) -> Vec<InputTarget> {
        if self.show_coordinate_help {
            return Vec::new();
        }
        match self.mode {
            Mode::DateCalculation => {
                let mut fields = vec![InputTarget::Date(DateField::From)];
                match self.date.mode() {
                    DateMode::Difference => fields.push(InputTarget::Date(DateField::To)),
                    DateMode::Shift => fields.extend([
                        InputTarget::Date(DateField::Years),
                        InputTarget::Date(DateField::Months),
                        InputTarget::Date(DateField::Days),
                    ]),
                }
                fields
            }
            Mode::UnitConversion => {
                let mut fields = vec![InputTarget::Unit(UnitField::Value)];
                if self.units.category_id() == CategoryId::DcsSignals {
                    fields.extend([
                        InputTarget::Unit(UnitField::LoopVoltage),
                        InputTarget::Unit(UnitField::LoopCurrentMa),
                    ]);
                }
                fields
            }
            Mode::Worksheet(_) => self
                .active_worksheet()
                .map(|session| {
                    session
                        .fields()
                        .iter()
                        .copied()
                        .map(InputTarget::Worksheet)
                        .collect()
                })
                .unwrap_or_default(),
            Mode::Coordinates => vec![
                InputTarget::Coordinate(CoordinateInputField::Latitude),
                InputTarget::Coordinate(CoordinateInputField::Longitude),
                InputTarget::Coordinate(CoordinateInputField::Height),
                InputTarget::CoordinateDefinition,
            ],
            _ => Vec::new(),
        }
    }

    fn focus_next_input(&mut self, backwards: bool) {
        let inputs = self.visible_inputs();
        if inputs.is_empty() {
            return;
        }
        if let Some(target) = self.focused_input {
            self.marked_ranges[input_index(target)] = None;
        }
        let current = self
            .focused_input
            .and_then(|target| inputs.iter().position(|candidate| *candidate == target));
        let next = match (current, backwards) {
            (Some(0), true) => inputs.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % inputs.len(),
            (None, true) => inputs.len() - 1,
            (None, false) => 0,
        };
        let target = inputs[next];
        self.focused_input = Some(target);
        let cursor = self.input_text(target).map(str::len).unwrap_or(0);
        self.selections[input_index(target)].move_to(cursor);
    }

    fn clear_hidden_focus(&mut self) {
        if let Some(target) = self.focused_input {
            if !self.visible_inputs().contains(&target) {
                self.marked_ranges[input_index(target)] = None;
                self.focused_input = None;
            }
        }
    }

    fn apply(&mut self, action: Action) {
        match self.mode {
            Mode::Standard | Mode::Scientific => {
                let index = arithmetic_index(self.mode).expect("arithmetic mode has a session");
                self.arithmetic[index].apply(action, self.lang, &mut self.memory);
            }
            Mode::Programmer => self.programmer.apply(action, &mut self.memory),
            Mode::Statistics => {
                let current = self.stats.current_value();
                apply_statistics(
                    action,
                    self.lang,
                    &mut self.stats,
                    &mut self.memory,
                    current,
                );
            }
            Mode::DateCalculation => {
                if matches!(action, Action::Calculate) {
                    self.date.calculate();
                }
            }
            _ => {}
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.control {
            if key.key.eq_ignore_ascii_case("q") {
                cx.quit();
                return;
            }
            if let Some(mode) = mode_for_keystroke(&key.key) {
                self.set_mode(mode);
                cx.notify();
                cx.stop_propagation();
                return;
            }
            if key.key.eq_ignore_ascii_case("a") && self.focused_input.is_some() {
                if let Some(target) = self.focused_input {
                    let len = self.input_text(target).map(str::len).unwrap_or(0);
                    self.selections[input_index(target)] = Selection {
                        range: 0..len,
                        reversed: false,
                    };
                    cx.notify();
                    cx.stop_propagation();
                }
                return;
            }
            if key.key.eq_ignore_ascii_case("c") && self.focused_input.is_some() {
                self.copy_input_selection(cx);
                cx.stop_propagation();
                return;
            }
            if key.key.eq_ignore_ascii_case("x") && self.focused_input.is_some() {
                self.copy_input_selection(cx);
                self.replace_input_selection("");
                cx.notify();
                cx.stop_propagation();
                return;
            }
            if key.key.eq_ignore_ascii_case("v") && self.focused_input.is_some() {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.replace_input_selection(&text.replace(['\r', '\n'], ""));
                    cx.notify();
                }
                cx.stop_propagation();
                return;
            }
        }

        if self.mode == Mode::Coordinates && self.show_coordinate_help {
            if !key.modifiers.control
                && !key.modifiers.alt
                && matches!(key.key.as_str(), "escape" | "f2")
            {
                self.toggle_coordinate_help();
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if self.mode == Mode::Coordinates
            && !key.modifiers.control
            && !key.modifiers.alt
            && key.key == "f2"
        {
            self.toggle_coordinate_help();
            cx.notify();
            cx.stop_propagation();
            return;
        }

        if key.key == "tab" && !self.visible_inputs().is_empty() {
            self.focus_next_input(key.modifiers.shift);
            cx.notify();
            cx.stop_propagation();
            return;
        }

        if self.focused_input.is_some() {
            if self.handle_input_key(key, cx) {
                cx.stop_propagation();
                return;
            }
        }

        let action = match key.key.as_str() {
            "enter" => action_for_key(self.mode, KeyInput::Enter),
            "backspace" => action_for_key(self.mode, KeyInput::Backspace),
            "escape" => action_for_key(self.mode, KeyInput::Escape),
            "delete" => action_for_key(self.mode, KeyInput::Delete),
            _ => key
                .key_char
                .as_deref()
                .and_then(|text| text.chars().next())
                .and_then(|ch| action_for_char(self.mode, ch)),
        };
        if let Some(action) = action {
            self.apply(action);
            cx.notify();
            cx.stop_propagation();
        }
    }

    fn handle_input_key(&mut self, key: &gpui::Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(target) = self.focused_input else {
            return false;
        };
        let index = input_index(target);
        let text = self.input_text(target).unwrap_or_default().to_owned();
        let selection = &mut self.selections[index];
        let cursor = selection.cursor().min(text.len());
        let shift = key.modifiers.shift;
        match key.key.as_str() {
            "left" => {
                let next = if selection.range.is_empty() {
                    previous_boundary(&text, cursor)
                } else if selection.reversed {
                    selection.range.end
                } else {
                    selection.range.start
                };
                if shift {
                    selection.select_to(next);
                } else {
                    selection.move_to(next);
                }
            }
            "right" => {
                let next = if selection.range.is_empty() {
                    next_boundary(&text, cursor)
                } else if selection.reversed {
                    selection.range.start
                } else {
                    selection.range.end
                };
                if shift {
                    selection.select_to(next);
                } else {
                    selection.move_to(next);
                }
            }
            "home" => {
                if shift {
                    selection.select_to(0);
                } else {
                    selection.move_to(0);
                }
            }
            "end" => {
                if shift {
                    selection.select_to(text.len());
                } else {
                    selection.move_to(text.len());
                }
            }
            "backspace" => {
                if selection.range.is_empty() && cursor > 0 {
                    selection.range = previous_boundary(&text, cursor)..cursor;
                }
                self.replace_input_selection("");
            }
            "delete" => {
                if selection.range.is_empty() && cursor < text.len() {
                    selection.range = cursor..next_boundary(&text, cursor);
                }
                self.replace_input_selection("");
            }
            "enter" => match self.mode {
                Mode::DateCalculation => self.date.calculate(),
                Mode::Worksheet(_) => {
                    if let Some(session) = self.active_worksheet_mut() {
                        let _ = session.calculate();
                    }
                }
                Mode::Coordinates => {
                    let _ = self.coordinates.calculate();
                }
                _ => return false,
            },
            _ => {
                if let Some(insert) = key
                    .key_char
                    .as_deref()
                    .filter(|_| !key.modifiers.control && !key.modifiers.alt)
                {
                    self.replace_input_selection(insert);
                } else {
                    return false;
                }
            }
        }
        cx.notify();
        true
    }

    fn replace_input_selection(&mut self, insert: &str) {
        let Some(target) = self.focused_input else {
            return;
        };
        let index = input_index(target);
        let text = self.input_text(target).unwrap_or_default().to_owned();
        let (start, end) = {
            let selection = &self.selections[index];
            (
                selection.range.start.min(text.len()),
                selection.range.end.min(text.len()),
            )
        };
        let mut updated = String::with_capacity(text.len() + insert.len());
        updated.push_str(&text[..start]);
        updated.push_str(insert);
        updated.push_str(&text[end..]);
        let cursor = start + insert.len();
        self.set_input_text(target, updated);
        self.selections[index].move_to(cursor);
        self.marked_ranges[index] = None;
    }

    fn copy_input_selection(&self, cx: &mut Context<Self>) {
        let Some(target) = self.focused_input else {
            return;
        };
        let Some(text) = self.input_text(target) else {
            return;
        };
        let range = &self.selections[input_index(target)].range;
        if range.start < range.end && range.end <= text.len() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                text[range.clone()].to_owned(),
            ));
        }
    }

    fn on_input_click(
        &mut self,
        target: InputTarget,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(previous) = self.focused_input
            && previous != target
        {
            self.marked_ranges[input_index(previous)] = None;
        }
        self.focused_input = Some(target);
        let index = input_index(target);
        let cursor = self
            .input_byte_index_for_point(target, event.position)
            .unwrap_or_else(|| self.input_text(target).map(str::len).unwrap_or(0));
        self.marked_ranges[index] = None;
        if event.modifiers.shift {
            self.selections[index].select_to(cursor);
        } else {
            self.selections[index].move_to(cursor);
        }
        self.selecting_input = Some(target);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn stop_input_selection(&mut self, target: InputTarget, cx: &mut Context<Self>) {
        if self.selecting_input == Some(target) {
            self.selecting_input = None;
            cx.notify();
        }
    }

    fn render_input(
        &self,
        target: InputTarget,
        label: &'static str,
        panel: gpui::Hsla,
        ink: gpui::Hsla,
        border: gpui::Hsla,
        accent: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let index = input_index(target);
        // Render already holds this entity mutably. Capture the required state
        // directly; input_editor must not read GpuiCalc back through the entity.
        let view = cx.entity();
        let on_mouse_down = cx.listener(
            move |this: &mut GpuiCalc,
                  event: &MouseDownEvent,
                  window: &mut Window,
                  cx: &mut Context<GpuiCalc>| {
                this.on_input_click(target, event, window, cx);
            },
        );
        let on_mouse_up = cx.listener(
            move |this: &mut GpuiCalc,
                  _: &MouseUpEvent,
                  _: &mut Window,
                  cx: &mut Context<GpuiCalc>| {
                this.stop_input_selection(target, cx);
            },
        );
        let on_mouse_up_out = cx.listener(
            move |this: &mut GpuiCalc,
                  _: &MouseUpEvent,
                  _: &mut Window,
                  cx: &mut Context<GpuiCalc>| {
                this.stop_input_selection(target, cx);
            },
        );
        input_editor(
            view,
            target,
            label,
            self.input_text(target).unwrap_or_default(),
            &self.selections[index],
            self.marked_ranges[index].clone(),
            self.focused_input == Some(target),
            self.focus_handle.clone(),
            self.selecting_input == Some(target),
            self.input_layout.clone(),
            panel,
            ink,
            border,
            accent,
            on_mouse_down,
            on_mouse_up,
            on_mouse_up_out,
        )
    }

    fn render_date(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.lang.text();
        let (bg, panel, ink, muted, border, accent) = colors(dark);
        let mut children = vec![self.render_input(
            InputTarget::Date(DateField::From),
            t.from,
            panel,
            ink,
            border,
            accent,
            cx,
        )];

        match self.date.mode() {
            DateMode::Difference => children.push(self.render_input(
                InputTarget::Date(DateField::To),
                t.to,
                panel,
                ink,
                border,
                accent,
                cx,
            )),
            DateMode::Shift => {
                for (field, label) in [
                    (DateField::Years, t.years),
                    (DateField::Months, t.months),
                    (DateField::Days, t.days),
                ] {
                    children.push(self.render_input(
                        InputTarget::Date(field),
                        label,
                        panel,
                        ink,
                        border,
                        accent,
                        cx,
                    ));
                }
            }
        }

        let fields = div()
            .grid()
            .grid_cols(if self.date.mode() == DateMode::Shift {
                2
            } else {
                1
            })
            .gap(px(10.))
            .children(children);
        let mode_buttons = div()
            .flex()
            .gap(px(8.))
            .child(toggle_button(
                t.date_diff_mode,
                self.date.mode() == DateMode::Difference,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.date.set_mode(DateMode::Difference);
                        this.clear_hidden_focus();
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                t.date_shift_mode,
                self.date.mode() == DateMode::Shift,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.date.set_mode(DateMode::Shift);
                        this.clear_hidden_focus();
                        cx.notify();
                    },
                ),
            ));
        let mut content = div().flex().flex_col().gap(px(14.)).child(mode_buttons);

        if self.date.mode() == DateMode::Shift {
            content = content.child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(toggle_button(
                        t.add,
                        self.date.operation() == DateOperation::Add,
                        dark,
                        cx.listener(
                            |this: &mut Self,
                             _: &MouseDownEvent,
                             _: &mut Window,
                             cx: &mut Context<Self>| {
                                this.date.set_operation(DateOperation::Add);
                                cx.notify();
                            },
                        ),
                    ))
                    .child(toggle_button(
                        t.subtract,
                        self.date.operation() == DateOperation::Subtract,
                        dark,
                        cx.listener(
                            |this: &mut Self,
                             _: &MouseDownEvent,
                             _: &mut Window,
                             cx: &mut Context<Self>| {
                                this.date.set_operation(DateOperation::Subtract);
                                cx.notify();
                            },
                        ),
                    )),
            );
        }

        content = content.child(fields).child(action_button(
            t.calculate,
            true,
            dark,
            cx.listener(
                |this: &mut Self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>| {
                    this.date.calculate();
                    cx.notify();
                },
            ),
        ));

        if let Some(result) = self.date.result() {
            let result_text = match result {
                Ok(DateOutcome::Difference {
                    years,
                    months,
                    weeks,
                    days,
                    total_days,
                }) => format!(
                    "{}\n{}: {total_days} {}",
                    rustic_calc_app::i18n::date_parts(self.lang, years, months, weeks, days),
                    t.diff_days,
                    t.days_suffix
                ),
                Ok(DateOutcome::Shifted(date)) => {
                    format!(
                        "{}: {}",
                        t.result_date,
                        rustic_calc_app::units::format_date(date)
                    )
                }
                Err(DateError::InvalidDate(_)) | Err(DateError::InvalidOffset(_)) => {
                    t.invalid_date.to_owned()
                }
                Err(DateError::OutOfBounds) => t.date_out_of_bound.to_owned(),
            };
            content = content.child(
                div()
                    .p(px(12.))
                    .rounded_md()
                    .bg(panel)
                    .text_color(ink)
                    .child(result_text),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .bg(bg)
            .child(div().text_size(px(20.)).text_color(ink).child(t.date_title))
            .child(div().text_color(muted).child(match self.lang {
                Lang::Ru => "Tab / Shift+Tab · Enter: вычислить · Ctrl+1…0: режимы",
                Lang::En => "Tab / Shift+Tab · Enter: calculate · Ctrl+1…0: modes",
            }))
            .child(content)
            .into_any()
    }

    fn render_unit(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.lang.text();
        let (bg, panel, ink, muted, border, accent) = colors(dark);
        let categories = self
            .units
            .categories()
            .iter()
            .map(|category| {
                let id = category.id;
                toggle_button(
                    rustic_calc_app::i18n::category(self.lang, category.name),
                    self.units.category_id() == id,
                    dark,
                    cx.listener(
                        move |this: &mut Self,
                              _: &MouseDownEvent,
                              _: &mut Window,
                              cx: &mut Context<Self>| {
                            let _ = this.units.select_category(id);
                            this.clear_hidden_focus();
                            cx.notify();
                        },
                    ),
                )
            })
            .collect::<Vec<_>>();
        let category_row = div().flex().flex_wrap().gap(px(5.)).children(categories);
        let category = self.units.category();
        let units = &category.units;
        let from_index = self.units.from_index().get();
        let to_index = self.units.to_index().get();
        let from_name = rustic_calc_app::i18n::unit(self.lang, units[from_index].name);
        let to_name = rustic_calc_app::i18n::unit(self.lang, units[to_index].name);
        let from_picker = unit_picker(
            t.from,
            from_name,
            (from_index + units.len() - 1) % units.len(),
            (from_index + 1) % units.len(),
            dark,
            cx,
            |this, index| {
                let _ = this.units.select_from(index);
            },
        );
        let to_picker = unit_picker(
            t.to,
            to_name,
            (to_index + units.len() - 1) % units.len(),
            (to_index + 1) % units.len(),
            dark,
            cx,
            |this, index| {
                let _ = this.units.select_to(index);
            },
        );
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(div().text_color(muted).child(t.unit_title))
            .child(category_row)
            .child(from_picker)
            .child(to_picker)
            .child(self.render_input(
                InputTarget::Unit(UnitField::Value),
                from_name,
                panel,
                ink,
                border,
                accent,
                cx,
            ));

        let conversion = match self.units.conversion_result() {
            ConversionResult::Empty => "—".to_owned(),
            ConversionResult::InvalidInput => t.field_number.to_owned(),
            ConversionResult::Value(value) => {
                rustic_calc_app::engine::format_number(value, self.grouping)
            }
        };
        content = content.child(
            div()
                .p(px(10.))
                .rounded_md()
                .bg(panel)
                .text_color(ink)
                .child(format!("{}: {conversion}", t.result_date)),
        );

        if let Some(loop_result) = self.units.dcs_loop_result() {
            let loop_content = match loop_result {
                Ok(value) => format!(
                    "R: {} Ω · P: {} W · E24: {} Ω · I: {} mA · ≥ {} W",
                    rustic_calc_app::engine::format_number(value.resistance, self.grouping),
                    rustic_calc_app::engine::format_number(value.power, self.grouping),
                    rustic_calc_app::engine::format_number(
                        value.standard_resistance,
                        self.grouping
                    ),
                    rustic_calc_app::engine::format_number(value.actual_current_ma, self.grouping),
                    rustic_calc_app::engine::format_number(
                        value.recommended_wattage,
                        self.grouping
                    ),
                ),
                Err(DcsLoopError::InvalidVoltage | DcsLoopError::InvalidCurrent) => {
                    t.field_number.to_owned()
                }
                Err(DcsLoopError::NonPositiveVoltage | DcsLoopError::NonPositiveCurrent) => {
                    t.loop_positive_values.to_owned()
                }
                Err(DcsLoopError::OutOfRange) => t.error_unable.to_owned(),
            };
            content = content
                .child(self.render_input(
                    InputTarget::Unit(UnitField::LoopVoltage),
                    t.dcs_loop_voltage,
                    panel,
                    ink,
                    border,
                    accent,
                    cx,
                ))
                .child(self.render_input(
                    InputTarget::Unit(UnitField::LoopCurrentMa),
                    t.dcs_loop_current,
                    panel,
                    ink,
                    border,
                    accent,
                    cx,
                ))
                .child(
                    div()
                        .p(px(10.))
                        .rounded_md()
                        .bg(panel)
                        .text_color(ink)
                        .child(loop_content),
                );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .bg(bg)
            .child(div().text_color(muted).child(match self.lang {
                Lang::Ru => "Tab / Shift+Tab · Ctrl+5 / Ctrl+U: преобразование единиц",
                Lang::En => "Tab / Shift+Tab · Ctrl+5 / Ctrl+U: unit conversion",
            }))
            .child(content)
            .into_any()
    }

    fn render_worksheet(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.lang.text();
        let (bg, panel, ink, muted, border, accent) = colors(dark);
        let Some(session) = self.active_worksheet() else {
            return div().into_any();
        };
        let target_options = session
            .target_options()
            .iter()
            .copied()
            .map(|field| {
                toggle_button(
                    field.label(self.lang),
                    session.target() == field,
                    dark,
                    cx.listener(
                        move |this: &mut Self,
                              _: &MouseDownEvent,
                              _: &mut Window,
                              cx: &mut Context<Self>| {
                            if let Some(session) = this.active_worksheet_mut() {
                                session.set_target(field);
                            }
                            cx.notify();
                        },
                    ),
                )
            })
            .collect::<Vec<_>>();
        let fields = session
            .fields()
            .iter()
            .copied()
            .map(|field| {
                self.render_input(
                    InputTarget::Worksheet(field),
                    field.label(self.lang),
                    panel,
                    ink,
                    border,
                    accent,
                    cx,
                )
            })
            .collect::<Vec<_>>();
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(div().text_color(muted).child(t.worksheet_title))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(5.))
                    .children(target_options),
            )
            .child(div().grid().grid_cols(2).gap(px(10.)).children(fields))
            .child(action_button(
                t.calculate,
                true,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        if let Some(session) = this.active_worksheet_mut() {
                            let _ = session.calculate();
                        }
                        cx.notify();
                    },
                ),
            ));
        if let Some(session) = self.active_worksheet() {
            if let Some(result) = session.result() {
                let result_text = match result {
                    Ok(outcome) => format!(
                        "{}: {}",
                        outcome.target.label(self.lang),
                        session.formatted_result(self.grouping).unwrap_or_default()
                    ),
                    Err(_) => session.display_error(self.lang).unwrap_or_default(),
                };
                content = content.child(
                    div()
                        .p(px(10.))
                        .rounded_md()
                        .bg(panel)
                        .text_color(ink)
                        .child(result_text),
                );
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .bg(bg)
            .child(div().text_color(muted).child(match self.lang {
                Lang::Ru => "Tab / Shift+Tab · Enter: вычислить · Ctrl+7…0: таблицы",
                Lang::En => "Tab / Shift+Tab · Enter: calculate · Ctrl+7…0: worksheets",
            }))
            .child(content)
            .into_any()
    }

    fn render_coordinates(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.lang.text();
        let ct = rustic_calc_app::i18n::coordinates::text(self.lang);
        let (bg, panel, ink, muted, border, accent) = colors(dark);
        if self.show_coordinate_help {
            return self.render_coordinate_help(dark, cx);
        }
        let regions = CoordinateSession::regions();
        let region_index = regions
            .iter()
            .position(|region| region.id == self.coordinates.region().as_str())
            .unwrap_or(0);
        let prev_region = regions[(region_index + regions.len() - 1) % regions.len()].id;
        let next_region = regions[(region_index + 1) % regions.len()].id;
        let region_name = regions[region_index].name(self.lang);
        let region_picker = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(toggle_button(
                "‹",
                false,
                dark,
                cx.listener(
                    move |this: &mut Self,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<Self>| {
                        let _ = this.coordinates.select_region(prev_region);
                        cx.notify();
                    },
                ),
            ))
            .child(div().text_color(ink).child(region_name))
            .child(toggle_button(
                "›",
                false,
                dark,
                cx.listener(
                    move |this: &mut Self,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<Self>| {
                        let _ = this.coordinates.select_region(next_region);
                        cx.notify();
                    },
                ),
            ));

        let templates = self.coordinates.templates();
        let selected_index = match self.coordinates.selection() {
            CoordinateTemplateSelection::Custom => 0,
            CoordinateTemplateSelection::Template(id) => templates
                .iter()
                .position(|template| template.id == *id)
                .map(|index| index + 1)
                .unwrap_or(0),
        };
        let template_count = templates.len() + 1;
        let prev_template = (selected_index + template_count - 1) % template_count;
        let next_template = (selected_index + 1) % template_count;
        let template_label = if selected_index == 0 {
            ct.custom.to_owned()
        } else {
            rustic_calc_app::i18n::coordinates::template_name(
                self.lang,
                templates[selected_index - 1].name,
            )
        };
        let template_picker = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(toggle_button(
                "‹",
                false,
                dark,
                cx.listener(
                    move |this: &mut Self,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<Self>| {
                        this.select_coordinate_template(prev_template);
                        cx.notify();
                    },
                ),
            ))
            .child(div().text_color(ink).child(template_label))
            .child(toggle_button(
                "›",
                false,
                dark,
                cx.listener(
                    move |this: &mut Self,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<Self>| {
                        this.select_coordinate_template(next_template);
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                ct.reset,
                false,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.coordinates.reset_definition();
                        cx.notify();
                    },
                ),
            ));
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(div().text_color(muted).child(ct.region))
            .child(region_picker)
            .child(div().text_color(muted).child(ct.template))
            .child(template_picker)
            .child(toggle_button(
                ct.info,
                false,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.toggle_coordinate_help();
                        cx.notify();
                    },
                ),
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap(px(10.))
                    .child(self.render_input(
                        InputTarget::Coordinate(CoordinateInputField::Latitude),
                        ct.latitude,
                        panel,
                        ink,
                        border,
                        accent,
                        cx,
                    ))
                    .child(self.render_input(
                        InputTarget::Coordinate(CoordinateInputField::Longitude),
                        ct.longitude,
                        panel,
                        ink,
                        border,
                        accent,
                        cx,
                    ))
                    .child(self.render_input(
                        InputTarget::Coordinate(CoordinateInputField::Height),
                        ct.height,
                        panel,
                        ink,
                        border,
                        accent,
                        cx,
                    )),
            )
            .child(self.render_input(
                InputTarget::CoordinateDefinition,
                ct.definition,
                panel,
                ink,
                border,
                accent,
                cx,
            ))
            .child(action_button(
                t.calculate,
                true,
                dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        let _ = this.coordinates.calculate();
                        cx.notify();
                    },
                ),
            ))
            .child(
                div().text_color(muted).child(
                    self.coordinates
                        .selected_template()
                        .map(|template| {
                            if self.coordinates.definition() == template.definition {
                                ct.original
                            } else {
                                ct.modified
                            }
                        })
                        .unwrap_or(ct.custom_status),
                ),
            )
            .child(div().text_color(muted).child(ct.accuracy))
            .child(div().text_color(muted).child(ct.session));

        if templates.is_empty() && selected_index == 0 {
            content = content.child(div().text_color(muted).child(ct.no_templates));
        }
        if let Some(result) = self.coordinates.result() {
            let result_text = match result {
                Ok(output) => format!(
                    "{}: {} · {}: {}",
                    ct.northing,
                    rustic_calc_app::engine::format_number(output.northing_m, self.grouping),
                    ct.easting,
                    rustic_calc_app::engine::format_number(output.easting_m, self.grouping)
                ),
                Err(error) => rustic_calc_app::i18n::coordinates::error(self.lang, error),
            };
            content = content.child(
                div()
                    .p(px(10.))
                    .rounded_md()
                    .bg(panel)
                    .text_color(ink)
                    .child(result_text),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .bg(bg)
            .child(div().text_color(muted).child(match self.lang {
                Lang::Ru => "Tab / Shift+Tab · Enter: вычислить · Ctrl+G: координаты",
                Lang::En => "Tab / Shift+Tab · Enter: calculate · Ctrl+G: coordinates",
            }))
            .child(content)
            .into_any()
    }

    fn render_coordinate_help(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (_, panel, ink, _, border, _) = colors(dark);
        let ct = rustic_calc_app::i18n::coordinates::text(self.lang);
        let help = coordinate_help_text(self.lang);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(20.)).text_color(ink).child(ct.title))
                    .child(toggle_button(
                        ct.back,
                        true,
                        dark,
                        cx.listener(
                            |this: &mut Self,
                             _: &MouseDownEvent,
                             _: &mut Window,
                             cx: &mut Context<Self>| {
                                this.toggle_coordinate_help();
                                cx.notify();
                            },
                        ),
                    )),
            )
            .child(
                div()
                    .id("coordinate-help-scroll")
                    .flex_grow()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .p(px(12.))
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .bg(panel)
                    .text_color(ink)
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .child(help),
            )
            .into_any()
    }

    fn select_coordinate_template(&mut self, index: usize) {
        if index == 0 {
            self.coordinates.select_custom();
        } else if let Some(template) = self.coordinates.templates().get(index - 1) {
            let _ = self.coordinates.select_template(template.id.clone());
        }
    }

    fn render_calculator(&self, dark: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        match self.mode {
            Mode::UnitConversion => return self.render_unit(dark, cx),
            Mode::Worksheet(_) => return self.render_worksheet(dark, cx),
            Mode::Coordinates => return self.render_coordinates(dark, cx),
            _ => {}
        }
        let (bg, panel, ink, muted, border, _accent) = colors(dark);

        let (display, expression) = match self.mode {
            Mode::Standard | Mode::Scientific => {
                let arithmetic = &self.arithmetic
                    [arithmetic_index(self.mode).expect("arithmetic mode has a session")];
                (
                    arithmetic.display(self.lang, self.grouping),
                    arithmetic.expression(self.grouping),
                )
            }
            Mode::Programmer => (
                self.programmer.display(self.lang),
                self.programmer.state.expression(),
            ),
            Mode::Statistics => (self.stats.display(), String::new()),
            _ => unreachable!(),
        };

        let mut content = div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .bg(bg)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .justify_center()
                    .h(px(104.))
                    .px(px(14.))
                    .rounded_md()
                    .bg(panel)
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .w_full()
                            .text_right()
                            .text_color(muted)
                            .text_size(px(16.))
                            .child(expression),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_right()
                            .text_color(ink)
                            .text_size(px(34.))
                            .child(display),
                    ),
            );

        if self.mode == Mode::Programmer {
            let base = self.programmer.state.base;
            let word = self.programmer.state.word;
            content = content
                .child(programmer_bit_rows(
                    self.programmer.state.value,
                    word,
                    dark,
                    cx,
                ))
                .child(div().text_color(muted).child(self.lang.text().base))
                .child(selector_row(
                    &[
                        (16, self.lang.text().hex),
                        (10, self.lang.text().dec),
                        (8, self.lang.text().oct),
                        (2, self.lang.text().bin),
                    ],
                    Some(base),
                    dark,
                    cx,
                    |this, value| this.apply(Action::ProgBase(value as u8)),
                ))
                .child(div().text_color(muted).child(self.lang.text().word_size))
                .child(selector_row(
                    &[
                        (64, self.lang.text().qword),
                        (32, self.lang.text().dword),
                        (16, self.lang.text().word),
                        (8, self.lang.text().byte),
                    ],
                    Some(word),
                    dark,
                    cx,
                    |this, value| this.apply(Action::ProgWord(value as u8)),
                ));
        }

        if self.mode == Mode::Statistics {
            content = content.child(div().text_color(muted).child(format!(
                "{}: {}",
                self.lang.text().count,
                self.stats.count()
            )));
        }

        let mut rows = div()
            .grid()
            .grid_cols(5)
            .gap(px(6.))
            .flex_grow()
            .content_start();
        if self.mode == Mode::Scientific {
            let sci = actions::scientific_keys();
            for key in sci {
                rows = rows.child(key_button(key, dark, cx));
            }
        }
        let keys = match self.mode {
            Mode::Standard => actions::standard_keys(),
            Mode::Scientific => actions::standard_keys(),
            Mode::Programmer => actions::programmer_keys(),
            Mode::Statistics => actions::stats_keys(self.lang.text().stats_add),
            _ => Vec::new(),
        };
        for key in keys {
            rows = rows.child(key_button(key, dark, cx));
        }
        content = content.child(rows);

        let footer = div().flex().gap(px(12.)).child(toggle_button(
            self.lang.text().menu_grouping,
            self.grouping,
            dark,
            cx.listener(
                |this: &mut Self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>| {
                    this.grouping = !this.grouping;
                    cx.notify();
                },
            ),
        ));
        content.child(footer).into_any()
    }
}

impl EntityInputHandler for GpuiCalc {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.input_text(self.focused_input?)?;
        let range = text_input::utf16_range_to_utf8(text, range_utf16);
        *adjusted_range = Some(text_input::utf8_range_to_utf16(text, range.clone()));
        text.get(range).map(str::to_owned)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let target = self.focused_input?;
        let selection = &self.selections[input_index(target)];
        let text = self.input_text(target)?;
        Some(UTF16Selection {
            range: text_input::utf8_range_to_utf16(text, selection.range.clone()),
            reversed: selection.reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let target = self.focused_input?;
        let text = self.input_text(target)?;
        self.marked_ranges[input_index(target)]
            .as_ref()
            .map(|range| text_input::utf8_range_to_utf16(text, range.clone()))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.apply_input_edit(InputEdit::Unmark, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_input_edit(
            InputEdit::Commit {
                range_utf16,
                text: text.to_owned(),
            },
            cx,
        );
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_input_edit(
            InputEdit::Mark {
                replacement_range_utf16: range_utf16,
                text: new_text.to_owned(),
                selected_range_utf16: new_selected_range,
            },
            cx,
        );
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let target = self.focused_input?;
        let text = self.input_text(target)?;
        let range = text_input::utf16_range_to_utf8(text, range_utf16);
        let layout = self.input_layout.borrow();
        let line = layout.lines[input_index(target)].as_ref()?;
        let start_x = line.x_for_index(range.start);
        let end_x = line.x_for_index(range.end);
        let left = if start_x < end_x { start_x } else { end_x };
        let width = if start_x < end_x {
            end_x - start_x
        } else {
            start_x - end_x
        }
        .max(px(1.));
        Some(Bounds::new(
            point(element_bounds.left() + left, element_bounds.top()),
            size(width, element_bounds.size.height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let target = self.focused_input?;
        let layout = self.input_layout.borrow();
        let bounds = layout.bounds[input_index(target)]?;
        let text = self.input_text(target)?;
        let x = if point.x < bounds.left() {
            px(0.)
        } else if point.x > bounds.right() {
            bounds.size.width
        } else {
            point.x - bounds.left()
        };
        let line = layout.lines[input_index(target)].as_ref()?;
        let byte_index = text_input::clamp_utf8_offset(text, line.closest_index_for_x(x), false);
        Some(text_input::utf8_range_to_utf16(text, byte_index..byte_index).start)
    }
}

impl Focusable for GpuiCalc {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GpuiCalc {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = match self.theme {
            ThemeMode::System => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
            ThemeMode::White => false,
            ThemeMode::Dark => true,
        };
        let (bg, panel, ink, _, border, _) = colors(is_dark);
        let t = self.lang.text();
        let title = startup::mode_title(self.mode, self.lang);

        let mode_bar =
            div()
                .flex()
                .flex_wrap()
                .gap(px(5.))
                .children(startup::MODES.iter().copied().map(|mode| {
                    let active = self.mode == mode;
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .rounded_md()
                        .bg(if active { border } else { panel })
                        .text_color(ink)
                        .text_size(px(13.))
                        .child(startup::mode_title(mode, self.lang))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(
                                move |this: &mut Self,
                                      _: &MouseDownEvent,
                                      _: &mut Window,
                                      cx: &mut Context<Self>| {
                                    this.set_mode(mode);
                                    cx.notify();
                                },
                            ),
                        )
                }));

        let options = div()
            .flex()
            .gap(px(8.))
            .child(toggle_button(
                t.lang_russian,
                self.lang == Lang::Ru,
                is_dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.lang = Lang::Ru;
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                t.lang_english,
                self.lang == Lang::En,
                is_dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.lang = Lang::En;
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                t.theme_system,
                self.theme == ThemeMode::System,
                is_dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.theme = ThemeMode::System;
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                t.theme_white,
                self.theme == ThemeMode::White,
                is_dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.theme = ThemeMode::White;
                        cx.notify();
                    },
                ),
            ))
            .child(toggle_button(
                t.theme_dark,
                self.theme == ThemeMode::Dark,
                is_dark,
                cx.listener(
                    |this: &mut Self,
                     _: &MouseDownEvent,
                     _: &mut Window,
                     cx: &mut Context<Self>| {
                        this.theme = ThemeMode::Dark;
                        cx.notify();
                    },
                ),
            ));

        let body = if self.mode == Mode::DateCalculation {
            self.render_date(is_dark, cx)
        } else {
            self.render_calculator(is_dark, cx)
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(14.))
            .bg(bg)
            .text_color(ink)
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(20.))
                            .child(format!("{} · {title}", t.app_title)),
                    )
                    .child(options),
            )
            .child(mode_bar)
            .child(
                div()
                    .id("calculator-content-scroll")
                    .flex_grow()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .child(body),
            )
    }
}

fn worksheet_index(mode: Mode) -> Option<usize> {
    match mode {
        Mode::Worksheet(WorksheetKind::Mortgage) => Some(0),
        Mode::Worksheet(WorksheetKind::VehicleLease) => Some(1),
        Mode::Worksheet(WorksheetKind::FuelMpg) => Some(2),
        Mode::Worksheet(WorksheetKind::FuelL100) => Some(3),
        _ => None,
    }
}

fn arithmetic_index(mode: Mode) -> Option<usize> {
    match mode {
        Mode::Standard => Some(0),
        Mode::Scientific => Some(1),
        _ => None,
    }
}

fn mode_shortcut_key(key: &str) -> Option<u8> {
    let bytes = key.as_bytes();
    match bytes {
        [key] if key.is_ascii_alphanumeric() => Some(*key),
        _ => None,
    }
}

fn mode_for_keystroke(key: &str) -> Option<Mode> {
    mode_shortcut_key(key).and_then(startup::mode_shortcut)
}

fn coordinate_help_text(lang: Lang) -> String {
    let text = rustic_calc_app::i18n::coordinates::text(lang);
    format!(
        "{}\n\n{}: {}\n{}: {}\n\n{}",
        text.help.replace("\r\n", "\n"),
        text.source,
        rustic_calc_app::coordinates::catalogue::SOURCE,
        text.revision,
        rustic_calc_app::coordinates::catalogue::REVISION,
        rustic_calc_app::coordinates::catalogue::LICENSE,
    )
}

fn input_index(target: InputTarget) -> usize {
    match target {
        InputTarget::Date(DateField::From) => 0,
        InputTarget::Date(DateField::To) => 1,
        InputTarget::Date(DateField::Years) => 2,
        InputTarget::Date(DateField::Months) => 3,
        InputTarget::Date(DateField::Days) => 4,
        InputTarget::Unit(UnitField::Value) => 5,
        InputTarget::Unit(UnitField::LoopVoltage) => 6,
        InputTarget::Unit(UnitField::LoopCurrentMa) => 7,
        InputTarget::Worksheet(WorksheetField::PurchasePrice) => 8,
        InputTarget::Worksheet(WorksheetField::DownPayment) => 9,
        InputTarget::Worksheet(WorksheetField::TermYears) => 10,
        InputTarget::Worksheet(WorksheetField::InterestRate) => 11,
        InputTarget::Worksheet(WorksheetField::MonthlyPayment) => 12,
        InputTarget::Worksheet(WorksheetField::LeaseValue) => 13,
        InputTarget::Worksheet(WorksheetField::LeasePeriod) => 14,
        InputTarget::Worksheet(WorksheetField::PaymentsPerYear) => 15,
        InputTarget::Worksheet(WorksheetField::ResidualValue) => 16,
        InputTarget::Worksheet(WorksheetField::PeriodicPayment) => 17,
        InputTarget::Worksheet(WorksheetField::DistanceMiles) => 18,
        InputTarget::Worksheet(WorksheetField::FuelGallons) => 19,
        InputTarget::Worksheet(WorksheetField::FuelEconomyMpg) => 20,
        InputTarget::Worksheet(WorksheetField::DistanceKilometers) => 21,
        InputTarget::Worksheet(WorksheetField::FuelLiters) => 22,
        InputTarget::Worksheet(WorksheetField::FuelEconomyL100Km) => 23,
        InputTarget::Coordinate(CoordinateInputField::Latitude) => 24,
        InputTarget::Coordinate(CoordinateInputField::Longitude) => 25,
        InputTarget::Coordinate(CoordinateInputField::Height) => 26,
        InputTarget::CoordinateDefinition => 27,
    }
}

fn previous_boundary(text: &str, index: usize) -> usize {
    text.char_indices()
        .map(|(at, _)| at)
        .take_while(|&at| at < index)
        .last()
        .unwrap_or(0)
}

fn next_boundary(text: &str, index: usize) -> usize {
    text.char_indices()
        .map(|(at, ch)| at + ch.len_utf8())
        .find(|&at| at > index)
        .unwrap_or(text.len())
}

fn shape_input_line_with_mark(
    text: &str,
    marked_range: Option<Range<usize>>,
    window: &Window,
) -> gpui::ShapedLine {
    let style = window.text_style();
    let base = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let runs = if let Some(range) = marked_range {
        let start = text_input::clamp_utf8_offset(text, range.start, false);
        let end = text_input::clamp_utf8_offset(text, range.end, true);
        if start < end {
            vec![
                TextRun {
                    len: start,
                    ..base.clone()
                },
                TextRun {
                    len: end - start,
                    underline: Some(UnderlineStyle {
                        color: Some(base.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..base.clone()
                },
                TextRun {
                    len: text.len() - end,
                    ..base
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![base]
        }
    } else {
        vec![base]
    };
    window.text_system().shape_line(
        text.to_owned().into(),
        style.font_size.to_pixels(window.rem_size()),
        &runs,
        None,
    )
}

fn input_editor(
    view: gpui::Entity<GpuiCalc>,
    target: InputTarget,
    label: &'static str,
    text: &str,
    selection: &Selection,
    marked_range: Option<Range<usize>>,
    focused: bool,
    focus_handle: FocusHandle,
    selecting: bool,
    layout: Rc<RefCell<InputLayout>>,
    panel: gpui::Hsla,
    ink: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
    on_mouse_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    on_mouse_up: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    on_mouse_up_out: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    let index = input_index(target);
    let text = text.to_owned();
    let reversed = selection.reversed;
    let selection = selection.range.clone();
    let paint_view = view;
    let paint_layout = layout;
    let paint_text = text.clone();
    let paint_marked_range = marked_range.clone();
    let paint_selection = selection.clone();
    let canvas = canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let line = shape_input_line_with_mark(&paint_text, paint_marked_range, window);
            let selection = text_input::clamp_utf8_offset(&paint_text, paint_selection.start, false)
                ..text_input::clamp_utf8_offset(&paint_text, paint_selection.end, true);
            let line_height = window.line_height();
            let line_top = bounds.top() + ((bounds.size.height - line_height) / 2.).max(px(0.));
            let origin = point(bounds.left(), line_top);

            if selection.start < selection.end {
                let start = line.x_for_index(selection.start);
                let end = line.x_for_index(selection.end);
                window.paint_quad(fill(
                    Bounds::from_corners(
                        point(bounds.left() + start, line_top),
                        point(bounds.left() + end, line_top + line_height),
                    ),
                    accent.opacity(0.35),
                ));
            }

            if focused {
                window.handle_input(
                    &focus_handle,
                    ElementInputHandler::new(bounds, paint_view.clone()),
                    cx,
                );
            }
            let _ = line.paint(origin, line_height, window, cx);

            if focused && selection.is_empty() {
                let cursor = if reversed {
                    selection.start
                } else {
                    selection.end
                };
                let cursor_x = line.x_for_index(cursor);
                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, line_top),
                        size(px(1.5), line_height),
                    ),
                    accent,
                ));
            }

            {
                let mut layout = paint_layout.borrow_mut();
                layout.bounds[index] = Some(bounds);
                layout.lines[index] = Some(line.clone());
            }

            if selecting {
                // These closures are invoked only on subsequent input events,
                // not while this frame is being rendered or painted.
                let move_view = paint_view.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let _ = move_view.update(cx, |this, cx| {
                        if this.selecting_input == Some(target)
                            && let Some(cursor) =
                                this.input_byte_index_for_point(target, event.position)
                        {
                            this.selections[index].select_to(cursor);
                            cx.notify();
                        }
                    });
                });
                let up_view = paint_view.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
                    if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                        return;
                    }
                    let _ = up_view.update(cx, |this, cx| {
                        if this.selecting_input == Some(target) {
                            this.selecting_input = None;
                            cx.notify();
                        }
                    });
                });
            }
        },
    );
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(div().text_size(px(13.)).child(label))
        .child(
            div()
                .w_full()
                .h(px(42.))
                .px(px(10.))
                .rounded_md()
                .overflow_x_hidden()
                .border_1()
                .border_color(if focused { accent } else { border })
                .bg(panel)
                .text_color(ink)
                .text_size(px(16.))
                .line_height(px(20.))
                .items_center()
                .cursor(gpui::CursorStyle::IBeam)
                .child(canvas.w_full().h_full())
                .on_mouse_down(MouseButton::Left, on_mouse_down)
                .on_mouse_up(MouseButton::Left, on_mouse_up)
                .on_mouse_up_out(MouseButton::Left, on_mouse_up_out),
        )
        .into_any()
}

fn programmer_bit_rows(
    value: u64,
    word: u8,
    dark: bool,
    cx: &mut Context<GpuiCalc>,
) -> gpui::AnyElement {
    let active_bits = u32::from(word);
    let mut high = div().grid().grid_cols(32).gap(px(2.));
    for bit in (32..64).rev() {
        high = high.child(programmer_bit_cell(bit, active_bits, value, dark, cx));
    }
    let mut low = div().grid().grid_cols(32).gap(px(2.));
    for bit in (0..32).rev() {
        low = low.child(programmer_bit_cell(bit, active_bits, value, dark, cx));
    }
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(high)
        .child(low)
        .into_any()
}

fn programmer_bit_cell(
    bit: u32,
    active_bits: u32,
    value: u64,
    dark: bool,
    cx: &mut Context<GpuiCalc>,
) -> gpui::AnyElement {
    let (_, panel, ink, muted, _border, accent) = colors(dark);
    let enabled = bit < active_bits;
    let is_on = enabled && value & (1u64 << bit) != 0;
    let label = if !enabled {
        " "
    } else if is_on {
        "1"
    } else {
        "0"
    };
    let cell = div()
        .min_h(px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(if is_on { accent } else { panel })
        .text_color(if enabled { ink } else { muted })
        .text_size(px(10.))
        .child(label);
    if enabled {
        cell.on_mouse_down(
            MouseButton::Left,
            cx.listener(
                move |this: &mut GpuiCalc,
                      _: &MouseDownEvent,
                      _: &mut Window,
                      cx: &mut Context<GpuiCalc>| {
                    this.apply(Action::ProgBit(bit));
                    cx.notify();
                },
            ),
        )
        .into_any()
    } else {
        cell.into_any()
    }
}

fn key_button(key: KeySpec, dark: bool, cx: &mut Context<GpuiCalc>) -> gpui::AnyElement {
    let (bg, panel, ink, _, border, accent) = colors(dark);
    let key_bg = match key.style {
        KeyStyle::Normal => panel,
        KeyStyle::Operator => border,
        KeyStyle::Equals => accent,
        KeyStyle::Memory => bg,
        KeyStyle::Blank => bg,
    };
    let text = key.label;
    let enabled = key.enabled;
    let action = key.action;
    div()
        .col_span(key.span_x as u16)
        .row_span(key.span_y as u16)
        .min_h(px(38.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(key_bg)
        .text_color(ink)
        .text_size(px(16.))
        .child(text)
        .when(enabled, |this| {
            this.hover(|style| style.opacity(0.82)).on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    move |this: &mut GpuiCalc,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<GpuiCalc>| {
                        this.apply(action);
                        cx.notify();
                    },
                ),
            )
        })
        .into_any()
}

fn toggle_button(
    label: impl IntoElement + 'static,
    selected: bool,
    dark: bool,
    on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (_, panel, ink, _, border, accent) = colors(dark);
    div()
        .px(px(9.))
        .py(px(6.))
        .rounded_md()
        .border_1()
        .border_color(if selected { accent } else { border })
        .bg(if selected { accent } else { panel })
        .text_color(ink)
        .text_size(px(12.))
        .child(label)
        .on_mouse_down(MouseButton::Left, on_click)
}

fn action_button(
    label: impl IntoElement + 'static,
    selected: bool,
    dark: bool,
    on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    toggle_button(label, selected, dark, on_click)
}

fn unit_picker<F>(
    label: &'static str,
    selected: &'static str,
    previous: usize,
    next: usize,
    dark: bool,
    cx: &mut Context<GpuiCalc>,
    action: F,
) -> gpui::AnyElement
where
    F: Fn(&mut GpuiCalc, UnitIndex) + Copy + 'static,
{
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().w(px(80.)).child(label))
        .child(toggle_button(
            "‹",
            false,
            dark,
            cx.listener(
                move |this: &mut GpuiCalc,
                      _: &MouseDownEvent,
                      _: &mut Window,
                      cx: &mut Context<GpuiCalc>| {
                    action(this, UnitIndex::new(previous));
                    cx.notify();
                },
            ),
        ))
        .child(
            div()
                .flex_grow()
                .px(px(10.))
                .py(px(7.))
                .rounded_md()
                .bg(colors(dark).1)
                .text_color(colors(dark).2)
                .child(selected),
        )
        .child(toggle_button(
            "›",
            false,
            dark,
            cx.listener(
                move |this: &mut GpuiCalc,
                      _: &MouseDownEvent,
                      _: &mut Window,
                      cx: &mut Context<GpuiCalc>| {
                    action(this, UnitIndex::new(next));
                    cx.notify();
                },
            ),
        ))
        .into_any()
}

fn selector_row<F>(
    options: &[(u8, &'static str)],
    selected: Option<u8>,
    dark: bool,
    cx: &mut Context<GpuiCalc>,
    action: F,
) -> impl IntoElement
where
    F: Fn(&mut GpuiCalc, u8) + Copy + 'static,
{
    div()
        .flex()
        .gap(px(5.))
        .children(options.iter().map(|&(value, label)| {
            let is_selected = selected == Some(value);
            toggle_button(
                label,
                is_selected,
                dark,
                cx.listener(
                    move |this: &mut GpuiCalc,
                          _: &MouseDownEvent,
                          _: &mut Window,
                          cx: &mut Context<GpuiCalc>| {
                        action(this, value);
                        cx.notify();
                    },
                ),
            )
        }))
}

fn colors(
    dark: bool,
) -> (
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
) {
    if dark {
        (
            rgb(0x1e1e1e).into(),
            rgb(0x2b2b2b).into(),
            rgb(0xf2f2f2).into(),
            rgb(0xaaaaaa).into(),
            rgb(0x414141).into(),
            rgb(0x765a45).into(),
        )
    } else {
        (
            rgb(0xf2f2f2).into(),
            rgb(0xffffff).into(),
            rgb(0x222222).into(),
            rgb(0x666666).into(),
            rgb(0xd4d4d4).into(),
            rgb(0xb98d6c).into(),
        )
    }
}

fn local_today() -> Date {
    let mut local_time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut local_time) };
    Date::new(
        i32::from(local_time.wYear),
        u32::from(local_time.wMonth),
        u32::from(local_time.wDay),
    )
    .expect("Windows returned an invalid local calendar date")
}

fn main() {
    let options = LaunchOptions::parse(std::env::args().skip(1));
    let screenshot_path = options.screenshot.clone();
    let app = Application::new();
    app.run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let capture_path = screenshot_path.clone();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| GpuiCalc::new(options, cx)),
            )
            .expect("open GPUI window");
        handle
            .update(cx, move |view, window, cx| {
                window.focus(&view.focus_handle(cx));
                window.set_window_title("Rustic Calc (GPUI)");
                if let Some(path) = capture_path {
                    let error_path = path.clone();
                    window.on_next_frame(move |_, _| {
                        let spawn_result = std::thread::Builder::new()
                            .name("gpui-screenshot".to_owned())
                            .spawn(move || {
                                // Allow the first frame to reach the desktop before
                                // attempting a best-effort native-window capture.
                                std::thread::sleep(Duration::from_millis(300));
                                finish_screenshot(path);
                            });
                        if let Err(error) = spawn_result {
                            finish_screenshot_error(&error_path, &error.to_string());
                        }
                    });
                }
            })
            .expect("initialize GPUI window");
        cx.activate(true);
    });
}

fn finish_screenshot(path: String) -> ! {
    let mut last_error = String::from("no capture attempt completed");
    for attempt in 0..12 {
        match windows_capture::capture_current_process_window_png(&path, "Rustic Calc (GPUI)") {
            Ok(()) => {
                let _ = std::fs::remove_file(screenshot_error_path(&path));
                std::process::exit(0);
            }
            Err(error) => {
                last_error = error;
                if attempt < 11 {
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }
    finish_screenshot_error(&path, &last_error)
}

fn screenshot_error_path(path: &str) -> std::path::PathBuf {
    let mut error_path = std::ffi::OsString::from(path);
    error_path.push(".error");
    error_path.into()
}

fn finish_screenshot_error(path: &str, error: &str) -> ! {
    let sidecar = screenshot_error_path(path);
    let details = format!(
        "GPUI CLI screenshot failed. Native window capture is best-effort and can \
         depend on compositor, visibility, and occlusion.\n\n{error}\n"
    );
    if let Err(sidecar_error) = std::fs::write(&sidecar, details) {
        eprintln!(
            "GPUI CLI screenshot failed: {error}; could not write {}: {sidecar_error}",
            sidecar.display()
        );
    }
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_slots_are_unique_and_cover_the_fixed_selection_store() {
        let targets = [
            InputTarget::Date(DateField::From),
            InputTarget::Date(DateField::To),
            InputTarget::Date(DateField::Years),
            InputTarget::Date(DateField::Months),
            InputTarget::Date(DateField::Days),
            InputTarget::Unit(UnitField::Value),
            InputTarget::Unit(UnitField::LoopVoltage),
            InputTarget::Unit(UnitField::LoopCurrentMa),
            InputTarget::Worksheet(WorksheetField::PurchasePrice),
            InputTarget::Worksheet(WorksheetField::DownPayment),
            InputTarget::Worksheet(WorksheetField::TermYears),
            InputTarget::Worksheet(WorksheetField::InterestRate),
            InputTarget::Worksheet(WorksheetField::MonthlyPayment),
            InputTarget::Worksheet(WorksheetField::LeaseValue),
            InputTarget::Worksheet(WorksheetField::LeasePeriod),
            InputTarget::Worksheet(WorksheetField::PaymentsPerYear),
            InputTarget::Worksheet(WorksheetField::ResidualValue),
            InputTarget::Worksheet(WorksheetField::PeriodicPayment),
            InputTarget::Worksheet(WorksheetField::DistanceMiles),
            InputTarget::Worksheet(WorksheetField::FuelGallons),
            InputTarget::Worksheet(WorksheetField::FuelEconomyMpg),
            InputTarget::Worksheet(WorksheetField::DistanceKilometers),
            InputTarget::Worksheet(WorksheetField::FuelLiters),
            InputTarget::Worksheet(WorksheetField::FuelEconomyL100Km),
            InputTarget::Coordinate(CoordinateInputField::Latitude),
            InputTarget::Coordinate(CoordinateInputField::Longitude),
            InputTarget::Coordinate(CoordinateInputField::Height),
            InputTarget::CoordinateDefinition,
        ];
        let indexes = targets.map(input_index);
        assert_eq!(indexes, std::array::from_fn(|index| index));
    }

    #[test]
    fn editor_selection_tracks_cursor_direction_and_utf8_boundaries() {
        let mut selection = Selection::default();
        selection.move_to(3);
        selection.select_to(1);
        assert_eq!(selection.range, 1..3);
        assert!(selection.reversed);
        assert_eq!(selection.cursor(), 1);

        let text = "aяb";
        assert_eq!(previous_boundary(text, text.len()), 3);
        assert_eq!(next_boundary(text, 1), 3);
    }

    #[test]
    fn mode_chords_accept_only_single_ascii_letters_or_digits() {
        assert_eq!(mode_for_keystroke("g"), Some(Mode::Coordinates));
        assert_eq!(mode_for_keystroke("U"), Some(Mode::UnitConversion));
        assert_eq!(
            mode_for_keystroke("7"),
            Some(Mode::Worksheet(WorksheetKind::Mortgage))
        );
        assert_eq!(mode_for_keystroke("up"), None);
        assert_eq!(mode_for_keystroke("Up"), None);
        assert_eq!(mode_for_keystroke("g1"), None);
        assert_eq!(mode_for_keystroke("!"), None);
        assert_eq!(mode_for_keystroke("é"), None);
    }

    #[test]
    fn coordinate_help_is_localized_and_includes_the_complete_embedded_license() {
        let license = rustic_calc_app::coordinates::catalogue::LICENSE;
        for (lang, heading) in [(Lang::Ru, "WGS84 → МСК"), (Lang::En, "WGS84 → MSK")] {
            let help = coordinate_help_text(lang);
            assert!(help.starts_with(heading));
            assert!(help.contains(rustic_calc_app::coordinates::catalogue::SOURCE));
            assert!(help.contains(rustic_calc_app::coordinates::catalogue::REVISION));
            assert!(
                help.ends_with(license),
                "the scrollable help must contain the full license"
            );
        }
    }

    #[test]
    fn standard_and_scientific_sessions_remain_independent_across_switches() {
        let mut sessions = [ArithmeticSession::new(false), ArithmeticSession::new(true)];
        let mut memory = 0.0;

        let standard = arithmetic_index(Mode::Standard).unwrap();
        let scientific = arithmetic_index(Mode::Scientific).unwrap();
        sessions[standard].apply(Action::Digit('7'), Lang::En, &mut memory);
        sessions[scientific].apply(Action::Digit('3'), Lang::En, &mut memory);
        sessions[scientific].apply(Action::Inv, Lang::En, &mut memory);
        sessions[standard].apply(Action::Digit('8'), Lang::En, &mut memory);

        assert_eq!(sessions[standard].engine.current_value(), 78.0);
        assert_eq!(sessions[scientific].engine.current_value(), 3.0);
        assert!(!sessions[standard].inv);
        assert!(sessions[scientific].inv);
    }
}
