//! Tauri shell: thin IPC adapter over `rombro-core` / `rombro-store`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod decide;
mod exceptions;
mod gamify;
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
            commands::db_sync,
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
            settings::rules_defaults,
            settings::rules_catalog,
            exceptions::exceptions_get,
            exceptions::ignore_set,
            exceptions::resolution_clear,
            thumbs::thumbnail,
            thumbs::thumbs_online_get,
            thumbs::thumbs_online_set,
            gamify::gamify_stats,
            gamify::gamify_enabled_get,
            gamify::gamify_enabled_set
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("rombro: {e}");
            std::process::exit(1);
        });
}
