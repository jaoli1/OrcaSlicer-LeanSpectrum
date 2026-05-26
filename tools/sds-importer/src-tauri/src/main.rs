// Tauri entry point. Real logic lives in the library crate.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

fn main() {
    leanspectrum_sds_importer_lib::run();
}
