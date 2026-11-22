//! Platform-independent state for the WGS84 → local Transverse Mercator mode.
//!
//! Frontends own their editors and localized presentation; this session owns
//! stable region/template selection, raw drafts, validation and typed results.

use std::collections::BTreeMap;

use crate::coordinates::{catalogue, Crs, Error, Wgs84};
use crate::i18n::coordinates::{self as coordinate_text, Region};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoordinateRegionId(String);

impl CoordinateRegionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CoordinateRegionId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for CoordinateRegionId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoordinateTemplateId(String);

impl CoordinateTemplateId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CoordinateTemplateId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for CoordinateTemplateId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateTemplateSelection {
    Custom,
    Template(CoordinateTemplateId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinateTemplate {
    pub id: CoordinateTemplateId,
    pub name: &'static str,
    pub definition: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateInputField {
    Latitude,
    Longitude,
    Height,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateOutput {
    pub northing_m: f64,
    pub easting_m: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateSelectionError {
    UnknownRegion(CoordinateRegionId),
    TemplateNotInRegion {
        region: CoordinateRegionId,
        template: CoordinateTemplateId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DraftKey {
    Custom(CoordinateRegionId),
    Template(CoordinateTemplateId),
}

/// Owns coordinate inputs, exact per-template/per-region drafts and results.
/// It has no windowing, filesystem, localization policy or platform dependency.
pub struct CoordinateSession {
    region: CoordinateRegionId,
    selection: CoordinateTemplateSelection,
    definition: String,
    latitude: String,
    longitude: String,
    height: String,
    drafts: BTreeMap<DraftKey, String>,
    region_selections: BTreeMap<CoordinateRegionId, CoordinateTemplateSelection>,
    result: Option<Result<CoordinateOutput, Error>>,
}

impl CoordinateSession {
    /// Starts in the Moscow Oblast group, using its first bundled template.
    pub fn new() -> Self {
        let region = CoordinateRegionId::from("50");
        let first_template = catalogue::for_region(region.as_str())
            .first()
            .map(|template| CoordinateTemplateSelection::Template(template.id.into()))
            .unwrap_or(CoordinateTemplateSelection::Custom);
        let definition = match &first_template {
            CoordinateTemplateSelection::Custom => String::new(),
            CoordinateTemplateSelection::Template(id) => catalogue::for_region(region.as_str())
                .into_iter()
                .find(|template| template.id == id.as_str())
                .map(|template| template.definition.to_owned())
                .unwrap_or_default(),
        };
        Self {
            region,
            selection: first_template,
            definition,
            latitude: String::new(),
            longitude: String::new(),
            height: "0".into(),
            drafts: BTreeMap::new(),
            region_selections: BTreeMap::new(),
            result: None,
        }
    }

    /// Region records provide stable IDs and localized names for frontend lists.
    pub fn regions() -> &'static [Region] {
        coordinate_text::REGIONS
    }

    /// Available templates for the currently selected region.
    pub fn templates(&self) -> Vec<CoordinateTemplate> {
        templates_for(&self.region)
    }

    pub fn region(&self) -> &CoordinateRegionId {
        &self.region
    }

    pub fn selection(&self) -> &CoordinateTemplateSelection {
        &self.selection
    }

    pub fn selected_template(&self) -> Option<CoordinateTemplate> {
        let CoordinateTemplateSelection::Template(id) = &self.selection else {
            return None;
        };
        self.templates()
            .into_iter()
            .find(|template| template.id == *id)
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }

    /// Replaces the raw PROJ draft without normalizing or partially parsing it.
    pub fn set_definition(&mut self, definition: impl Into<String>) {
        let definition = definition.into();
        if self.definition == definition {
            return;
        }
        self.definition = definition;
        self.drafts
            .insert(self.draft_key(), self.definition.clone());
        self.result = None;
    }

    pub fn text(&self, field: CoordinateInputField) -> &str {
        match field {
            CoordinateInputField::Latitude => &self.latitude,
            CoordinateInputField::Longitude => &self.longitude,
            CoordinateInputField::Height => &self.height,
        }
    }

    pub fn set_text(&mut self, field: CoordinateInputField, value: impl Into<String>) {
        let slot = match field {
            CoordinateInputField::Latitude => &mut self.latitude,
            CoordinateInputField::Longitude => &mut self.longitude,
            CoordinateInputField::Height => &mut self.height,
        };
        let value = value.into();
        if *slot == value {
            return;
        }
        *slot = value;
        self.result = None;
    }

    /// Selects a region by stable ID. The current draft and selection are kept.
    pub fn select_region(
        &mut self,
        region: impl Into<CoordinateRegionId>,
    ) -> Result<(), CoordinateSelectionError> {
        let region = region.into();
        if !coordinate_text::REGIONS
            .iter()
            .any(|known| known.id == region.as_str())
        {
            return Err(CoordinateSelectionError::UnknownRegion(region));
        }

        self.remember_current();
        self.region = region;
        self.selection = self
            .region_selections
            .get(&self.region)
            .filter(|selection| self.selection_is_valid(selection, &self.region))
            .cloned()
            .unwrap_or_else(|| default_selection(&self.region));
        self.load_definition();
        Ok(())
    }

    /// Selects the custom definition for the current region.
    pub fn select_custom(&mut self) {
        self.remember_current();
        self.selection = CoordinateTemplateSelection::Custom;
        self.load_definition();
    }

    /// Selects a template only when it belongs to the active region.
    pub fn select_template(
        &mut self,
        template: impl Into<CoordinateTemplateId>,
    ) -> Result<(), CoordinateSelectionError> {
        let template = template.into();
        if !catalogue::for_region(self.region.as_str())
            .iter()
            .any(|known| known.id == template.as_str())
        {
            return Err(CoordinateSelectionError::TemplateNotInRegion {
                region: self.region.clone(),
                template,
            });
        }
        self.remember_current();
        self.selection = CoordinateTemplateSelection::Template(template);
        self.load_definition();
        Ok(())
    }

    /// Restores the selected template's original definition or clears custom.
    pub fn reset_definition(&mut self) {
        self.definition = self
            .selected_template()
            .map(|template| template.definition.to_owned())
            .unwrap_or_default();
        self.drafts
            .insert(self.draft_key(), self.definition.clone());
        self.region_selections
            .insert(self.region.clone(), self.selection.clone());
        self.result = None;
    }

    /// Calculates and stores a typed result or the original domain error.
    pub fn calculate(&mut self) -> Result<CoordinateOutput, Error> {
        let result = Crs::parse(&self.definition)
            .and_then(|crs| {
                Wgs84::parse(&self.latitude, &self.longitude, &self.height)
                    .and_then(|point| crs.convert(point))
            })
            .map(|point| CoordinateOutput {
                northing_m: point.northing_m,
                easting_m: point.easting_m,
            });
        self.result = Some(result.clone());
        result
    }

    pub fn result(&self) -> Option<&Result<CoordinateOutput, Error>> {
        self.result.as_ref()
    }

    fn draft_key(&self) -> DraftKey {
        match &self.selection {
            CoordinateTemplateSelection::Custom => DraftKey::Custom(self.region.clone()),
            CoordinateTemplateSelection::Template(id) => DraftKey::Template(id.clone()),
        }
    }

    fn remember_current(&mut self) {
        self.drafts
            .insert(self.draft_key(), self.definition.clone());
        self.region_selections
            .insert(self.region.clone(), self.selection.clone());
    }

    fn load_definition(&mut self) {
        let key = self.draft_key();
        self.definition = self
            .drafts
            .get(&key)
            .cloned()
            .unwrap_or_else(|| match &self.selection {
                CoordinateTemplateSelection::Custom => String::new(),
                CoordinateTemplateSelection::Template(id) => {
                    catalogue::for_region(self.region.as_str())
                        .into_iter()
                        .find(|template| template.id == id.as_str())
                        .map(|template| template.definition.to_owned())
                        .unwrap_or_default()
                }
            });
        self.result = None;
    }

    fn selection_is_valid(
        &self,
        selection: &CoordinateTemplateSelection,
        region: &CoordinateRegionId,
    ) -> bool {
        match selection {
            CoordinateTemplateSelection::Custom => true,
            CoordinateTemplateSelection::Template(id) => catalogue::for_region(region.as_str())
                .iter()
                .any(|template| template.id == id.as_str()),
        }
    }
}

impl Default for CoordinateSession {
    fn default() -> Self {
        Self::new()
    }
}

fn templates_for(region: &CoordinateRegionId) -> Vec<CoordinateTemplate> {
    catalogue::for_region(region.as_str())
        .into_iter()
        .map(|template| CoordinateTemplate {
            id: template.id.into(),
            name: template.name,
            definition: template.definition,
        })
        .collect()
}

fn default_selection(region: &CoordinateRegionId) -> CoordinateTemplateSelection {
    catalogue::for_region(region.as_str())
        .first()
        .map(|template| CoordinateTemplateSelection::Template(template.id.into()))
        .unwrap_or(CoordinateTemplateSelection::Custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::CoordinateField;

    #[test]
    fn defaults_expose_stable_region_template_and_raw_input_state() {
        let session = CoordinateSession::new();
        assert_eq!(session.region().as_str(), "50");
        assert_eq!(
            session.selection(),
            &CoordinateTemplateSelection::Template("MSK50z1".into())
        );
        assert_eq!(session.templates().len(), 2);
        assert_eq!(session.text(CoordinateInputField::Latitude), "");
        assert_eq!(session.text(CoordinateInputField::Longitude), "");
        assert_eq!(session.text(CoordinateInputField::Height), "0");
        assert!(session.result().is_none());
    }

    #[test]
    fn each_template_and_region_custom_draft_and_selection_are_independent() {
        let mut session = CoordinateSession::new();
        let first = match session.selection() {
            CoordinateTemplateSelection::Template(id) => id.clone(),
            CoordinateTemplateSelection::Custom => panic!("Moscow has templates"),
        };
        let second = session.templates()[1].id.clone();

        session.set_definition("first edited draft");
        session.select_template(second.clone()).unwrap();
        assert_ne!(session.definition(), "first edited draft");
        session.set_definition("second edited draft");

        session.select_custom();
        assert_eq!(session.definition(), "");
        session.set_definition("Moscow custom draft");

        session.select_region("01").unwrap();
        assert_eq!(
            session.selection(),
            &CoordinateTemplateSelection::Template("MSK01z1".into())
        );
        session.set_definition("regional template draft");
        session.select_custom();
        session.set_definition("regional custom draft");

        session.select_region("50").unwrap();
        assert_eq!(session.selection(), &CoordinateTemplateSelection::Custom);
        assert_eq!(session.definition(), "Moscow custom draft");
        session.select_template(first.clone()).unwrap();
        assert_eq!(session.definition(), "first edited draft");
        session.select_template(second).unwrap();
        assert_eq!(session.definition(), "second edited draft");

        session.select_region("01").unwrap();
        assert_eq!(session.definition(), "regional custom draft");
        session.select_template("MSK01z1").unwrap();
        assert_eq!(session.definition(), "regional template draft");
    }

    #[test]
    fn invalid_selection_is_typed_and_does_not_change_session_state() {
        let mut session = CoordinateSession::new();
        let original_region = session.region().clone();
        let original_selection = session.selection().clone();
        let original_definition = session.definition().to_owned();

        assert_eq!(
            session.select_region("not-a-region"),
            Err(CoordinateSelectionError::UnknownRegion(
                "not-a-region".into()
            ))
        );
        assert_eq!(
            session.select_template("MSK01z1"),
            Err(CoordinateSelectionError::TemplateNotInRegion {
                region: "50".into(),
                template: "MSK01z1".into(),
            })
        );
        assert_eq!(session.region(), &original_region);
        assert_eq!(session.selection(), &original_selection);
        assert_eq!(session.definition(), original_definition);
    }

    #[test]
    fn calculation_returns_typed_output_and_original_coordinate_errors() {
        let mut session = CoordinateSession::new();
        session.set_text(CoordinateInputField::Latitude, "55,7558");
        session.set_text(CoordinateInputField::Longitude, "37.6173");
        session.set_text(CoordinateInputField::Height, "150");
        let output = session.calculate().unwrap();
        assert!(output.northing_m.is_finite());
        assert!(output.easting_m.is_finite());
        assert_eq!(session.result(), Some(&Ok(output)));

        session.set_definition(session.definition().to_owned());
        session.set_text(CoordinateInputField::Latitude, "55,7558");
        assert_eq!(
            session.result(),
            Some(&Ok(output)),
            "syncing unchanged native drafts must preserve the result"
        );
        session.set_text(CoordinateInputField::Latitude, "90");
        assert!(session.result().is_none(), "edits invalidate stale output");
        assert_eq!(
            session.calculate(),
            Err(Error::Coordinate(CoordinateField::Latitude))
        );
        assert_eq!(
            session.result(),
            Some(&Err(Error::Coordinate(CoordinateField::Latitude)))
        );
    }

    #[test]
    fn reset_only_reverts_the_selected_draft_and_preserves_other_drafts() {
        let mut session = CoordinateSession::new();
        let original = session.definition().to_owned();
        session.set_definition("edited template");
        session.reset_definition();
        assert_eq!(session.definition(), original);

        session.select_custom();
        session.set_definition("custom definition");
        session.reset_definition();
        assert_eq!(session.definition(), "");
        session.select_template("MSK50z1").unwrap();
        assert_eq!(session.definition(), original);
    }

    #[test]
    fn failed_projection_keeps_parser_error_typed() {
        let mut session = CoordinateSession::new();
        session.set_definition("+proj=other");
        assert_eq!(
            session.calculate(),
            Err(Error::Unsupported("proj (tmerc)".into()))
        );
        assert_eq!(
            session.result(),
            Some(&Err(Error::Unsupported("proj (tmerc)".into())))
        );
    }

    #[test]
    fn region_templates_have_stable_ids_and_missing_regions_remain_custom() {
        let mut session = CoordinateSession::new();
        session.select_region("17").unwrap();
        assert!(session.templates().is_empty());
        assert_eq!(session.selection(), &CoordinateTemplateSelection::Custom);
        assert_eq!(session.definition(), "");
        assert_eq!(
            session.select_template("MSK50z1"),
            Err(CoordinateSelectionError::TemplateNotInRegion {
                region: "17".into(),
                template: "MSK50z1".into(),
            })
        );
    }
}
