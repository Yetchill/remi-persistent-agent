pub mod database;
pub mod memory;
mod pet_pack;
mod pet_state;
mod platform;
mod profile;
mod provider;
mod settings;
mod soul;
mod state;
mod trace;
mod window;
mod working_memory;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let database = database::Database::open(app_data_dir.join("remi.sqlite3"))?;
            let soul_path = app_data_dir.join("SOUL.md");
            soul::ensure_soul_exists(&soul_path)?;
            app.manage(state::AppState::new(database, soul_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::get_work_area,
            window::get_pet_position,
            window::set_pet_position,
            window::set_pet_window_size,
            window::open_chat_bubble,
            window::is_chat_bubble_visible,
            window::open_proactive_bubble,
            window::hide_current_window,
            window::sync_chat_bubble_position,
            window::open_settings_window,
            window::quit_app,
            pet_state::get_pet_state,
            pet_state::update_pet_state,
            pet_state::persist_pet_position,
            pet_pack::list_pet_packs,
            pet_pack::import_pet_pack,
            pet_pack::read_pet_pack_frame,
            pet_pack::activate_pet_pack,
            settings::get_app_settings,
            settings::update_app_settings,
            settings::get_runtime_overview,
            soul::get_soul,
            soul::update_soul,
            provider::get_provider_catalog,
            provider::save_provider,
            provider::delete_provider,
            provider::set_active_model,
            provider::complete_llm,
            profile::export_companion_profile,
            profile::preview_companion_profile,
            profile::import_companion_profile,
            trace::trace_event,
            trace::trace_action,
            memory::write_memory,
            memory::retrieve_memories,
            memory::get_relationship_summary,
            memory::get_memory_viewer,
            memory::get_memory_detail,
            memory::consolidate_memories,
            memory::archive_memory,
            memory::edit_memory,
            memory::restore_memory,
            memory::delete_memory,
            memory::pin_memory,
            working_memory::persist_message,
            working_memory::get_recent_messages,
            working_memory::clear_current_conversation,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Remi");
}
