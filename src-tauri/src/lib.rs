pub mod commands;
pub mod storage;

use std::sync::Mutex;
use storage::Vault;

pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            vault: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::create_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::list_entries,
            commands::search_entries,
            commands::create_entry,
            commands::read_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::get_backlinks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
