use std::{cell::RefCell, rc::Rc, time::Duration};

use rustic_calc_app::{
    actions::{self, Action, KeySpec, KeyStyle},
    coordinate_session::{CoordinateInputField, CoordinateSession, CoordinateTemplateSelection},
    coordinates::catalogue,
    date::{DateError, DateField, DateMode, DateOperation, DateOutcome, DateSession},
    engine::{self, AngleMode, Mode, StatsState, WorksheetKind},
    i18n::{self, Lang, Text},
    keypad::{
        action_for_char, action_for_key, apply_statistics, ArithmeticSession, KeyInput,
        ProgrammerSession,
    },
    startup::{mode_shortcut, mode_title, LaunchOptions, ThemeMode, MODES},
    unit_session::{ConversionResult, DcsLoopError, UnitField, UnitIndex, UnitSession},
    units::{self, Date},
    worksheet::WorksheetSession,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{AppWindow, FieldSpec, KeyCell, Palette};

const ID_DATE_MODE: i32 = 300;
const ID_FROM: i32 = 301;
const ID_TO: i32 = 302;
const ID_OPERATION: i32 = 303;
const ID_YEARS: i32 = 304;
const ID_MONTHS: i32 = 305;
const ID_DAYS: i32 = 306;
const ID_ANGLE: i32 = 320;
const ID_PROG_BASE: i32 = 321;
const ID_PROG_WORD: i32 = 322;
const ID_UNIT_CATEGORY: i32 = 330;
const ID_UNIT_FROM: i32 = 331;
const ID_UNIT_TO: i32 = 332;
const ID_UNIT_VALUE: i32 = 333;
const ID_LOOP_VOLTAGE: i32 = 334;
const ID_LOOP_CURRENT: i32 = 335;
const ID_WORKSHEET_TARGET: i32 = 350;
const ID_WORKSHEET_FIELD_BASE: i32 = 360;
const ID_COORD_REGION: i32 = 400;
const ID_COORD_TEMPLATE: i32 = 401;
const ID_COORD_DEFINITION: i32 = 402;
const ID_COORD_LATITUDE: i32 = 403;
const ID_COORD_LONGITUDE: i32 = 404;
const ID_COORD_HEIGHT: i32 = 405;
const ID_COORD_NORTHING: i32 = 406;
const ID_COORD_EASTING: i32 = 407;
const ID_LOOP_RESULT_BASE: i32 = 431;
const ID_DATE_RESULT_BASE: i32 = 440;
const ID_WORKSHEET_RESULT: i32 = 450;
const ID_COORD_SOURCE: i32 = 460;
const ID_CALCULATE: i32 = -100;
const ID_COORD_RESET: i32 = -101;
const ID_COORD_INFO: i32 = -102;
const ID_PRIMARY_KEY_BASE: i32 = 0;
const ID_SECONDARY_KEY_BASE: i32 = 1000;
const ID_BIT_KEY_BASE: i32 = 2000;

struct AppState {
    mode: Mode,
    lang: Lang,
    theme: ThemeMode,
    system_dark: bool,
    grouping: bool,
    show_history: bool,
    show_about: bool,
    show_coordinate_help: bool,
    memory: f64,
    standard: ArithmeticSession,
    scientific: ArithmeticSession,
    programmer: ProgrammerSession,
    statistics: StatsState,
    unit: UnitSession,
    date: DateSession,
    worksheets: [WorksheetSession; 4],
    coordinates: CoordinateSession,
    field_model: Rc<VecModel<FieldSpec>>,
    key_model: Rc<VecModel<KeyCell>>,
    secondary_key_model: Rc<VecModel<KeyCell>>,
    bit_model: Rc<VecModel<KeyCell>>,
}

impl AppState {
    fn new(options: &LaunchOptions) -> Self {
        Self::with_today(options, today())
    }

    fn with_today(options: &LaunchOptions, today: Date) -> Self {
        let kinds = [
            WorksheetKind::Mortgage,
            WorksheetKind::VehicleLease,
            WorksheetKind::FuelMpg,
            WorksheetKind::FuelL100,
        ];
        Self {
            mode: options.mode,
            lang: options.lang,
            theme: options.theme,
            system_dark: false,
            grouping: true,
            show_history: false,
            show_about: false,
            show_coordinate_help: false,
            memory: 0.0,
            standard: ArithmeticSession::new(false),
            scientific: ArithmeticSession::new(true),
            programmer: ProgrammerSession::new(),
            statistics: StatsState::new(),
            unit: UnitSession::new(),
            date: DateSession::new(today),
            worksheets: kinds.map(WorksheetSession::new),
            coordinates: CoordinateSession::new(),
            field_model: Rc::new(VecModel::default()),
            key_model: Rc::new(VecModel::default()),
            secondary_key_model: Rc::new(VecModel::default()),
            bit_model: Rc::new(VecModel::default()),
        }
    }

    fn text(&self) -> &'static Text {
        self.lang.text()
    }

    fn worksheet_index(kind: WorksheetKind) -> usize {
        match kind {
            WorksheetKind::Mortgage => 0,
            WorksheetKind::VehicleLease => 1,
            WorksheetKind::FuelMpg => 2,
            WorksheetKind::FuelL100 => 3,
        }
    }

    fn worksheet(&self) -> Option<&WorksheetSession> {
        let Mode::Worksheet(kind) = self.mode else {
            return None;
        };
        self.worksheets.get(Self::worksheet_index(kind))
    }

    fn worksheet_mut(&mut self) -> Option<&mut WorksheetSession> {
        let Mode::Worksheet(kind) = self.mode else {
            return None;
        };
        self.worksheets.get_mut(Self::worksheet_index(kind))
    }

    fn set_mode_index(&mut self, index: i32) {
        self.mode = MODES
            .get(index.max(0) as usize)
            .copied()
            .unwrap_or(Mode::Standard);
        self.show_coordinate_help = false;
    }

    fn set_field_text(&mut self, id: i32, value: String) {
        match self.mode {
            Mode::DateCalculation => {
                let field = match id {
                    ID_FROM => Some(DateField::From),
                    ID_TO => Some(DateField::To),
                    ID_YEARS => Some(DateField::Years),
                    ID_MONTHS => Some(DateField::Months),
                    ID_DAYS => Some(DateField::Days),
                    _ => None,
                };
                if let Some(field) = field {
                    self.date.set_text(field, value);
                }
            }
            Mode::UnitConversion => match id {
                ID_UNIT_VALUE => self.unit.set_text(UnitField::Value, value),
                ID_LOOP_VOLTAGE => self.unit.set_text(UnitField::LoopVoltage, value),
                ID_LOOP_CURRENT => self.unit.set_text(UnitField::LoopCurrentMa, value),
                _ => {}
            },
            Mode::Worksheet(_) => {
                if let Some(session) = self.worksheet_mut() {
                    let index = id - ID_WORKSHEET_FIELD_BASE;
                    if let Some(field) = session.fields().get(index.max(0) as usize).copied() {
                        session.set_text(field, value);
                    }
                }
            }
            Mode::Coordinates => match id {
                ID_COORD_DEFINITION => self.coordinates.set_definition(value),
                ID_COORD_LATITUDE => self
                    .coordinates
                    .set_text(CoordinateInputField::Latitude, value),
                ID_COORD_LONGITUDE => self
                    .coordinates
                    .set_text(CoordinateInputField::Longitude, value),
                ID_COORD_HEIGHT => self
                    .coordinates
                    .set_text(CoordinateInputField::Height, value),
                _ => {}
            },
            _ => {}
        }
    }

    fn set_selection(&mut self, id: i32, index: i32) {
        let index = index.max(0) as usize;
        match id {
            ID_DATE_MODE => self.date.set_mode(if index == 1 {
                DateMode::Shift
            } else {
                DateMode::Difference
            }),
            ID_OPERATION => self.date.set_operation(if index == 1 {
                DateOperation::Subtract
            } else {
                DateOperation::Add
            }),
            ID_ANGLE => {
                self.scientific.angle = match index {
                    1 => AngleMode::Radians,
                    2 => AngleMode::Gradians,
                    _ => AngleMode::Degrees,
                }
            }
            ID_PROG_BASE => {
                self.programmer.apply(
                    Action::ProgBase([16, 10, 8, 2].get(index).copied().unwrap_or(10)),
                    &mut self.memory,
                );
            }
            ID_PROG_WORD => {
                self.programmer.apply(
                    Action::ProgWord([64, 32, 16, 8].get(index).copied().unwrap_or(64)),
                    &mut self.memory,
                );
            }
            ID_UNIT_CATEGORY => {
                if let Some(category) = self.unit.categories().get(index) {
                    let _ = self.unit.select_category(category.id);
                }
            }
            ID_UNIT_FROM => {
                let _ = self.unit.select_from(UnitIndex::new(index));
            }
            ID_UNIT_TO => {
                let _ = self.unit.select_to(UnitIndex::new(index));
            }
            ID_WORKSHEET_TARGET => {
                if let Some(target) = self
                    .worksheet()
                    .and_then(|session| session.target_options().get(index).copied())
                {
                    if let Some(session) = self.worksheet_mut() {
                        session.set_target(target);
                    }
                }
            }
            ID_COORD_REGION => {
                if let Some(region) = CoordinateSession::regions().get(index) {
                    let _ = self.coordinates.select_region(region.id);
                }
            }
            ID_COORD_TEMPLATE => {
                if index == 0 {
                    self.coordinates.select_custom();
                } else if let Some(template) = self.coordinates.templates().get(index - 1) {
                    let _ = self.coordinates.select_template(template.id.clone());
                }
            }
            _ => {}
        }
    }

    fn calculate(&mut self) {
        match self.mode {
            Mode::DateCalculation => {
                self.date.calculate();
            }
            Mode::Worksheet(_) => {
                if let Some(session) = self.worksheet_mut() {
                    let _ = session.calculate();
                }
            }
            Mode::Coordinates => {
                let _ = self.coordinates.calculate();
            }
            _ => {}
        }
    }

    fn apply_action(&mut self, action: Action) {
        match self.mode {
            Mode::Standard => self.standard.apply(action, self.lang, &mut self.memory),
            Mode::Scientific => self.scientific.apply(action, self.lang, &mut self.memory),
            Mode::Programmer => self.programmer.apply(action, &mut self.memory),
            Mode::Statistics => {
                let current = self.statistics.current_value();
                apply_statistics(
                    action,
                    self.lang,
                    &mut self.statistics,
                    &mut self.memory,
                    current,
                );
            }
            _ => {}
        }
    }

    fn invoke_command(&mut self, command: i32) {
        match command {
            ID_CALCULATE => self.calculate(),
            ID_COORD_RESET if self.mode == Mode::Coordinates => {
                self.coordinates.reset_definition();
            }
            ID_COORD_INFO if self.mode == Mode::Coordinates => {
                self.show_coordinate_help = !self.show_coordinate_help;
            }
            _ if (ID_BIT_KEY_BASE..ID_BIT_KEY_BASE + 64).contains(&command)
                && self.mode == Mode::Programmer =>
            {
                self.programmer.apply(
                    Action::ProgBit((command - ID_BIT_KEY_BASE) as u32),
                    &mut self.memory,
                );
            }
            _ if command >= ID_SECONDARY_KEY_BASE => {
                let index = (command - ID_SECONDARY_KEY_BASE) as usize;
                if let Some(spec) = secondary_key_specs(self.mode, self.text()).get(index) {
                    self.apply_action(spec.action);
                }
            }
            _ if command >= ID_PRIMARY_KEY_BASE => {
                let index = (command - ID_PRIMARY_KEY_BASE) as usize;
                if let Some(spec) = primary_key_specs(self.mode, self.text()).get(index) {
                    self.apply_action(spec.action);
                }
            }
            _ => {}
        }
    }

    fn set_grouping(&mut self, grouping: bool) {
        self.grouping = grouping;
    }

    fn set_history_visible(&mut self, visible: bool) {
        self.show_history = visible;
    }

    fn fields(&self) -> Vec<FieldSpec> {
        let t = self.text();
        match self.mode {
            Mode::Standard => Vec::new(),
            Mode::Scientific => vec![field(
                ID_ANGLE,
                t.angle_degrees,
                "",
                1,
                vec![
                    t.angle_degrees.to_owned(),
                    t.angle_radians.to_owned(),
                    t.angle_grads.to_owned(),
                ],
                match self.scientific.angle {
                    AngleMode::Degrees => 0,
                    AngleMode::Radians => 1,
                    AngleMode::Gradians => 2,
                },
                -1,
            )],
            Mode::Programmer => vec![
                field(
                    ID_PROG_BASE,
                    t.base,
                    "",
                    1,
                    vec![
                        t.hex.to_owned(),
                        t.dec.to_owned(),
                        t.oct.to_owned(),
                        t.bin.to_owned(),
                    ],
                    [16, 10, 8, 2]
                        .iter()
                        .position(|base| *base == self.programmer.state.base)
                        .unwrap_or(1),
                    -1,
                ),
                field(
                    ID_PROG_WORD,
                    t.word_size,
                    "",
                    1,
                    vec![
                        t.qword.to_owned(),
                        t.dword.to_owned(),
                        t.word.to_owned(),
                        t.byte.to_owned(),
                    ],
                    [64, 32, 16, 8]
                        .iter()
                        .position(|word| *word == self.programmer.state.word)
                        .unwrap_or(0),
                    -1,
                ),
            ],
            Mode::Statistics => Vec::new(),
            Mode::UnitConversion => self.unit_fields(),
            Mode::DateCalculation => self.date_fields(),
            Mode::Worksheet(_) => self.worksheet_fields(),
            Mode::Coordinates => self.coordinate_fields(),
        }
    }

    fn unit_fields(&self) -> Vec<FieldSpec> {
        let t = self.text();
        let category = self.unit.category();
        let category_options = self
            .unit
            .categories()
            .iter()
            .map(|item| i18n::category(self.lang, item.name).to_owned())
            .collect();
        let unit_options = category
            .units
            .iter()
            .map(|item| i18n::unit(self.lang, item.name).to_owned())
            .collect::<Vec<_>>();
        let from_label = if self.lang == Lang::En {
            "From unit"
        } else {
            "Единица (из)"
        };
        let to_label = if self.lang == Lang::En {
            "To unit"
        } else {
            "Единица (в)"
        };
        let mut fields = vec![
            field(
                ID_UNIT_CATEGORY,
                t.unit_title,
                "",
                1,
                category_options,
                self.unit.category_index(),
                -1,
            ),
            field(
                ID_UNIT_VALUE,
                t.from,
                self.unit.text(UnitField::Value),
                0,
                Vec::new(),
                0,
                -1,
            ),
            field(
                ID_UNIT_FROM,
                from_label,
                "",
                1,
                unit_options.clone(),
                self.unit.from_index().get(),
                -1,
            ),
            field(
                ID_UNIT_TO,
                to_label,
                "",
                1,
                unit_options,
                self.unit.to_index().get(),
                -1,
            ),
        ];

        if let Some(result) = self.unit.dcs_loop_result() {
            let (resistance, power, standard, current, wattage) = match result {
                Ok(result) => (
                    units::format_resistance(result.resistance, self.grouping),
                    format!("{} W", engine::format_number(result.power, self.grouping)),
                    units::format_resistance(result.standard_resistance, self.grouping),
                    format!(
                        "{} mA",
                        engine::format_number(result.actual_current_ma, self.grouping)
                    ),
                    format!(
                        "{} W",
                        engine::format_number(result.recommended_wattage, false)
                    ),
                ),
                Err(_) => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };
            fields.extend([
                field(-2, t.dcs_loop_title, "", 2, Vec::new(), 0, -1),
                field(
                    ID_LOOP_VOLTAGE,
                    t.dcs_loop_voltage,
                    self.unit.text(UnitField::LoopVoltage),
                    0,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(
                    ID_LOOP_CURRENT,
                    t.dcs_loop_current,
                    self.unit.text(UnitField::LoopCurrentMa),
                    0,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(ID_LOOP_RESULT_BASE, "R", resistance, 2, Vec::new(), 0, -1),
                field(ID_LOOP_RESULT_BASE + 1, "P", power, 2, Vec::new(), 0, -1),
                field(
                    ID_LOOP_RESULT_BASE + 2,
                    "E24",
                    standard,
                    2,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(
                    ID_LOOP_RESULT_BASE + 3,
                    t.dcs_loop_current,
                    current,
                    2,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(ID_LOOP_RESULT_BASE + 4, "P", wattage, 2, Vec::new(), 0, -1),
            ]);
        }
        fields
    }

    fn date_fields(&self) -> Vec<FieldSpec> {
        let t = self.text();
        let mut fields = vec![
            field(
                ID_DATE_MODE,
                t.date_title,
                "",
                1,
                vec![t.date_diff_mode.to_owned(), t.date_shift_mode.to_owned()],
                usize::from(self.date.mode() == DateMode::Shift),
                -1,
            ),
            field(
                ID_FROM,
                t.from,
                self.date.text(DateField::From),
                0,
                Vec::new(),
                0,
                -1,
            ),
        ];
        match self.date.mode() {
            DateMode::Difference => fields.push(field(
                ID_TO,
                t.to,
                self.date.text(DateField::To),
                0,
                Vec::new(),
                0,
                -1,
            )),
            DateMode::Shift => fields.extend([
                field(
                    ID_OPERATION,
                    t.operation,
                    "",
                    1,
                    vec![t.add.to_owned(), t.subtract.to_owned()],
                    usize::from(self.date.operation() == DateOperation::Subtract),
                    -1,
                ),
                field(
                    ID_YEARS,
                    t.years,
                    self.date.text(DateField::Years),
                    0,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(
                    ID_MONTHS,
                    t.months,
                    self.date.text(DateField::Months),
                    0,
                    Vec::new(),
                    0,
                    -1,
                ),
                field(
                    ID_DAYS,
                    t.days,
                    self.date.text(DateField::Days),
                    0,
                    Vec::new(),
                    0,
                    -1,
                ),
            ]),
        }
        fields.push(field(0, t.calculate, "", 5, Vec::new(), 0, ID_CALCULATE));

        match self.date.mode() {
            DateMode::Difference => {
                let (parts, days) = match self.date.result() {
                    Some(Ok(DateOutcome::Difference {
                        years,
                        months,
                        weeks,
                        days,
                        total_days,
                    })) => (
                        i18n::date_parts(self.lang, years, months, weeks, days),
                        format!("{total_days} {}", t.days_suffix),
                    ),
                    _ => (String::new(), String::new()),
                };
                fields.extend([
                    field(
                        ID_DATE_RESULT_BASE,
                        t.diff_ymwd,
                        parts,
                        2,
                        Vec::new(),
                        0,
                        -1,
                    ),
                    field(
                        ID_DATE_RESULT_BASE + 1,
                        t.diff_days,
                        days,
                        2,
                        Vec::new(),
                        0,
                        -1,
                    ),
                ]);
            }
            DateMode::Shift => {
                let value = match self.date.result() {
                    Some(Ok(DateOutcome::Shifted(date))) => units::format_date(date),
                    _ => String::new(),
                };
                fields.push(field(
                    ID_DATE_RESULT_BASE,
                    t.result_date,
                    value,
                    2,
                    Vec::new(),
                    0,
                    -1,
                ));
            }
        }
        fields
    }

    fn worksheet_fields(&self) -> Vec<FieldSpec> {
        let Some(session) = self.worksheet() else {
            return Vec::new();
        };
        let t = self.text();
        let target_options = session
            .target_options()
            .iter()
            .map(|field| field.label(self.lang).to_owned())
            .collect::<Vec<_>>();
        let target_index = session
            .target_options()
            .iter()
            .position(|field| *field == session.target())
            .unwrap_or(0);
        let mut fields = vec![field(
            ID_WORKSHEET_TARGET,
            t.worksheet_title,
            "",
            1,
            target_options,
            target_index,
            -1,
        )];
        fields.extend(
            session
                .fields()
                .iter()
                .enumerate()
                .filter(|(_, field)| **field != session.target())
                .map(|(index, worksheet_field)| {
                    field(
                        ID_WORKSHEET_FIELD_BASE + index as i32,
                        worksheet_field.label(self.lang),
                        session.text(*worksheet_field).unwrap_or(""),
                        0,
                        Vec::new(),
                        0,
                        -1,
                    )
                }),
        );
        fields.push(field(0, t.calculate, "", 5, Vec::new(), 0, ID_CALCULATE));
        let result = session.formatted_result(self.grouping).unwrap_or_default();
        fields.push(field(
            ID_WORKSHEET_RESULT,
            session.target().label(self.lang),
            result,
            2,
            Vec::new(),
            0,
            -1,
        ));
        fields
    }

    fn coordinate_fields(&self) -> Vec<FieldSpec> {
        let text = i18n::coordinates::text(self.lang);
        if self.show_coordinate_help {
            let help = format!(
                "{}\n\n{}: {}\n{}: {}\n\n{}",
                text.help,
                text.source,
                catalogue::SOURCE,
                text.revision,
                catalogue::REVISION,
                catalogue::LICENSE
            );
            return vec![
                field(-1, text.title, help, 4, Vec::new(), 0, -1),
                field(0, text.back, "", 5, Vec::new(), 0, ID_COORD_INFO),
            ];
        }

        let regions = CoordinateSession::regions()
            .iter()
            .map(|region| region.name(self.lang).to_owned())
            .collect::<Vec<_>>();
        let mut templates = vec![text.custom.to_owned()];
        templates.extend(
            self.coordinates
                .templates()
                .iter()
                .map(|template| i18n::coordinates::template_name(self.lang, template.name)),
        );
        let region_index = CoordinateSession::regions()
            .iter()
            .position(|region| region.id == self.coordinates.region().as_str())
            .unwrap_or(0);
        let template_index = match self.coordinates.selection() {
            CoordinateTemplateSelection::Custom => 0,
            CoordinateTemplateSelection::Template(id) => self
                .coordinates
                .templates()
                .iter()
                .position(|template| template.id == *id)
                .map(|index| index + 1)
                .unwrap_or(0),
        };
        let output = self
            .coordinates
            .result()
            .and_then(|result| result.as_ref().ok());
        let northing = output
            .map(|point| engine::format_number(point.northing_m, self.grouping))
            .unwrap_or_default();
        let easting = output
            .map(|point| engine::format_number(point.easting_m, self.grouping))
            .unwrap_or_default();
        let note = if let Some(template) = self.coordinates.selected_template() {
            let status = if self.coordinates.definition().trim() == template.definition {
                text.original
            } else {
                text.modified
            };
            format!("{}: {} · {status}", text.source, template.id.as_str())
        } else if self.coordinates.templates().is_empty()
            && self.coordinates.definition().is_empty()
        {
            text.no_templates.to_owned()
        } else {
            text.custom_status.to_owned()
        };

        vec![
            field(
                ID_COORD_REGION,
                text.region,
                "",
                1,
                regions,
                region_index,
                -1,
            ),
            field(
                ID_COORD_TEMPLATE,
                text.template,
                "",
                1,
                templates,
                template_index,
                -1,
            ),
            field(
                ID_COORD_DEFINITION,
                text.definition,
                self.coordinates.definition(),
                3,
                Vec::new(),
                0,
                -1,
            ),
            field(
                ID_COORD_LATITUDE,
                text.latitude,
                self.coordinates.text(CoordinateInputField::Latitude),
                0,
                Vec::new(),
                0,
                -1,
            ),
            field(
                ID_COORD_LONGITUDE,
                text.longitude,
                self.coordinates.text(CoordinateInputField::Longitude),
                0,
                Vec::new(),
                0,
                -1,
            ),
            field(
                ID_COORD_HEIGHT,
                text.height,
                self.coordinates.text(CoordinateInputField::Height),
                0,
                Vec::new(),
                0,
                -1,
            ),
            field(0, self.text().calculate, "", 5, Vec::new(), 0, ID_CALCULATE),
            field(1, text.reset, "", 5, Vec::new(), 0, ID_COORD_RESET),
            field(
                ID_COORD_NORTHING,
                text.northing,
                northing,
                2,
                Vec::new(),
                0,
                -1,
            ),
            field(
                ID_COORD_EASTING,
                text.easting,
                easting,
                2,
                Vec::new(),
                0,
                -1,
            ),
            field(ID_COORD_SOURCE, text.accuracy, note, 2, Vec::new(), 0, -1),
            field(2, text.info, "", 5, Vec::new(), 0, ID_COORD_INFO),
        ]
    }

    fn result_text(&self) -> String {
        match self.mode {
            Mode::Standard => self.standard.display(self.lang, self.grouping),
            Mode::Scientific => self.scientific.display(self.lang, self.grouping),
            Mode::Programmer => self.programmer.display(self.lang),
            Mode::Statistics => i18n::runtime_error(self.lang, &self.statistics.display()),
            Mode::UnitConversion => self.unit_result(),
            Mode::DateCalculation => match self.date.result() {
                Some(Ok(DateOutcome::Difference { total_days, .. })) => {
                    format!("{total_days} {}", self.text().days_suffix)
                }
                Some(Ok(DateOutcome::Shifted(date))) => units::format_date(date),
                _ => String::new(),
            },
            Mode::Worksheet(_) => self
                .worksheet()
                .and_then(|session| session.formatted_result(self.grouping))
                .unwrap_or_default(),
            Mode::Coordinates => self
                .coordinates
                .result()
                .and_then(|result| result.as_ref().ok())
                .map(|point| {
                    format!(
                        "X {}   Y {}",
                        engine::format_number(point.northing_m, self.grouping),
                        engine::format_number(point.easting_m, self.grouping)
                    )
                })
                .unwrap_or_default(),
        }
    }

    fn unit_result(&self) -> String {
        match self.unit.conversion_result() {
            ConversionResult::Value(value) => {
                i18n::runtime_error(self.lang, &engine::format_number(value, self.grouping))
            }
            ConversionResult::Empty | ConversionResult::InvalidInput => String::new(),
        }
    }

    fn expression(&self) -> String {
        match self.mode {
            Mode::Standard => self.standard.expression(self.grouping),
            Mode::Scientific => self.scientific.expression(self.grouping),
            Mode::Statistics => format!("{}: {}", self.text().count, self.statistics.count()),
            Mode::UnitConversion => {
                let category = self.unit.category();
                let from = category.units.get(self.unit.from_index().get());
                let to = category.units.get(self.unit.to_index().get());
                match (from, to) {
                    (Some(from), Some(to)) => format!(
                        "{} → {}",
                        i18n::unit(self.lang, from.name),
                        i18n::unit(self.lang, to.name)
                    ),
                    _ => String::new(),
                }
            }
            Mode::Coordinates => self
                .coordinates
                .selected_template()
                .map(|template| i18n::coordinates::template_name(self.lang, template.name))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn status(&self) -> (String, bool) {
        let t = self.text();
        match self.mode {
            Mode::Standard | Mode::Scientific => {
                let engine = if self.mode == Mode::Standard {
                    &self.standard.engine
                } else {
                    &self.scientific.engine
                };
                engine
                    .error
                    .as_deref()
                    .map(|error| (i18n::runtime_error(self.lang, error), true))
                    .unwrap_or_default()
            }
            Mode::Programmer => self
                .programmer
                .state
                .error
                .as_deref()
                .map(|error| (i18n::runtime_error(self.lang, error), true))
                .unwrap_or_default(),
            Mode::Statistics => self
                .statistics
                .error
                .as_deref()
                .map(|error| (i18n::runtime_error(self.lang, error), true))
                .unwrap_or_default(),
            Mode::UnitConversion => {
                if matches!(
                    self.unit.conversion_result(),
                    ConversionResult::InvalidInput
                ) {
                    (t.field_number.to_owned(), true)
                } else if let Some(Err(error)) = self.unit.dcs_loop_result() {
                    let text = match error {
                        DcsLoopError::InvalidVoltage => {
                            format!("{}: {}", t.dcs_loop_voltage, t.field_number)
                        }
                        DcsLoopError::InvalidCurrent => {
                            format!("{}: {}", t.dcs_loop_current, t.field_number)
                        }
                        DcsLoopError::NonPositiveVoltage | DcsLoopError::NonPositiveCurrent => {
                            t.loop_positive_values.to_owned()
                        }
                        DcsLoopError::OutOfRange => t.loop_positive_values.to_owned(),
                    };
                    (text, true)
                } else {
                    (String::new(), false)
                }
            }
            Mode::DateCalculation => match self.date.result() {
                Some(Err(error)) => {
                    let (field, message) = match error {
                        DateError::InvalidDate(field) => (Some(field), t.invalid_date),
                        DateError::InvalidOffset(field) => {
                            (Some(field), t.field_nonnegative_integer)
                        }
                        DateError::OutOfBounds => return (t.date_out_of_bound.to_owned(), true),
                    };
                    let label = match field {
                        Some(DateField::From) => t.from,
                        Some(DateField::To) => t.to,
                        Some(DateField::Years) => t.years,
                        Some(DateField::Months) => t.months,
                        Some(DateField::Days) => t.days,
                        None => "",
                    };
                    (format!("{label}: {message}"), true)
                }
                _ => (String::new(), false),
            },
            Mode::Worksheet(_) => self
                .worksheet()
                .and_then(|session| session.display_error(self.lang))
                .map(|error| (error, true))
                .unwrap_or_default(),
            Mode::Coordinates => self
                .coordinates
                .result()
                .and_then(|result| result.as_ref().err())
                .map(|error| (i18n::coordinates::error(self.lang, error), true))
                .unwrap_or_default(),
        }
    }

    fn history_text(&self) -> String {
        let items: &[String] = match self.mode {
            Mode::Standard => &self.standard.engine.history,
            Mode::Scientific => &self.scientific.engine.history,
            Mode::Programmer => &self.programmer.state.history,
            Mode::Statistics => &self.statistics.history,
            _ => &[],
        };
        items.join("\n")
    }

    fn refresh(&mut self, ui: &AppWindow) {
        let t = self.text();
        let fields = self.fields();
        self.field_model.set_vec(fields);
        let (primary, secondary) = key_models(self.mode, t, self.programmer.state.base);
        self.key_model.set_vec(primary);
        self.secondary_key_model.set_vec(secondary);
        self.bit_model.set_vec(if self.mode == Mode::Programmer {
            bit_cells(&self.programmer)
        } else {
            Vec::new()
        });

        ui.set_mode_options(string_model(
            MODES
                .iter()
                .copied()
                .map(|mode| mode_title(mode, self.lang).to_owned()),
        ));
        ui.set_mode_index(
            MODES
                .iter()
                .position(|mode| *mode == self.mode)
                .unwrap_or(0) as i32,
        );
        ui.set_language_options(string_model(vec![
            t.lang_russian.to_owned(),
            t.lang_english.to_owned(),
        ]));
        ui.set_language_index(if self.lang == Lang::En { 1 } else { 0 });
        ui.set_theme_options(string_model(vec![
            t.theme_system.to_owned(),
            t.theme_white.to_owned(),
            t.theme_dark.to_owned(),
        ]));
        ui.set_theme_index(theme_index(self.theme));
        ui.set_section_title(SharedString::from(mode_title(self.mode, self.lang)));
        ui.set_expression(SharedString::from(self.expression()));
        ui.set_display(SharedString::from(self.result_text()));
        let (status, is_error) = self.status();
        ui.set_status_text(SharedString::from(status));
        ui.set_status_error(is_error);
        ui.set_fields(ModelRc::new(self.field_model.clone()));
        ui.set_keys(ModelRc::new(self.key_model.clone()));
        ui.set_secondary_keys(ModelRc::new(self.secondary_key_model.clone()));
        ui.set_bit_keys(ModelRc::new(self.bit_model.clone()));
        ui.set_grouping(self.grouping);
        ui.set_show_history(self.show_history);
        ui.set_history_text(SharedString::from(self.history_text()));
        ui.set_show_about(self.show_about);
        ui.set_about_button_label(SharedString::from(t.menu_about));
        ui.set_about_title(SharedString::from(t.about_title));
        ui.set_about_app_text(SharedString::from(t.about_text));
        ui.set_close_label(SharedString::from(if self.lang == Lang::Ru {
            "Закрыть"
        } else {
            "Close"
        }));
        ui.set_grouping_label(SharedString::from(t.menu_grouping));
        ui.set_history_label(SharedString::from(t.menu_history));

        let dark = self.theme.is_dark(self.system_dark);
        ui.global::<Palette>().set_color_scheme(match self.theme {
            ThemeMode::System => slint::language::ColorScheme::Unknown,
            ThemeMode::White => slint::language::ColorScheme::Light,
            ThemeMode::Dark => slint::language::ColorScheme::Dark,
        });
        let colors = UiColors::for_theme(dark);
        ui.set_window_background(colors.window);
        ui.set_panel_background(colors.panel);
        ui.set_panel_border(colors.border);
        ui.set_foreground(colors.foreground);
        ui.set_muted(colors.muted);
        ui.set_accent(colors.accent);
        ui.set_key_background(colors.key);
        ui.set_operator_background(colors.operator);
        ui.set_memory_background(colors.memory);
        ui.set_equals_background(colors.equals);
        ui.set_key_border(colors.key_border);
        ui.set_disabled_background(colors.disabled);
        ui.set_disabled_text(colors.disabled_text);
        ui.set_bit_on(colors.bit_on);
        ui.set_bit_off(colors.bit_off);
    }

    /// Update result rows in place after edits so LineEdit/TextEdit models are
    /// not rebuilt on every keystroke and retain their caret and selection.
    fn refresh_derived(&self, ui: &AppWindow) {
        let fresh = self.fields();
        for row in 0..self.field_model.row_count() {
            let Some(mut current) = self.field_model.row_data(row) else {
                continue;
            };
            if current.kind != 2 {
                continue;
            }
            if let Some(updated) = fresh
                .iter()
                .find(|field| field.id == current.id && field.kind == current.kind)
            {
                current.value = updated.value.clone();
                self.field_model.set_row_data(row, current);
            }
        }
        ui.set_display(SharedString::from(self.result_text()));
        ui.set_expression(SharedString::from(self.expression()));
        let (status, is_error) = self.status();
        ui.set_status_text(SharedString::from(status));
        ui.set_status_error(is_error);
        ui.set_history_text(SharedString::from(self.history_text()));
    }
}

pub fn run(options: LaunchOptions) -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let state = Rc::new(RefCell::new(AppState::new(&options)));
    {
        let mut state = state.borrow_mut();
        state.system_dark =
            ui.global::<Palette>().get_color_scheme() == slint::language::ColorScheme::Dark;
        state.refresh(&ui);
    }

    let weak = ui.as_weak();
    let state_for_mode = state.clone();
    ui.on_request_mode(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_mode.borrow_mut();
            state.set_mode_index(index);
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_lang = state.clone();
    ui.on_request_language(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_lang.borrow_mut();
            state.lang = if index == 1 { Lang::En } else { Lang::Ru };
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_theme = state.clone();
    ui.on_request_theme(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_theme.borrow_mut();
            state.theme = match index {
                1 => ThemeMode::White,
                2 => ThemeMode::Dark,
                _ => ThemeMode::System,
            };
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_text = state.clone();
    ui.on_request_field_text(move |id, value| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let state_rc = state_for_text.clone();
        let mut state = state_rc.borrow_mut();
        state.set_field_text(id, value.to_string());
        state.refresh_derived(&ui);
    });

    let weak = ui.as_weak();
    let state_for_selection = state.clone();
    ui.on_request_field_selection(move |id, index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_selection.borrow_mut();
            state.set_selection(id, index);
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_action = state.clone();
    ui.on_request_action(move |command| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_action.borrow_mut();
            state.invoke_command(command);
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_about = state.clone();
    ui.on_request_about(move |show| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_about.borrow_mut();
            state.show_about = show;
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_grouping = state.clone();
    ui.on_request_grouping(move |checked| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_grouping.borrow_mut();
            state.set_grouping(checked);
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_history = state.clone();
    ui.on_request_history(move |checked| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_history.borrow_mut();
            state.set_history_visible(checked);
            state.refresh(&ui);
        }
    });

    let weak = ui.as_weak();
    let state_for_keyboard = state.clone();
    ui.on_request_keyboard(move |key, control| {
        let Some(ui) = weak.upgrade() else {
            return false;
        };
        let mut state = state_for_keyboard.borrow_mut();
        if control {
            if let Some(key) = key.as_bytes().first().copied() {
                if let Some(mode) = mode_shortcut(key) {
                    state.mode = mode;
                    state.show_coordinate_help = false;
                    state.refresh(&ui);
                    return true;
                }
            }
            return false;
        }

        if key.as_str() == "@f2" && state.mode == Mode::Coordinates {
            state.show_coordinate_help = !state.show_coordinate_help;
            state.refresh(&ui);
            return true;
        }
        let key_input = match key.as_str() {
            "@enter" => Some(KeyInput::Enter),
            "@backspace" => Some(KeyInput::Backspace),
            "@escape" => Some(KeyInput::Escape),
            "@delete" => Some(KeyInput::Delete),
            _ => None,
        };
        if key.as_str() == "@escape"
            && state.mode == Mode::Coordinates
            && state.show_coordinate_help
        {
            state.show_coordinate_help = false;
            state.refresh(&ui);
            return true;
        }
        if matches!(key.as_str(), "@enter")
            && matches!(
                state.mode,
                Mode::DateCalculation | Mode::Worksheet(_) | Mode::Coordinates
            )
        {
            state.calculate();
            state.refresh(&ui);
            return true;
        }
        let action = key_input
            .and_then(|key| action_for_key(state.mode, key))
            .or_else(|| {
                let mut chars = key.chars();
                let ch = chars.next()?;
                chars
                    .next()
                    .is_none()
                    .then(|| action_for_char(state.mode, ch))
                    .flatten()
            });
        if let Some(action) = action {
            state.apply_action(action);
            state.refresh(&ui);
            true
        } else {
            false
        }
    });

    let screenshot_result = Rc::new(RefCell::new(None));
    let screenshot_requested = options.screenshot.is_some();
    if let Some(path) = options.screenshot {
        let weak = ui.as_weak();
        let capture = screenshot_result.clone();
        slint::Timer::single_shot(Duration::from_millis(700), move || {
            let result = weak
                .upgrade()
                .ok_or_else(|| "Slint window closed before screenshot".to_owned())
                .and_then(|ui| {
                    let pixels = ui
                        .window()
                        .take_snapshot()
                        .map_err(|error| error.to_string())?;
                    let mut bgra = Vec::with_capacity(pixels.as_bytes().len());
                    for pixel in pixels.as_slice() {
                        bgra.extend_from_slice(&[pixel.b, pixel.g, pixel.r, pixel.a]);
                    }
                    crate::png::write_bgra(&path, pixels.width(), pixels.height(), &bgra)
                        .map_err(|error| error.to_string())
                });
            *capture.borrow_mut() = Some(result);
            let _ = slint::quit_event_loop();
        });
    }

    ui.run()?;
    if screenshot_requested {
        match screenshot_result.borrow_mut().take() {
            Some(result) => result?,
            None => return Err("Slint event loop closed before the screenshot was captured".into()),
        }
    }
    Ok(())
}

fn field(
    id: i32,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    kind: i32,
    options: Vec<String>,
    selected: usize,
    command: i32,
) -> FieldSpec {
    FieldSpec {
        id,
        label: label.into(),
        value: value.into(),
        kind,
        options: string_model(options),
        selected: selected as i32,
        command,
    }
}

fn string_model(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn primary_key_specs(mode: Mode, text: &Text) -> Vec<KeySpec> {
    match mode {
        Mode::Standard => actions::standard_keys(),
        Mode::Scientific => actions::scientific_keys(),
        Mode::Programmer => actions::programmer_keys(),
        Mode::Statistics => actions::stats_keys(text.stats_add),
        _ => Vec::new(),
    }
}

fn secondary_key_specs(mode: Mode, _text: &Text) -> Vec<KeySpec> {
    if mode == Mode::Scientific {
        actions::standard_keys()
    } else {
        Vec::new()
    }
}

fn key_models(mode: Mode, text: &Text, base: u8) -> (Vec<KeyCell>, Vec<KeyCell>) {
    let cols = match mode {
        Mode::Programmer => 8,
        _ => 5,
    };
    let mut primary_specs = primary_key_specs(mode, text);
    if mode == Mode::Programmer {
        for spec in &mut primary_specs {
            if let Action::ProgDigit(ch) = spec.action {
                spec.enabled = ch.to_digit(16).is_some_and(|digit| digit < base as u32);
            }
        }
    }
    let secondary_specs = secondary_key_specs(mode, text);
    (
        key_cells(&primary_specs, cols, ID_PRIMARY_KEY_BASE),
        key_cells(&secondary_specs, 5, ID_SECONDARY_KEY_BASE),
    )
}

fn key_cells(specs: &[KeySpec], columns: i32, command_base: i32) -> Vec<KeyCell> {
    let columns = columns.max(1) as usize;
    let mut occupied: Vec<bool> = Vec::new();
    let mut cursor = 0usize;
    let mut cells = Vec::with_capacity(specs.len());
    for spec in specs {
        let span_x = spec.span_x.max(1).min(columns);
        let span_y = spec.span_y.max(1);
        let position = loop {
            let col = cursor % columns;
            if col + span_x > columns {
                cursor += columns - col;
                continue;
            }
            let row = cursor / columns;
            let fits = (0..span_y).all(|dy| {
                (0..span_x).all(|dx| {
                    let index = (row + dy) * columns + col + dx;
                    !occupied.get(index).copied().unwrap_or(false)
                })
            });
            if fits {
                break cursor;
            }
            cursor += 1;
        };
        let row = position / columns;
        let col = position % columns;
        for dy in 0..span_y {
            for dx in 0..span_x {
                let index = (row + dy) * columns + col + dx;
                if index >= occupied.len() {
                    occupied.resize(index + 1, false);
                }
                occupied[index] = true;
            }
        }
        let style = match spec.style {
            KeyStyle::Normal => 0,
            KeyStyle::Operator => 1,
            KeyStyle::Equals => 2,
            KeyStyle::Memory => 3,
            KeyStyle::Blank => 4,
        };
        cells.push(KeyCell {
            label: SharedString::from(spec.label),
            command: command_base + cells.len() as i32,
            row: row as i32,
            col: col as i32,
            row_span: span_y as i32,
            col_span: span_x as i32,
            enabled: spec.enabled,
            style,
        });
        cursor = position + span_x;
    }
    cells
}

fn bit_cells(programmer: &ProgrammerSession) -> Vec<KeyCell> {
    let state = &programmer.state;
    (0..64)
        .map(|slot| {
            let row = slot / 32;
            let col = slot % 32;
            let bit = if row == 0 { 63 - col } else { 31 - col };
            let enabled = bit < state.word as usize;
            let set = enabled && state.value & (1u64 << bit) != 0;
            KeyCell {
                label: SharedString::from(if set { "1" } else { "0" }),
                command: ID_BIT_KEY_BASE + bit as i32,
                row: row as i32,
                col: col as i32,
                row_span: 1,
                col_span: 1,
                enabled,
                style: i32::from(set),
            }
        })
        .collect()
}

fn theme_index(theme: ThemeMode) -> i32 {
    match theme {
        ThemeMode::System => 0,
        ThemeMode::White => 1,
        ThemeMode::Dark => 2,
    }
}

fn today() -> Date {
    crate::date::local_date()
}

struct UiColors {
    window: slint::Color,
    panel: slint::Color,
    border: slint::Color,
    foreground: slint::Color,
    muted: slint::Color,
    accent: slint::Color,
    key: slint::Color,
    operator: slint::Color,
    memory: slint::Color,
    equals: slint::Color,
    key_border: slint::Color,
    disabled: slint::Color,
    disabled_text: slint::Color,
    bit_on: slint::Color,
    bit_off: slint::Color,
}

impl UiColors {
    fn for_theme(dark: bool) -> Self {
        let values = if dark {
            [
                0x1d2430, 0x252e3b, 0x3a4859, 0xf1f4f8, 0xa8b5c5, 0xe2a84b, 0x303b49, 0x34485a,
                0x2b3948, 0x9a6a22, 0x4d5c6e, 0x273240, 0x778597, 0x4b8bc3, 0xa8b5c5,
            ]
        } else {
            [
                0xeaf1f8, 0xf8fbfe, 0xb8c8d8, 0x1e2832, 0x5a6978, 0xd99a36, 0xf7fafd, 0xe8f1f9,
                0xedf3f8, 0xf5b84c, 0xbecddc, 0xe0e8f1, 0x919dab, 0x1c568c, 0x788c9e,
            ]
        };
        Self {
            window: color(values[0]),
            panel: color(values[1]),
            border: color(values[2]),
            foreground: color(values[3]),
            muted: color(values[4]),
            accent: color(values[5]),
            key: color(values[6]),
            operator: color(values[7]),
            memory: color(values[8]),
            equals: color(values[9]),
            key_border: color(values[10]),
            disabled: color(values[11]),
            disabled_text: color(values[12]),
            bit_on: color(values[13]),
            bit_off: color(values[14]),
        }
    }
}

fn color(rgb: u32) -> slint::Color {
    slint::Color::from_rgb_u8(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustic_calc_app::units::CategoryId;

    fn state() -> AppState {
        let options = LaunchOptions {
            mode: Mode::Standard,
            lang: Lang::En,
            theme: ThemeMode::White,
            screenshot: None,
        };
        AppState::with_today(&options, Date::new(2024, 2, 29).unwrap())
    }

    #[test]
    fn all_eleven_modes_have_live_frontend_content() {
        let mut state = state();
        for (index, mode) in MODES.iter().copied().enumerate() {
            state.set_mode_index(index as i32);
            assert_eq!(state.mode, mode);
            let has_controls =
                !state.fields().is_empty() || !primary_key_specs(mode, state.text()).is_empty();
            assert!(has_controls, "missing Slint controls for {mode:?}");
            assert!(!state.status().0.contains("not connected"));
        }
    }

    #[test]
    fn arithmetic_and_programmer_key_models_dispatch_to_shared_sessions() {
        fn click(state: &mut AppState, specs: &[KeySpec], base: i32, label: &str) {
            let index = specs
                .iter()
                .position(|spec| spec.label == label)
                .unwrap_or_else(|| panic!("missing keypad key {label}"));
            state.invoke_command(base + index as i32);
        }

        let mut state = state();
        let standard = actions::standard_keys();
        for label in ["7", "+", "3", "="] {
            click(&mut state, &standard, ID_PRIMARY_KEY_BASE, label);
        }
        assert_eq!(state.result_text(), "10");

        state.mode = Mode::Programmer;
        state.set_selection(ID_PROG_BASE, 0);
        let programmer = actions::programmer_keys();
        click(&mut state, &programmer, ID_PRIMARY_KEY_BASE, "F");
        assert_eq!(state.result_text(), "F");
        assert_eq!(bit_cells(&state.programmer).len(), 64);

        state.mode = Mode::Scientific;
        let standard = actions::standard_keys();
        for label in ["7", "+", "3", "="] {
            click(&mut state, &standard, ID_SECONDARY_KEY_BASE, label);
        }
        assert_eq!(state.result_text(), "10");

        state.mode = Mode::Statistics;
        let stats_add_label = state.text().stats_add;
        let stats = actions::stats_keys(stats_add_label);
        click(&mut state, &stats, ID_PRIMARY_KEY_BASE, "5");
        click(&mut state, &stats, ID_PRIMARY_KEY_BASE, &stats_add_label);
        assert_eq!(state.statistics.data, vec![5.0]);
    }

    #[test]
    fn shared_date_worksheet_unit_and_coordinate_sessions_are_wired() {
        let mut state = state();
        state.mode = Mode::DateCalculation;
        state.set_field_text(ID_TO, "07.03.2024".into());
        state.calculate();
        assert_eq!(state.result_text(), "7 days");

        state.mode = Mode::Worksheet(WorksheetKind::FuelMpg);
        state.set_selection(ID_WORKSHEET_TARGET, 2);
        state.set_field_text(ID_WORKSHEET_FIELD_BASE, "300".into());
        state.set_field_text(ID_WORKSHEET_FIELD_BASE + 1, "10".into());
        state.calculate();
        assert_eq!(state.result_text(), "30");

        state.mode = Mode::UnitConversion;
        state.set_selection(
            ID_UNIT_CATEGORY,
            state
                .unit
                .categories()
                .iter()
                .position(|category| category.id == CategoryId::Temperature)
                .unwrap() as i32,
        );
        assert!(!state
            .fields()
            .iter()
            .any(|field| field.kind == 2 && field.label == "В"));
        state.set_selection(ID_UNIT_FROM, 1);
        state.set_selection(ID_UNIT_TO, 0);
        state.set_field_text(ID_UNIT_VALUE, "32".into());
        assert_eq!(state.unit_result(), "0");

        state.mode = Mode::Coordinates;
        state.set_field_text(ID_COORD_LATITUDE, "55.7558".into());
        state.set_field_text(ID_COORD_LONGITUDE, "37.6173".into());
        state.set_field_text(ID_COORD_HEIGHT, "150".into());
        state.calculate();
        assert!(state.coordinates.result().is_some_and(Result::is_ok));
    }

    #[test]
    fn date_mode_and_language_updates_keep_shared_drafts() {
        let mut state = state();
        state.mode = Mode::DateCalculation;
        state.set_field_text(ID_TO, "15.03.2024".into());
        state.set_selection(ID_DATE_MODE, 1);
        state.set_selection(ID_DATE_MODE, 0);
        assert_eq!(state.date.text(DateField::To), "15.03.2024");
        state.lang = Lang::Ru;
        assert!(state.fields().iter().any(|field| field.label == "В"));
    }
}
