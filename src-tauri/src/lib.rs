mod api;
mod client;

use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub com_channel: Mutex<client::hooks::Channel>,
    pub api_context: Mutex<api::ApiContext>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            app.manage(AppState {
                com_channel: Mutex::new(client::hooks::init(app.handle().clone())),
                api_context: Mutex::new(api::create())
            });
            {
                let state = app.state::<AppState>();
                state.com_channel.lock().unwrap().init_chatlog(app.handle().clone());
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            api::get_servers,
            api::add_server,
            api::delete_server,
            api::remove_client,
            api::get_client,
            api::get_client_by_user,
            api::get_clients,
            api::auth::auth_validity,
            api::auth::recall_authentication,
            api::auth::auth_offline,
            api::auth::auth_ms_cache,
            api::auth::auth_ms_init,
            api::auth::auth_ms_finish,
            api::controller::create_connection,
            api::controller::connect_client,
            api::controller::disconnect_client,
            api::controller::send_chat,
            api::controller::kill_client,
            api::controller::get_instances,
            api::controller::get_available_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
