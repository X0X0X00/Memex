pub mod schema;
pub mod error;
pub mod db;
pub mod parser;
pub mod stats;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running memex");
}
