//! Tauri shell: thin IPC adapter over `rombro-core` / `rombro-store`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod import;
mod library;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(import::Pending::default())
        .invoke_handler(tauri::generate_handler![
            commands::db_stats,
            commands::scan,
            import::plan_import,
            import::execute_plan,
            import::undo_last,
            library::library_list
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("rombro: {e}");
            std::process::exit(1);
        });
}
