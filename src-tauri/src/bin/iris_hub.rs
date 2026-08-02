//! Kopfloser Hub: derselbe Kern wie in der Tauri-App (`api::serve`), aber
//! ganz ohne Fenster/Webview. Ein Raspberry Pi läuft meist ohne Bildschirm
//! (Raspberry Pi OS Lite) — eine Tauri-GUI-App würde dort mangels
//! Fenstersystem gar nicht erst starten. Dieses Binary schon: es ist reines
//! Axum/Tokio, läuft also genauso auf der Kommandozeile unter Windows (zum
//! Testen, bevor ein Pi angeschafft ist) wie später auf dem Pi selbst
//! (aarch64-unknown-linux-gnu-Build).
//!
//! Erwartet eine separat installierte Ollama-Instanz (wie die normale,
//! nicht-Offline-Variante der App) — das Offline-Bundle mit mitgeliefertem
//! Modell existiert nur für x86_64.
use std::path::PathBuf;

fn dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("IRIS_DIST_DIR") {
        return PathBuf::from(p);
    }
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("dist")));
    match beside_exe {
        Some(p) if p.exists() => p,
        _ => PathBuf::from("../dist"),
    }
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Tokio-Runtime konnte nicht gestartet werden");

    println!("Iris-Hub läuft auf Port {}. Strg+C zum Beenden.", iris_app_lib::api::PORT);
    println!("Fernzugriff (Handy o.ä.) über Tailscale, siehe README \"Thin Client einrichten\".");

    rt.block_on(iris_app_lib::api::serve(dist_dir()));
}
