mod commands;
mod config;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::get_server_url,
            commands::set_server_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
