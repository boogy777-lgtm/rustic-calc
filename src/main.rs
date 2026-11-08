#![windows_subsystem = "windows"]
#![deny(unsafe_op_in_unsafe_fn)]

mod actions;
mod app;
mod engine;
mod i18n;
mod platform;
mod tests;
mod ui;
mod units;
mod views;

use app::App;
use engine::{Mode, WorksheetKind};
use i18n::Lang;
use platform::WindowHandler;
use ui::palette::ThemeMode;

pub use units::date_parts;

fn parse_mode(text: &str) -> Mode {
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
        _ => Mode::Standard,
    }
}

fn main() {
    let mut screenshot = None;
    let mut initial_mode = Mode::Standard;
    let mut lang = Lang::default();
    let mut theme = ThemeMode::System;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if let Some(path) = args.get(i + 1) {
                    screenshot = Some(path.clone());
                }
                i += 2;
            }
            "--mode" => {
                if let Some(value) = args.get(i + 1) {
                    initial_mode = parse_mode(value);
                }
                i += 2;
            }
            "--lang" => {
                if let Some(value) = args.get(i + 1) {
                    lang = match value.to_ascii_lowercase().as_str() {
                        "en" | "english" => Lang::En,
                        _ => Lang::Ru,
                    };
                }
                i += 2;
            }
            "--theme" => {
                if let Some(value) = args.get(i + 1) {
                    theme = match value.to_ascii_lowercase().as_str() {
                        "white" | "light" => ThemeMode::White,
                        "dark" => ThemeMode::Dark,
                        _ => ThemeMode::System,
                    };
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    let app = App::new(initial_mode, screenshot, lang, theme);
    let (w, h) = app.preferred_client_size(initial_mode);
    let mut handler: Box<dyn WindowHandler> = Box::new(app);
    platform::run(&mut handler, lang.text().app_title, w, h);
}
