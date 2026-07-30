mod hardware;
mod ollama;

use hardware::ModelRecommendation;

#[tauri::command]
fn recommend_model() -> ModelRecommendation {
    hardware::recommend_model()
}

#[tauri::command]
async fn ollama_status() -> bool {
    ollama::is_available().await
}

#[tauri::command]
async fn list_models() -> Result<Vec<String>, String> {
    ollama::list_installed_models().await
}

#[tauri::command]
async fn ask(model: String, prompt: String) -> Result<String, String> {
    ollama::generate(&model, &prompt).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            recommend_model,
            ollama_status,
            list_models,
            ask
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
