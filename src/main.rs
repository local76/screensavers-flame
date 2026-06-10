#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod runner;
mod flame;

fn main() {
    let effect = flame::Flame::new();
    runner::run_main(effect, "flame");
}