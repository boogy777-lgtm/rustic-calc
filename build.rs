use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc() -> Option<PathBuf> {
    if let Ok(path) = env::var("RC") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(path) = which("rc.exe") {
        return Some(path);
    }
    let kits = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(kits)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    for version in versions.iter().rev() {
        let rc = version.join("x64").join("rc.exe");
        if rc.is_file() {
            return Some(rc);
        }
    }
    None
}

fn which(name: &str) -> Result<PathBuf, ()> {
    let paths = env::var_os("PATH").ok_or(())?;
    for dir in env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(())
}

fn main() {
    println!("cargo:rerun-if-changed=assets/calc.rc");
    println!("cargo:rerun-if-changed=assets/calc.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-env-changed=RC");

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let resource = out.join("calc1.res");
    let version = env::var("CARGO_PKG_VERSION").expect("package version");
    let parts: Vec<u16> = version
        .split('.')
        .map(|part| part.parse().expect("numeric release version"))
        .collect();
    assert_eq!(
        parts.len(),
        3,
        "release version must have three numeric components"
    );
    let arch = match env::var("CARGO_CFG_TARGET_ARCH")
        .expect("target architecture")
        .as_str()
    {
        "x86_64" => "amd64",
        "x86" => "x86",
        "aarch64" => "arm64",
        other => panic!("unsupported Windows architecture: {other}"),
    };
    let manifest = std::fs::read_to_string("assets/app.manifest")
        .expect("read manifest template")
        .replace("@APP_VERSION@", &format!("{version}.0"))
        .replace("@APP_ARCH@", arch);
    let manifest_path = out.join("rustic-app.manifest");
    std::fs::write(&manifest_path, manifest).expect("write manifest");
    std::fs::write(out.join("version.h"), format!(
        "#define RUSTIC_VERSION_QUAD {},{},{},0\n#define RUSTIC_VERSION_STRING \"{}\"\n#define RUSTIC_MANIFEST_PATH \"{}\"\n",
        parts[0], parts[1], parts[2], version, manifest_path.to_string_lossy().replace('\\', "\\\\"))).expect("write version resource header");
    let Some(rc) = find_rc() else {
        panic!("rc.exe is required; install the Windows SDK or set RC to its path");
    };

    let status = Command::new(&rc)
        .current_dir("assets")
        .arg("/nologo")
        .arg("/c65001")
        .arg("/i")
        .arg(&out)
        .arg("/fo")
        .arg(&resource)
        .arg("calc.rc")
        .status()
        .expect("failed to run resource compiler");
    if !status.success() {
        panic!("resource compiler failed with {status}");
    }
    println!("cargo:rustc-link-arg={}", resource.display());
}
