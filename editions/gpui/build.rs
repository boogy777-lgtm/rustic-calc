#[allow(dead_code)]
#[path = "../../build.rs"]
mod winres;

use std::path::Path;

fn main() {
    winres::compile_resources_with_common_controls(
        Path::new("../../assets"),
        "Rustic Calc (GPUI)",
        "RusticCalc-GPUI.exe",
    );
}
