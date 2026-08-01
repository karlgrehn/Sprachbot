mod api;
mod clock;
mod command;
mod hardware;
mod http_client;
mod mcp;
mod message;
mod oauth;
mod ollama;
#[cfg(feature = "offline-bundle")]
mod ollama_sidecar;
mod permissions;
mod postfach;
mod registry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(feature = "offline-bundle")]
    let builder = builder.plugin(tauri_plugin_shell::init());

    builder
        .setup(|app| {
            // Gebündelt (Release-Build): "dist" liegt im Resource-Verzeichnis
            // (siehe tauri.conf.json "bundle.resources"). Im Entwicklungsbetrieb
            // gibt es das noch nicht zwangsläufig — dann greift der relative
            // Pfad, sobald einmal `npm run build` gelaufen ist.
            use tauri::Manager;
            let dist_dir = app
                .path()
                .resolve("dist", tauri::path::BaseDirectory::Resource)
                .unwrap_or_else(|_| std::path::PathBuf::from("../dist"));

            #[cfg(feature = "offline-bundle")]
            ollama_sidecar::spawn(app.handle());

            tauri::async_runtime::spawn(api::serve(dist_dir));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
