use std::path::Path;

#[allow(dead_code)]
#[path = "../../build.rs"]
mod winres;

fn main() {
    println!("cargo:rerun-if-changed=ui/app-window.slint");
    slint_build::compile("ui/app-window.slint").expect("compile Slint UI");

    winres::compile_resources(
        Path::new("../../assets"),
        "Rustic Calc (Slint)",
        "RusticCalc-Slint.exe",
    );
}
