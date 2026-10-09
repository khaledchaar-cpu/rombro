//! Tauri shell: thin IPC adapter over `romburak-core` / `romburak-store`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cheevos;
mod commands;
mod decide;
mod exceptions;
mod gamify;
mod import;
mod library;
mod live;
mod managed_ra;
mod play;
mod ra_video;
mod retroarch;
mod settings;
mod states;
mod thumbs;

fn main() {
    if let Err(e) = romburak_core::paths::legacy::migrate() {
        eprintln!("warning: moving data from the old app name failed: {e}");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("thumb", |_ctx, req, res| {
            thumbs::protocol(req, res)
        })
        .manage(import::Pending::default())
        .manage(import::Leftovers::default())
        .invoke_handler(tauri::generate_handler![
            commands::db_stats,
            commands::db_sync,
            commands::scan,
            import::plan_import,
            import::execute_plan,
            import::inbox_clear,
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
            states::save_states,
            states::delete_save_state,
            thumbs::thumbs_online_get,
            thumbs::thumbs_online_set,
            cheevos::cheevos_status,
            cheevos::cheevos_set_key,
            cheevos::cheevos_login,
            cheevos::cheevos_logout,
            cheevos::cheevos_set_hardcore,
            cheevos::cheevos_sync,
            cheevos::cheevos_progress,
            cheevos::cheevos_hash,
            cheevos::cheevos_achievements,
            cheevos::cheevos_players,
            cheevos::cheevos_players_cancel,
            managed_ra::ra_status,
            managed_ra::ra_install,
            managed_ra::ra_display,
            ra_video::ra_video,
            ra_video::ra_shaders,
            ra_video::ra_set_shader,
            live::ra_running,
            ra_video::ra_set_aspect,
            managed_ra::ra_set_display,
            play::game_cores,
            play::set_game_core,
            play::play,
            play::set_favorite,
            play::play_sessions,
            retroarch::retroarch_cores,
            gamify::gamify_stats,
            gamify::gamify_enabled_get,
            gamify::gamify_enabled_set
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("romburak: {e}");
            std::process::exit(1);
        });
}
