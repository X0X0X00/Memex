pub mod schema;
pub mod error;
pub mod db;
pub mod parser;
pub mod stats;
pub mod commands;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("memex.db"))?;
            app.manage(commands::AppState { db: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_export,
            commands::import_claude_code,
            commands::list_conversations,
            commands::get_conversation,
            commands::get_stats,
            commands::clear_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running memex");
}
