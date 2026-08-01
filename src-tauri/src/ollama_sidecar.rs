//! Nur in der Offline-Variante (Cargo-Feature `offline-bundle`, siehe
//! tauri.offline.conf.json): startet das mitgelieferte Ollama als Sidecar
//! mit einem vorab gebündelten Modell, statt eine separat installierte
//! Ollama-Instanz vorauszusetzen. `ollama.rs` selbst ändert sich dadurch
//! nicht — es spricht weiterhin einfach `127.0.0.1:11434` an, egal wer
//! dort lauscht.

use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;

/// Startet den Ollama-Sidecar mit `OLLAMA_MODELS` auf das gebündelte,
/// beim Bauen vorab befüllte Modellverzeichnis gesetzt. Der Kindprozess
/// wird nicht separat verwaltet — er endet mit dem App-Prozess, da er
/// dessen Prozessgruppe/Job-Objekt teilt (Standard-Sidecar-Verhalten).
pub fn spawn(app: &AppHandle) {
    let models_dir = app
        .path()
        .resolve("bundled-model", tauri::path::BaseDirectory::Resource)
        .unwrap_or_else(|_| std::path::PathBuf::from("../bundled-model"));

    let sidecar = match app.shell().sidecar("ollama") {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Ollama-Sidecar nicht gefunden: {e}");
            return;
        }
    };

    let result = sidecar
        .env("OLLAMA_MODELS", models_dir.to_string_lossy().to_string())
        .args(["serve"])
        .spawn();

    if let Err(e) = result {
        eprintln!("Ollama-Sidecar konnte nicht gestartet werden: {e}");
    }
}
