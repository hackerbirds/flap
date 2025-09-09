// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::filter::LevelFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .with_line_number(true)
        .with_target(false)
        .init();

    flap_app_lib::run()
}
