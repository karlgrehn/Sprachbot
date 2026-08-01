//! Der Kern von Iris: ein lokaler HTTP-Dienst. Alle Logik (RAM-Erkennung,
//! Ollama-Anbindung, Aktions-Registry) lebt ausschließlich hinter dieser
//! Schnittstelle. Die Tauri-Oberfläche ist nur ein Client davon (siehe
//! docs/PLAN.md, Abschnitt 7) — kein Tauri-`invoke`, keine Geschäftslogik
//! im Frontend.

use axum::{
    extract::{Json as JsonExtract, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::{hardware, ollama, registry};

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

#[derive(Serialize)]
struct RegistryResponse {
    actions: &'static [registry::Action],
}

async fn registry_handler() -> Json<RegistryResponse> {
    Json(RegistryResponse {
        actions: registry::REGISTRY,
    })
}

#[derive(Deserialize)]
struct AskRequest {
    model: String,
    prompt: String,
}

#[derive(Serialize)]
struct AskResponse {
    response: String,
    /// true, wenn die Antwort direkt aus der Registry kommt (keine
    /// Modellanfrage) — der Nutzer soll immer sehen, ob Iris gerade
    /// gehandelt hat oder nur geantwortet hat.
    command_handled: bool,
}

async fn ask_handler(
    State(state): State<registry::SharedState>,
    JsonExtract(req): JsonExtract<AskRequest>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    if let Some(notice) = registry::handle_command(&state, &req.prompt) {
        return Ok(Json(AskResponse {
            response: notice,
            command_handled: true,
        }));
    }

    let instruction = registry::current_instruction(&state);
    let full_prompt = format!("{instruction}\n\n{}", req.prompt);

    ollama::generate(&req.model, &full_prompt)
        .await
        .map(|response| Json(AskResponse {
            response,
            command_handled: false,
        }))
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

fn router(state: registry::SharedState) -> Router {
    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/models", get(models_handler))
        .route("/api/registry", get(registry_handler))
        .route("/api/ask", post(ask_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Startet den Kern und blockiert, bis er beendet wird. Bindet nur an
/// 127.0.0.1 — in M1 ist Iris ausschließlich lokal erreichbar.
pub async fn serve() {
    let state = registry::new_state();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("Iris-Kern konnte Port nicht binden");
    axum::serve(listener, router(state))
        .await
        .expect("Iris-Kern abgestürzt");
}
