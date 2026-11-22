//! Shared raw drafts and calculation state for the four worksheet modes.
//!
//! Frontends own their editors and layout; this session owns field text,
//! target selection, calculation results, and typed errors.

use crate::engine::{self, WorksheetKind};
use crate::i18n::{self, Lang};
use crate::units;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorksheetField {
    PurchasePrice,
    DownPayment,
    TermYears,
    InterestRate,
    MonthlyPayment,
    LeaseValue,
    LeasePeriod,
    PaymentsPerYear,
    ResidualValue,
    PeriodicPayment,
    DistanceMiles,
    FuelGallons,
    FuelEconomyMpg,
    DistanceKilometers,
    FuelLiters,
    FuelEconomyL100Km,
}

impl WorksheetField {
    /// Stable English key used by the existing RU/EN worksheet labels.
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::PurchasePrice => "Purchase price",
            Self::DownPayment => "Down payment",
            Self::TermYears => "Term (years)",
            Self::InterestRate => "Interest rate (%)",
            Self::MonthlyPayment => "Monthly payment",
            Self::LeaseValue => "Lease value",
            Self::LeasePeriod => "Lease period",
            Self::PaymentsPerYear => "Payments per year",
            Self::ResidualValue => "Residual value",
            Self::PeriodicPayment => "Periodic payment",
            Self::DistanceMiles => "Distance (miles)",
            Self::FuelGallons => "Fuel used (gallons)",
            Self::FuelEconomyMpg => "Fuel economy (mpg)",
            Self::DistanceKilometers => "Distance (kilometers)",
            Self::FuelLiters => "Fuel used (liters)",
            Self::FuelEconomyL100Km => "Fuel economy (L/100 km)",
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        i18n::worksheet_arg(lang, self.label_key())
    }
}

const MORTGAGE_FIELDS: &[WorksheetField] = &[
    WorksheetField::PurchasePrice,
    WorksheetField::DownPayment,
    WorksheetField::TermYears,
    WorksheetField::InterestRate,
    WorksheetField::MonthlyPayment,
];
const MORTGAGE_TARGETS: &[WorksheetField] = &[
    WorksheetField::PurchasePrice,
    WorksheetField::DownPayment,
    WorksheetField::TermYears,
    WorksheetField::MonthlyPayment,
];
const LEASE_FIELDS: &[WorksheetField] = &[
    WorksheetField::LeaseValue,
    WorksheetField::LeasePeriod,
    WorksheetField::PaymentsPerYear,
    WorksheetField::ResidualValue,
    WorksheetField::InterestRate,
    WorksheetField::PeriodicPayment,
];
const LEASE_TARGETS: &[WorksheetField] = &[
    WorksheetField::LeaseValue,
    WorksheetField::LeasePeriod,
    WorksheetField::ResidualValue,
    WorksheetField::PeriodicPayment,
];
const MPG_FIELDS: &[WorksheetField] = &[
    WorksheetField::DistanceMiles,
    WorksheetField::FuelGallons,
    WorksheetField::FuelEconomyMpg,
];
const L100_FIELDS: &[WorksheetField] = &[
    WorksheetField::DistanceKilometers,
    WorksheetField::FuelLiters,
    WorksheetField::FuelEconomyL100Km,
];

fn fields_for(kind: WorksheetKind) -> &'static [WorksheetField] {
    match kind {
        WorksheetKind::Mortgage => MORTGAGE_FIELDS,
        WorksheetKind::VehicleLease => LEASE_FIELDS,
        WorksheetKind::FuelMpg => MPG_FIELDS,
        WorksheetKind::FuelL100 => L100_FIELDS,
    }
}

fn targets_for(kind: WorksheetKind) -> &'static [WorksheetField] {
    match kind {
        WorksheetKind::Mortgage => MORTGAGE_TARGETS,
        WorksheetKind::VehicleLease => LEASE_TARGETS,
        WorksheetKind::FuelMpg => MPG_FIELDS,
        WorksheetKind::FuelL100 => L100_FIELDS,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorksheetInputError {
    Required,
    InvalidNumber,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorksheetCalculationError {
    ValueCannotBeBlank,
    InvalidValue,
    UnableToCalculate,
    Other(String),
}

impl WorksheetCalculationError {
    fn from_solver_message(message: String) -> Self {
        match message.as_str() {
            "This value cannot be blank." => Self::ValueCannotBeBlank,
            "This value is not valid." => Self::InvalidValue,
            "Unable to calculate" => Self::UnableToCalculate,
            _ => Self::Other(message),
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::ValueCannotBeBlank => "This value cannot be blank.",
            Self::InvalidValue => "This value is not valid.",
            Self::UnableToCalculate => "Unable to calculate",
            Self::Other(message) => message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorksheetError {
    InvalidInput {
        field: WorksheetField,
        reason: WorksheetInputError,
    },
    Calculation(WorksheetCalculationError),
}

impl WorksheetError {
    /// Localizes the typed error using the calculator's shared RU/EN strings.
    pub fn display(&self, lang: Lang) -> String {
        match self {
            Self::InvalidInput { field, reason } => {
                let text = lang.text();
                let reason = match reason {
                    WorksheetInputError::Required => text.field_required,
                    WorksheetInputError::InvalidNumber => text.field_number,
                };
                format!("{}: {reason}", field.label(lang))
            }
            Self::Calculation(error) => i18n::runtime_error(lang, error.message()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorksheetOutcome {
    pub target: WorksheetField,
    pub value: f64,
}

/// Worksheet state shared by Win32 and other frontends.
pub struct WorksheetSession {
    kind: WorksheetKind,
    target: WorksheetField,
    values: Vec<String>,
    result: Option<Result<WorksheetOutcome, WorksheetError>>,
}

impl WorksheetSession {
    pub fn new(kind: WorksheetKind) -> Self {
        let fields = fields_for(kind);
        let def = units::worksheet_def(kind);
        let target = fields.get(def.default_target).copied().unwrap_or(fields[0]);
        let mut values = vec![String::new(); fields.len()];
        if kind == WorksheetKind::Mortgage {
            if let Some(down_payment) = fields
                .iter()
                .position(|field| *field == WorksheetField::DownPayment)
            {
                values[down_payment] = "0".to_string();
            }
        }
        Self {
            kind,
            target,
            values,
            result: None,
        }
    }

    pub fn kind(&self) -> WorksheetKind {
        self.kind
    }

    /// All fields in their worksheet/display order, including non-targetable
    /// fields such as interest rate and payments per year.
    pub fn fields(&self) -> &'static [WorksheetField] {
        fields_for(self.kind)
    }

    /// Fields the user may select as the solve target, in combo-box order.
    pub fn target_options(&self) -> &'static [WorksheetField] {
        targets_for(self.kind)
    }

    pub fn target(&self) -> WorksheetField {
        self.target
    }

    /// Select a valid solve target. Invalid fields leave the session unchanged.
    pub fn set_target(&mut self, target: WorksheetField) -> bool {
        if !self.target_options().contains(&target) {
            return false;
        }
        self.target = target;
        self.result = None;
        true
    }

    /// Raw, unparsed draft text. Returns `None` for a field from another mode.
    pub fn text(&self, field: WorksheetField) -> Option<&str> {
        self.fields()
            .iter()
            .position(|candidate| *candidate == field)
            .and_then(|index| self.values.get(index))
            .map(String::as_str)
    }

    /// Store raw draft text without normalizing it. Any edit invalidates the
    /// previous calculation, including edits to the currently selected target.
    pub fn set_text(&mut self, field: WorksheetField, value: String) -> bool {
        let Some(index) = self
            .fields()
            .iter()
            .position(|candidate| *candidate == field)
        else {
            return false;
        };
        self.values[index] = value;
        self.result = None;
        true
    }

    /// Last calculation, if any. The result is cleared by edits or target
    /// changes; raw drafts are retained.
    pub fn result(&self) -> Option<Result<WorksheetOutcome, WorksheetError>> {
        self.result.clone()
    }

    pub fn calculate(&mut self) -> Result<WorksheetOutcome, WorksheetError> {
        let result = self.evaluate();
        self.result = Some(result.clone());
        result
    }

    /// Formatted numeric output for a frontend's grouping preference.
    pub fn formatted_result(&self, grouping: bool) -> Option<String> {
        match self.result.as_ref()? {
            Ok(outcome) => Some(engine::format_number(outcome.value, grouping)),
            Err(_) => None,
        }
    }

    /// Localized error corresponding to the last failed calculation.
    pub fn display_error(&self, lang: Lang) -> Option<String> {
        match self.result.as_ref()? {
            Err(error) => Some(error.display(lang)),
            Ok(_) => None,
        }
    }

    fn evaluate(&self) -> Result<WorksheetOutcome, WorksheetError> {
        let fields = self.fields();
        let mut values = Vec::with_capacity(fields.len());
        for (field, text) in fields.iter().zip(self.values.iter()) {
            if *field == self.target {
                values.push(f64::NAN);
                continue;
            }
            match engine::parse_number(text).filter(|value| value.is_finite()) {
                Some(value) => values.push(value),
                None => {
                    return Err(WorksheetError::InvalidInput {
                        field: *field,
                        reason: if text.trim().is_empty() {
                            WorksheetInputError::Required
                        } else {
                            WorksheetInputError::InvalidNumber
                        },
                    });
                }
            }
        }

        let target = fields
            .iter()
            .position(|field| *field == self.target)
            .unwrap_or(0);
        units::solve_worksheet(self.kind, target, &values)
            .map(|value| WorksheetOutcome {
                target: self.target,
                value,
            })
            .map_err(|message| {
                WorksheetError::Calculation(WorksheetCalculationError::from_solver_message(message))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_order_and_target_options_match_existing_worksheet_definitions() {
        for kind in [
            WorksheetKind::Mortgage,
            WorksheetKind::VehicleLease,
            WorksheetKind::FuelMpg,
            WorksheetKind::FuelL100,
        ] {
            let session = WorksheetSession::new(kind);
            let def = units::worksheet_def(kind);
            assert_eq!(session.fields().len(), def.args.len());
            for (field, label) in session.fields().iter().zip(def.args) {
                assert_eq!(field.label_key(), *label);
            }
            let targets: Vec<_> = session
                .fields()
                .iter()
                .enumerate()
                .filter(|(index, _)| !def.non_computable.contains(index))
                .map(|(_, field)| *field)
                .collect();
            assert_eq!(session.target_options(), targets);
            assert!(session.target_options().contains(&session.target()));
        }
    }

    #[test]
    fn defaults_and_target_changes_preserve_raw_drafts_and_clear_results() {
        let mut session = WorksheetSession::new(WorksheetKind::Mortgage);
        assert_eq!(session.target(), WorksheetField::MonthlyPayment);
        assert_eq!(session.text(WorksheetField::DownPayment), Some("0"));
        assert_eq!(session.text(WorksheetField::PurchasePrice), Some(""));

        session.set_text(WorksheetField::PurchasePrice, "120000.00".into());
        session.set_text(WorksheetField::TermYears, " 20 ".into());
        session.set_text(WorksheetField::InterestRate, "5,25".into());
        let result = session.calculate().unwrap();
        assert_eq!(result.target, WorksheetField::MonthlyPayment);
        assert!(result.value.is_finite() && result.value > 0.0);
        assert!(session.formatted_result(false).is_some());

        assert!(!session.set_target(WorksheetField::InterestRate));
        assert!(session.result().is_some());
        assert!(session.set_target(WorksheetField::PurchasePrice));
        assert_eq!(session.result(), None);
        assert_eq!(
            session.text(WorksheetField::PurchasePrice),
            Some("120000.00")
        );
        assert_eq!(session.text(WorksheetField::TermYears), Some(" 20 "));
        assert_eq!(session.text(WorksheetField::InterestRate), Some("5,25"));
    }

    #[test]
    fn input_errors_are_typed_localized_and_raw_drafts_survive() {
        let mut session = WorksheetSession::new(WorksheetKind::FuelMpg);
        session.set_text(WorksheetField::DistanceMiles, "  ".into());
        session.set_text(WorksheetField::FuelGallons, "not a number".into());

        assert_eq!(
            session.calculate(),
            Err(WorksheetError::InvalidInput {
                field: WorksheetField::DistanceMiles,
                reason: WorksheetInputError::Required,
            })
        );
        assert_eq!(
            session.display_error(Lang::En).as_deref(),
            Some("Distance (miles): field is required")
        );
        assert_eq!(
            session.display_error(Lang::Ru).as_deref(),
            Some("Расстояние (мили): заполните поле")
        );
        assert_eq!(session.text(WorksheetField::DistanceMiles), Some("  "));

        session.set_text(WorksheetField::DistanceMiles, "300".into());
        assert_eq!(
            session.calculate(),
            Err(WorksheetError::InvalidInput {
                field: WorksheetField::FuelGallons,
                reason: WorksheetInputError::InvalidNumber,
            })
        );
        assert_eq!(
            session.display_error(Lang::En).as_deref(),
            Some("Fuel used (gallons): enter a number")
        );
    }

    #[test]
    fn every_computable_target_uses_the_shared_solver_and_returns_typed_outcomes() {
        for (kind, mut values) in [
            (
                WorksheetKind::Mortgage,
                vec![120000.0, 20000.0, 20.0, 5.0, 0.0],
            ),
            (
                WorksheetKind::VehicleLease,
                vec![30000.0, 3.0, 12.0, 15000.0, 6.0, 0.0],
            ),
            (WorksheetKind::FuelMpg, vec![300.0, 10.0, 0.0]),
            (WorksheetKind::FuelL100, vec![300.0, 10.0, 0.0]),
        ] {
            let mut session = WorksheetSession::new(kind);
            for (field, value) in session.fields().iter().zip(values.iter()) {
                session.set_text(*field, value.to_string());
            }

            let default = session.calculate().unwrap();
            assert_eq!(default.target, session.target());
            let default_index = session
                .fields()
                .iter()
                .position(|field| *field == default.target)
                .unwrap();
            values[default_index] = default.value;
            session.set_text(default.target, default.value.to_string());

            for target in session.target_options().iter().copied() {
                let target_index = session
                    .fields()
                    .iter()
                    .position(|field| *field == target)
                    .unwrap();
                assert!(session.set_target(target));
                let outcome = session.calculate().unwrap();
                assert_eq!(outcome.target, target);
                assert!(
                    (outcome.value - values[target_index]).abs()
                        < 1e-6 * values[target_index].abs().max(1.0),
                    "{kind:?} target {target:?}: {} != {}",
                    outcome.value,
                    values[target_index]
                );
            }
        }
    }

    #[test]
    fn calculation_errors_remain_typed_and_edits_clear_them() {
        let mut session = WorksheetSession::new(WorksheetKind::FuelL100);
        session.set_text(WorksheetField::DistanceKilometers, "0".into());
        session.set_text(WorksheetField::FuelLiters, "10".into());

        assert_eq!(
            session.calculate(),
            Err(WorksheetError::Calculation(
                WorksheetCalculationError::InvalidValue
            ))
        );
        assert_eq!(
            session.display_error(Lang::En).as_deref(),
            Some("This value is not valid.")
        );

        session.set_text(WorksheetField::DistanceKilometers, "100".into());
        assert_eq!(session.result(), None);
        assert_eq!(session.display_error(Lang::En), None);
    }
}
