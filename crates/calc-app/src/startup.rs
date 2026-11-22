//! Shared command-line options. Only frontends open windows or write PNGs.

use crate::engine::{Mode, WorksheetKind};
use crate::i18n::Lang;

pub const MODES: [Mode; 11] = [
    Mode::Standard,
    Mode::Scientific,
    Mode::Programmer,
    Mode::Statistics,
    Mode::UnitConversion,
    Mode::DateCalculation,
    Mode::Worksheet(WorksheetKind::Mortgage),
    Mode::Worksheet(WorksheetKind::VehicleLease),
    Mode::Worksheet(WorksheetKind::FuelMpg),
    Mode::Worksheet(WorksheetKind::FuelL100),
    Mode::Coordinates,
];

pub fn mode_index(mode: Mode) -> usize {
    MODES
        .iter()
        .position(|&candidate| candidate == mode)
        .expect("all modes must have a stable slot")
}

pub fn mode_title(mode: Mode, lang: Lang) -> &'static str {
    let text = lang.text();
    match mode {
        Mode::Standard => text.menu_standard,
        Mode::Scientific => text.menu_scientific,
        Mode::Programmer => text.menu_programmer,
        Mode::Statistics => text.menu_statistics,
        Mode::UnitConversion => text.menu_unit,
        Mode::DateCalculation => text.menu_date,
        Mode::Worksheet(WorksheetKind::Mortgage) => text.menu_mortgage,
        Mode::Worksheet(WorksheetKind::VehicleLease) => text.menu_lease,
        Mode::Worksheet(WorksheetKind::FuelMpg) => text.menu_mpg,
        Mode::Worksheet(WorksheetKind::FuelL100) => text.menu_l100,
        Mode::Coordinates => text.menu_coordinates,
    }
}

/// Control-modified mode keys, expressed as ASCII rather than backend key IDs.
pub const MODE_SHORTCUTS: [(u8, Mode); 13] = [
    (b'1', Mode::Standard),
    (b'2', Mode::Scientific),
    (b'3', Mode::Programmer),
    (b'4', Mode::Statistics),
    (b'5', Mode::UnitConversion),
    (b'6', Mode::DateCalculation),
    (b'7', Mode::Worksheet(WorksheetKind::Mortgage)),
    (b'8', Mode::Worksheet(WorksheetKind::VehicleLease)),
    (b'9', Mode::Worksheet(WorksheetKind::FuelMpg)),
    (b'0', Mode::Worksheet(WorksheetKind::FuelL100)),
    (b'U', Mode::UnitConversion),
    (b'E', Mode::DateCalculation),
    (b'G', Mode::Coordinates),
];

pub fn mode_shortcut(key: u8) -> Option<Mode> {
    let key = key.to_ascii_uppercase();
    MODE_SHORTCUTS
        .iter()
        .find_map(|&(shortcut, mode)| (key == shortcut).then_some(mode))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    White,
    Dark,
}

impl ThemeMode {
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::White => false,
            Self::Dark => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    pub mode: Mode,
    pub lang: Lang,
    pub theme: ThemeMode,
    pub screenshot: Option<String>,
}

impl LaunchOptions {
    /// Pass arguments after the executable name. Keep the legacy defaults and
    /// aliases; unknown options are ignored and known flags consume one value.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self {
            mode: Mode::Standard,
            lang: Lang::default(),
            theme: ThemeMode::default(),
            screenshot: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--mode" => {
                    if let Some(value) = args.next() {
                        options.mode = parse_mode(&value);
                    }
                }
                "--lang" => {
                    if let Some(value) = args.next() {
                        options.lang = match value.to_ascii_lowercase().as_str() {
                            "en" | "english" => Lang::En,
                            _ => Lang::Ru,
                        };
                    }
                }
                "--theme" => {
                    if let Some(value) = args.next() {
                        options.theme = match value.to_ascii_lowercase().as_str() {
                            "white" | "light" => ThemeMode::White,
                            "dark" => ThemeMode::Dark,
                            _ => ThemeMode::System,
                        };
                    }
                }
                "--screenshot" => {
                    if let Some(value) = args.next() {
                        options.screenshot = Some(value);
                    }
                }
                _ => {}
            }
        }
        options
    }
}

pub fn parse_mode(text: &str) -> Mode {
    match text.to_ascii_lowercase().as_str() {
        "scientific" | "sci" => Mode::Scientific,
        "programmer" | "prog" => Mode::Programmer,
        "statistics" | "stats" => Mode::Statistics,
        "unit" | "conversion" => Mode::UnitConversion,
        "date" => Mode::DateCalculation,
        "mortgage" => Mode::Worksheet(WorksheetKind::Mortgage),
        "lease" | "vehicle" => Mode::Worksheet(WorksheetKind::VehicleLease),
        "mpg" | "fuel" => Mode::Worksheet(WorksheetKind::FuelMpg),
        "l100" | "fuell" => Mode::Worksheet(WorksheetKind::FuelL100),
        "coordinates" | "wgs84" | "msk" => Mode::Coordinates,
        _ => Mode::Standard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> LaunchOptions {
        LaunchOptions::parse(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn parses_legacy_aliases_and_preserves_screenshot_paths() {
        assert_eq!(
            parse(&[
                "--ignored",
                "--screenshot",
                "folder with spaces/date.png",
                "--mode",
                "MSK",
                "--lang",
                "English",
                "--theme",
                "LIGHT",
            ]),
            LaunchOptions {
                mode: Mode::Coordinates,
                lang: Lang::En,
                theme: ThemeMode::White,
                screenshot: Some("folder with spaces/date.png".into()),
            },
        );
    }

    #[test]
    fn defaults_and_last_option_keep_the_existing_cli_contract() {
        assert_eq!(
            parse(&["--mode", "unrecognized", "--lang", "ru", "--theme"]),
            parse(&[]),
        );
        let opts = parse(&["--mode", "sci", "--mode", "prog", "--screenshot"]);
        assert_eq!(opts.mode, Mode::Programmer);
        assert_eq!(opts.screenshot, None);
        assert!(ThemeMode::System.is_dark(true));
        assert!(!ThemeMode::System.is_dark(false));
    }

    #[test]
    fn mode_shortcuts_ignore_case_but_not_unrecognized_keys() {
        assert_eq!(mode_shortcut(b'u'), Some(Mode::UnitConversion));
        assert_eq!(mode_shortcut(b'e'), Some(Mode::DateCalculation));
        assert_eq!(mode_shortcut(b'G'), Some(Mode::Coordinates));
        assert_eq!(mode_shortcut(b'C'), None);
    }

    #[test]
    fn every_mode_has_a_stable_slot_and_a_label_in_both_locales() {
        for (slot, mode) in MODES.into_iter().enumerate() {
            assert_eq!(mode_index(mode), slot);
            assert!(!mode_title(mode, Lang::Ru).is_empty());
            assert!(!mode_title(mode, Lang::En).is_empty());
        }
    }
}
