#![windows_subsystem = "windows"]
#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod platform;
mod tests;
mod ui;
mod views;

use app::App;
use platform::WindowHandler;
use rustic_calc_app::startup::LaunchOptions;
use rustic_calc_app::{actions, coordinates, engine, i18n, units};

pub use units::date_parts;

fn main() {
    let options = LaunchOptions::parse(std::env::args().skip(1));
    let app = App::new(
        options.mode,
        options.screenshot,
        options.lang,
        options.theme,
    );
    let (w, h) = app.preferred_client_size(options.mode);
    let mut handler: Box<dyn WindowHandler> = Box::new(app);
    platform::run(&mut handler, options.lang.text().app_title, w, h);
}
