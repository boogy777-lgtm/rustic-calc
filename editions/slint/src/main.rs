#![windows_subsystem = "windows"]

mod date;
mod png;
mod slint_app;

slint::include_modules!();

fn main() {
    let options = rustic_calc_app::startup::LaunchOptions::parse(std::env::args().skip(1));
    let screenshot = options.screenshot.clone();
    if let Err(error) = slint_app::run(options) {
        if let Some(path) = screenshot {
            let _ = std::fs::write(format!("{path}.error"), error.to_string());
        } else {
            eprintln!("{error}");
        }
        std::process::exit(1);
    }
}
