mod api;
mod clock;
mod command;
mod hardware;
mod http_client;
mod mcp;
mod message;
mod oauth;
mod ollama;
mod permissions;
mod postfach;
mod registry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            tauri::async_runtime::spawn(api::serve());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
