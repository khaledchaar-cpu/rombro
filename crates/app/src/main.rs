//! Tauri shell: thin IPC adapter over `rombro-core` / `rombro-store`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod decide;
mod import;
mod library;
mod settings;
mod thumbs;

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
            import::journal_list,
            decide::resolve_ambiguous,
            decide::set_verdict,
            decide::trash_list,
            decide::trash_empty,
            library::session_get,
            library::library_list,
            settings::rules_get,
            settings::rules_set,
            thumbs::thumbnail
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("rombro: {e}");
            std::process::exit(1);
        });
}
