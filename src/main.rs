#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

fn main() {
    library::screensaver_runtime::run_main(
        library::role::application::scenes::flame::Flame::new(),
        "flame",
    );
}
