//! Tauri shell: thin IPC adapter over `rombro-core` / `rombro-store`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::db_stats, commands::scan])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("rombro: {e}");
            std::process::exit(1);
        });
}
