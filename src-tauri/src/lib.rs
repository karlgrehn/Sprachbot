pub mod api;
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

// Nur Teil des Builds, wenn das "desktop-gui"-Feature aktiv ist (Default,
// siehe Cargo.toml) - iris-hub baut ohne dieses Feature und braucht diese
// Funktion (und damit tauri selbst) nicht.
#[cfg(feature = "desktop-gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // tauri-plugin-shell wird immer initialisiert (siehe Cargo.toml-Kommentar
    // zu offline-bundle): nur die normale Variante gewährt ihm nie die
    // shell:allow-execute-Permission (capabilities/default.json enthält sie
    // nicht), der Sidecar wird dort auch nie gestartet.
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init());

    builder
        .setup(|_app| {
            // Nur Desktop startet den eigenen Kern (Fat Client, siehe
            // docs/PLAN.md): Android/iOS sind bewusst reine Thin Clients ohne
            // eigenen lokalen Server — sie verbinden sich stattdessen über
            // die in main.ts gemerkte Serveradresse mit einem entfernten
            // Iris-Rechner. Ein lokaler Kern auf dem Handy wäre nicht nur
            // unnötig, sondern auch funktionslos ohne Ollama/MCP dort.
            #[cfg(desktop)]
            {
                use tauri::Manager;
                let dist_dir = _app
                    .path()
                    .resolve("dist", tauri::path::BaseDirectory::Resource)
                    .unwrap_or_else(|_| std::path::PathBuf::from("../dist"));

                #[cfg(feature = "offline-bundle")]
                ollama_sidecar::spawn(_app.handle());

                tauri::async_runtime::spawn(api::serve(dist_dir));
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
