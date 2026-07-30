//! Der Kern von Iris: ein lokaler HTTP-Dienst. Alle Logik (RAM-Erkennung,
//! Ollama-Anbindung) lebt ausschließlich hinter dieser Schnittstelle. Die
//! Tauri-Oberfläche ist nur ein Client davon (siehe docs/PLAN.md, Abschnitt 3)
//! — kein Tauri-`invoke`, keine Geschäftslogik im Frontend.

use axum::{
    extract::Json as JsonExtract,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::{hardware, ollama};

/// Muss mit `API_BASE` in src/main.ts übereinstimmen.
pub const PORT: u16 = 47615;

#[derive(Serialize)]
struct StatusResponse {
    ram_gb: f64,
    tier: &'static str,
    recommended_model: &'static str,
    ollama_available: bool,
}

async fn status_handler() -> Json<StatusResponse> {
    let recommendation = hardware::recommend_model();
    let ollama_available = ollama::is_available().await;
    Json(StatusResponse {
        ram_gb: recommendation.ram_gb,
        tier: recommendation.tier,
        recommended_model: recommendation.model,
        ollama_available,
    })
}

async fn models_handler() -> Result<Json<Vec<String>>, (StatusCode, String)> {
    ollama::list_installed_models()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

#[derive(Deserialize)]
struct AskRequest {
    model: String,
    prompt: String,
}

#[derive(Serialize)]
struct AskResponse {
    response: String,
}

async fn ask_handler(
    JsonExtract(req): JsonExtract<AskRequest>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    ollama::generate(&req.model, &req.prompt)
        .await
        .map(|response| Json(AskResponse { response }))
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

fn router() -> Router {
    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/models", get(models_handler))
        .route("/api/ask", post(ask_handler))
        .layer(CorsLayer::permissive())
}

/// Startet den Kern und blockiert, bis er beendet wird. Bindet nur an
/// 127.0.0.1 — in M1 ist Iris ausschließlich lokal erreichbar.
pub async fn serve() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("Iris-Kern konnte Port nicht binden");
    axum::serve(listener, router())
        .await
        .expect("Iris-Kern abgestürzt");
}
