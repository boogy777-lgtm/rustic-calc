use crate::units::{self, Date};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateField {
    From,
    To,
    Years,
    Months,
    Days,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateMode {
    Difference,
    Shift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateOperation {
    Add,
    Subtract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateOutcome {
    Difference {
        years: i32,
        months: i32,
        weeks: i32,
        days: i32,
        total_days: i64,
    },
    Shifted(Date),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateError {
    InvalidDate(DateField),
    InvalidOffset(DateField),
    OutOfBounds,
}

/// Owns raw drafts and typed results. The frontend supplies the current date
/// and manages the editors and localized display independently.
pub struct DateSession {
    mode: DateMode,
    operation: DateOperation,
    from: String,
    to: String,
    years: String,
    months: String,
    days: String,
    result: Option<Result<DateOutcome, DateError>>,
}

impl DateSession {
    pub fn new(today: Date) -> Self {
        let text = units::format_date(today);
        Self {
            mode: DateMode::Difference,
            operation: DateOperation::Add,
            from: text.clone(),
            to: text,
            years: "0".into(),
            months: "0".into(),
            days: "0".into(),
            result: None,
        }
    }

    pub fn mode(&self) -> DateMode {
        self.mode
    }

    pub fn operation(&self) -> DateOperation {
        self.operation
    }

    pub fn set_mode(&mut self, mode: DateMode) {
        self.mode = mode;
        self.result = None;
    }

    pub fn set_operation(&mut self, operation: DateOperation) {
        self.operation = operation;
        self.result = None;
    }

    pub fn text(&self, field: DateField) -> &str {
        match field {
            DateField::From => &self.from,
            DateField::To => &self.to,
            DateField::Years => &self.years,
            DateField::Months => &self.months,
            DateField::Days => &self.days,
        }
    }

    pub fn set_text(&mut self, field: DateField, value: String) {
        *match field {
            DateField::From => &mut self.from,
            DateField::To => &mut self.to,
            DateField::Years => &mut self.years,
            DateField::Months => &mut self.months,
            DateField::Days => &mut self.days,
        } = value;
        self.result = None;
    }

    pub fn result(&self) -> Option<Result<DateOutcome, DateError>> {
        self.result
    }

    pub fn calculate(&mut self) {
        self.result = Some(self.evaluate());
    }

    fn evaluate(&self) -> Result<DateOutcome, DateError> {
        let read_date =
            |field| units::parse_date(self.text(field)).ok_or(DateError::InvalidDate(field));
        let from = read_date(DateField::From)?;
        match self.mode {
            DateMode::Difference => {
                let to = read_date(DateField::To)?;
                let (years, months, weeks, days, total_days) = units::date_difference(from, to);
                Ok(DateOutcome::Difference {
                    years,
                    months,
                    weeks,
                    days,
                    total_days,
                })
            }
            DateMode::Shift => {
                let read_offset = |field| {
                    self.text(field)
                        .trim()
                        .parse::<i32>()
                        .ok()
                        .filter(|n| *n >= 0)
                        .ok_or(DateError::InvalidOffset(field))
                };
                units::shift_date(
                    from,
                    read_offset(DateField::Years)?,
                    read_offset(DateField::Months)?,
                    read_offset(DateField::Days)?,
                    self.operation == DateOperation::Add,
                )
                .map(DateOutcome::Shifted)
                .ok_or(DateError::OutOfBounds)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> DateSession {
        DateSession::new(Date::new(2024, 2, 29).unwrap())
    }

    #[test]
    fn switching_modes_preserves_all_raw_drafts_and_clears_previous_result() {
        let mut date = session();
        assert_eq!(date.text(DateField::From), "29.02.2024");
        date.set_text(DateField::To, " 15.03. ".into());
        date.calculate();
        assert_eq!(
            date.result(),
            Some(Err(DateError::InvalidDate(DateField::To)))
        );

        date.set_mode(DateMode::Shift);
        assert_eq!(date.result(), None);
        date.set_text(DateField::Months, " -1 ".into());
        date.calculate();
        assert_eq!(
            date.result(),
            Some(Err(DateError::InvalidOffset(DateField::Months)))
        );
        date.set_mode(DateMode::Difference);
        assert_eq!(date.text(DateField::To), " 15.03. ");
        date.set_mode(DateMode::Shift);
        assert_eq!(date.text(DateField::Months), " -1 ");
    }

    #[test]
    fn date_sessions_return_typed_results_and_clear_them_after_edits() {
        let mut date = session();
        date.set_text(DateField::To, "2024-03-07".into());
        date.calculate();
        assert_eq!(
            date.result(),
            Some(Ok(DateOutcome::Difference {
                years: 0,
                months: 0,
                weeks: 1,
                days: 0,
                total_days: 7,
            }))
        );
        date.set_mode(DateMode::Shift);
        date.set_operation(DateOperation::Subtract);
        date.set_text(DateField::Years, " 1 ".into());
        date.calculate();
        assert_eq!(
            date.result(),
            Some(Ok(DateOutcome::Shifted(Date::new(2023, 2, 28).unwrap())))
        );
        date.set_text(DateField::Years, i32::MAX.to_string());
        assert_eq!(date.result(), None);
        date.calculate();
        assert_eq!(date.result(), Some(Err(DateError::OutOfBounds)));
    }
}
