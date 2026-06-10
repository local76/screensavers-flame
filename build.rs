use std::path::Path;

fn main() {
    // Note: this still has the rc.exe 10.0+ ICONDIR corruption bug.
    // The migration to embed-resource 2.x (see library::build_resources) is
    // queued for when a workaround for the SDK bug is found. Until then,
    // winres produces the same broken output as the original pre-migration
    // build, so we restore it to keep the toolchain working.
    println!("cargo:rerun-if-changed=assets/icon.ico");
    let ico_path = Path::new("assets/icon.ico");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" && ico_path.exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("failed to compile winres resource");
    }
}