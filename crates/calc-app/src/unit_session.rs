//! Pure state and typed results for unit conversion and the DCS loop helper.

use crate::engine::parse_number;
use crate::units::{self, Category, CategoryId, LoopResistor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitField {
    Value,
    LoopVoltage,
    LoopCurrentMa,
}

/// A unit's position within the selected category.
///
/// Unit definitions currently use table order. This category-relative index is
/// validated when selected; callers should not persist it across table changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitIndex(usize);

impl UnitIndex {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSelectionError {
    UnknownCategory(CategoryId),
    UnitIndexOutOfRange {
        category: CategoryId,
        index: UnitIndex,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConversionResult {
    /// No input has been entered (or the draft contains only whitespace).
    Empty,
    /// The draft is not a finite number.
    InvalidInput,
    /// The converted value. It may be infinite if the conversion overflows;
    /// frontends can render that with the shared number formatter.
    Value(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DcsLoopError {
    InvalidVoltage,
    InvalidCurrent,
    NonPositiveVoltage,
    NonPositiveCurrent,
    OutOfRange,
}

/// Shared unit-mode drafts and selections.
///
/// This type owns no frontend controls and performs no I/O. Conversion results
/// are derived from the current drafts, so edits are immediately reflected and
/// unsaved text survives frontend control recreation.
pub struct UnitSession {
    categories: Vec<Category>,
    category: CategoryId,
    from: UnitIndex,
    to: UnitIndex,
    value: String,
    loop_voltage: String,
    loop_current_ma: String,
}

impl Default for UnitSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitSession {
    pub fn new() -> Self {
        let categories = units::categories();
        let category = categories
            .iter()
            .find(|category| category.id == CategoryId::Angle)
            .or_else(|| categories.first())
            .map(|category| category.id)
            .expect("unit table must contain at least one category");
        Self {
            categories,
            category,
            from: UnitIndex::new(0),
            to: UnitIndex::new(0),
            value: String::new(),
            loop_voltage: "24".to_string(),
            loop_current_ma: "12".to_string(),
        }
    }

    /// All conversion categories and their units, in display order.
    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn category(&self) -> &Category {
        self.categories
            .iter()
            .find(|category| category.id == self.category)
            .expect("selected category is always present in the unit table")
    }

    pub fn category_id(&self) -> CategoryId {
        self.category
    }

    /// The selected category's position, for index-based native combo boxes.
    pub fn category_index(&self) -> usize {
        self.categories
            .iter()
            .position(|category| category.id == self.category)
            .unwrap_or(0)
    }

    pub fn from_index(&self) -> UnitIndex {
        self.from
    }

    pub fn to_index(&self) -> UnitIndex {
        self.to
    }

    /// Select a category and reset both unit selectors to their first entries.
    pub fn select_category(&mut self, category: CategoryId) -> Result<(), UnitSelectionError> {
        if !self.categories.iter().any(|item| item.id == category) {
            return Err(UnitSelectionError::UnknownCategory(category));
        }
        self.category = category;
        self.from = UnitIndex::new(0);
        self.to = UnitIndex::new(0);
        Ok(())
    }

    pub fn select_from(&mut self, index: UnitIndex) -> Result<(), UnitSelectionError> {
        self.validate_index(index)?;
        self.from = index;
        Ok(())
    }

    pub fn select_to(&mut self, index: UnitIndex) -> Result<(), UnitSelectionError> {
        self.validate_index(index)?;
        self.to = index;
        Ok(())
    }

    fn validate_index(&self, index: UnitIndex) -> Result<(), UnitSelectionError> {
        if index.get() >= self.category().units.len() {
            Err(UnitSelectionError::UnitIndexOutOfRange {
                category: self.category,
                index,
            })
        } else {
            Ok(())
        }
    }

    pub fn text(&self, field: UnitField) -> &str {
        match field {
            UnitField::Value => &self.value,
            UnitField::LoopVoltage => &self.loop_voltage,
            UnitField::LoopCurrentMa => &self.loop_current_ma,
        }
    }

    pub fn set_text(&mut self, field: UnitField, value: String) {
        *match field {
            UnitField::Value => &mut self.value,
            UnitField::LoopVoltage => &mut self.loop_voltage,
            UnitField::LoopCurrentMa => &mut self.loop_current_ma,
        } = value;
    }

    pub fn conversion_result(&self) -> ConversionResult {
        let value = match parse_number(&self.value) {
            Some(value) if value.is_finite() => value,
            _ if self.value.trim().is_empty() => return ConversionResult::Empty,
            _ => return ConversionResult::InvalidInput,
        };
        let category = self.category();
        ConversionResult::Value(units::convert(
            value,
            &category.units[self.from.get()],
            &category.units[self.to.get()],
        ))
    }

    /// Returns `None` outside DCS Signals; otherwise returns the typed helper
    /// result, including a typed error for incomplete or invalid drafts.
    pub fn dcs_loop_result(&self) -> Option<Result<LoopResistor, DcsLoopError>> {
        if self.category != CategoryId::DcsSignals {
            return None;
        }

        let voltage = parse_number(&self.loop_voltage)
            .filter(|value| value.is_finite())
            .ok_or(DcsLoopError::InvalidVoltage)
            .and_then(|value| {
                if value > 0.0 {
                    Ok(value)
                } else {
                    Err(DcsLoopError::NonPositiveVoltage)
                }
            });
        let current_ma = parse_number(&self.loop_current_ma)
            .filter(|value| value.is_finite())
            .ok_or(DcsLoopError::InvalidCurrent)
            .and_then(|value| {
                if value > 0.0 {
                    Ok(value)
                } else {
                    Err(DcsLoopError::NonPositiveCurrent)
                }
            });

        Some(match (voltage, current_ma) {
            (Ok(voltage), Ok(current_ma)) => {
                units::loop_resistor(voltage, current_ma).ok_or(DcsLoopError::OutOfRange)
            }
            (Err(error), _) | (_, Err(error)) => Err(error),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_existing_unit_view() {
        let session = UnitSession::new();
        assert_eq!(session.category_id(), CategoryId::Angle);
        assert_eq!(session.from_index(), UnitIndex::new(0));
        assert_eq!(session.to_index(), UnitIndex::new(0));
        assert_eq!(session.text(UnitField::Value), "");
        assert_eq!(session.text(UnitField::LoopVoltage), "24");
        assert_eq!(session.text(UnitField::LoopCurrentMa), "12");
        assert_eq!(session.conversion_result(), ConversionResult::Empty);
        assert!(session.dcs_loop_result().is_none());
    }

    #[test]
    fn typed_category_and_unit_selection_preserve_existing_conversion_semantics() {
        let mut session = UnitSession::new();
        session.select_category(CategoryId::Temperature).unwrap();
        session
            .select_from(UnitIndex::new(1))
            .expect("Fahrenheit is available");
        session
            .select_to(UnitIndex::new(0))
            .expect("Celsius is available");
        session.set_text(UnitField::Value, "32".into());
        assert_eq!(session.conversion_result(), ConversionResult::Value(0.0));

        session.select_category(CategoryId::Angle).unwrap();
        assert_eq!(session.from_index(), UnitIndex::new(0));
        assert_eq!(session.to_index(), UnitIndex::new(0));
        assert_eq!(session.text(UnitField::Value), "32");
    }

    #[test]
    fn invalid_indexes_are_rejected_without_mutating_the_selection() {
        let mut session = UnitSession::new();
        let selected = session.from_index();
        let out_of_range = UnitIndex::new(session.category().units.len());
        assert_eq!(
            session.select_from(out_of_range),
            Err(UnitSelectionError::UnitIndexOutOfRange {
                category: CategoryId::Angle,
                index: out_of_range,
            })
        );
        assert_eq!(session.from_index(), selected);
    }

    #[test]
    fn conversion_distinguishes_empty_invalid_and_non_finite_text() {
        let mut session = UnitSession::new();
        for text in ["", "  "] {
            session.set_text(UnitField::Value, text.into());
            assert_eq!(session.conversion_result(), ConversionResult::Empty);
        }
        for text in ["-", "not a number", "NaN", "inf"] {
            session.set_text(UnitField::Value, text.into());
            assert_eq!(session.conversion_result(), ConversionResult::InvalidInput);
        }
    }

    #[test]
    fn dcs_loop_returns_typed_results_and_uses_the_existing_defaults() {
        let mut session = UnitSession::new();
        session.select_category(CategoryId::DcsSignals).unwrap();
        session.select_from(UnitIndex::new(0)).unwrap();
        session.select_to(UnitIndex::new(1)).unwrap();
        session.set_text(UnitField::Value, "50".into());
        assert_eq!(session.conversion_result(), ConversionResult::Value(12.0));

        let loop_result = session.dcs_loop_result().unwrap().unwrap();
        assert_eq!(loop_result.resistance, 2000.0);
        assert_eq!(loop_result.power, 0.288);
        assert_eq!(loop_result.standard_resistance, 2000.0);
        assert_eq!(loop_result.actual_current_ma, 12.0);
        assert_eq!(loop_result.recommended_wattage, 0.5);

        session.set_text(UnitField::LoopVoltage, "0".into());
        assert!(matches!(
            session.dcs_loop_result(),
            Some(Err(DcsLoopError::NonPositiveVoltage))
        ));
        session.set_text(UnitField::LoopVoltage, "not a number".into());
        assert!(matches!(
            session.dcs_loop_result(),
            Some(Err(DcsLoopError::InvalidVoltage))
        ));
        session.select_category(CategoryId::Angle).unwrap();
        assert!(session.dcs_loop_result().is_none());
    }

    #[test]
    fn dcs_drafts_survive_category_switches_while_unit_choices_reset() {
        let mut session = UnitSession::new();
        session.select_category(CategoryId::DcsSignals).unwrap();
        session.set_text(UnitField::Value, "50".into());
        session.set_text(UnitField::LoopVoltage, "36".into());
        session.set_text(UnitField::LoopCurrentMa, "18".into());
        session.select_to(UnitIndex::new(1)).unwrap();

        session.select_category(CategoryId::Angle).unwrap();
        assert_eq!(session.conversion_result(), ConversionResult::Value(50.0));
        assert!(session.dcs_loop_result().is_none());
        session.select_category(CategoryId::DcsSignals).unwrap();

        assert_eq!(session.from_index(), UnitIndex::new(0));
        assert_eq!(session.to_index(), UnitIndex::new(0));
        assert_eq!(session.text(UnitField::Value), "50");
        assert_eq!(session.text(UnitField::LoopVoltage), "36");
        assert_eq!(session.text(UnitField::LoopCurrentMa), "18");
        assert_eq!(session.conversion_result(), ConversionResult::Value(50.0));
        assert_eq!(
            session.dcs_loop_result().unwrap().unwrap().resistance,
            2000.0
        );
    }
}
