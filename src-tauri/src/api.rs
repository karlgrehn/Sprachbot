//! Der Kern von Iris: ein lokaler HTTP-Dienst. Alle Logik (RAM-Erkennung,
//! Ollama-Anbindung, Aktions-Registry, Berechtigungen) lebt ausschließlich
//! hinter dieser Schnittstelle. Die Tauri-Oberfläche ist nur ein Client
//! davon (siehe docs/PLAN.md, Abschnitt 7) — kein Tauri-`invoke`, keine
//! Geschäftslogik im Frontend.

use axum::{
    extract::{Json as JsonExtract, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_http::cors::CorsLayer;

use crate::{hardware, mcp, message, ollama, permissions, postfach, registry};

/// Muss mit `API_BASE` in src/main.ts übereinstimmen.
pub const PORT: u16 = 47615;

const MCP_PERMISSION_ID: &str = "mcp.server.verbinden";

#[derive(Clone)]
struct AppState {
    verbosity: registry::SharedState,
    permissions: permissions::SharedPermissions,
    postfach: postfach::SharedPostfach,
    mcp: mcp::SharedMcp,
}

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

#[derive(Serialize)]
struct PermissionsResponse {
    definitions: &'static [permissions::PermissionDef],
    active: Vec<permissions::GrantedPermission>,
}

async fn permissions_handler(State(state): State<AppState>) -> Json<PermissionsResponse> {
    Json(PermissionsResponse {
        definitions: permissions::PERMISSIONS,
        active: permissions::list_active(&state.permissions),
    })
}

#[derive(Deserialize)]
struct GrantRequest {
    id: String,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Serialize)]
struct GrantResponse {
    id: String,
    scope: Option<String>,
    granted_at_unix: u64,
    /// gesetzt, wenn das Erteilen sofort eine MCP-Verbindung ausgelöst hat
    /// ("ab jetzt läuft es ohne Rückfrage", docs/PLAN.md Abschnitt 2).
    tools: Option<Vec<String>>,
    connect_error: Option<String>,
}

async fn grant_handler(
    State(state): State<AppState>,
    JsonExtract(req): JsonExtract<GrantRequest>,
) -> Result<Json<GrantResponse>, (StatusCode, String)> {
    let granted = permissions::grant(&state.permissions, &req.id, req.scope.clone())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let mut tools = None;
    let mut connect_error = None;
    if granted.id == MCP_PERMISSION_ID {
        if let Some(url) = &granted.scope {
            match mcp::connect(&state.mcp, url).await {
                Ok(t) => tools = Some(t.into_iter().map(|tool| tool.name).collect()),
                Err(e) => connect_error = Some(e),
            }
        }
    }

    Ok(Json(GrantResponse {
        id: granted.id,
        scope: granted.scope,
        granted_at_unix: granted.granted_at_unix,
        tools,
        connect_error,
    }))
}

#[derive(Serialize)]
struct McpServersResponse {
    servers: Vec<mcp::McpServerInfo>,
}

async fn mcp_servers_handler(State(state): State<AppState>) -> Json<McpServersResponse> {
    Json(McpServersResponse {
        servers: mcp::list_servers(&state.mcp),
    })
}

#[derive(Deserialize)]
struct ToolCallRequest {
    url: String,
    tool: String,
    #[serde(default)]
    arguments: Value,
}

async fn mcp_tool_call_handler(
    State(state): State<AppState>,
    JsonExtract(req): JsonExtract<ToolCallRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    mcp::call_tool(&state.mcp, &req.url, &req.tool, req.arguments)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

#[derive(Serialize)]
struct PostfachResponse {
    eintraege: Vec<postfach::PostfachEintrag>,
}

async fn postfach_handler(State(state): State<AppState>) -> Json<PostfachResponse> {
    Json(PostfachResponse {
        eintraege: postfach::liste(&state.postfach),
    })
}

/// Nimmt Nachrichten entgegen, gruppiert nach Chat und fasst jede Gruppe
/// lokal zusammen. Das ist die Schnittstelle, die später die Conduit/
/// mautrix-whatsapp-Bridge bedient (M2) — bis dahin lässt sie sich auch
/// von Hand befüllen, um die Zusammenfassungs-Pipeline zu prüfen.
#[derive(Deserialize)]
struct NachrichtenRequest {
    model: String,
    nachrichten: Vec<message::Nachricht>,
}

#[derive(Serialize)]
struct NachrichtenResponse {
    zusammengefasste_chats: Vec<String>,
}

async fn nachrichten_handler(
    State(state): State<AppState>,
    JsonExtract(req): JsonExtract<NachrichtenRequest>,
) -> Result<Json<NachrichtenResponse>, (StatusCode, String)> {
    let gruppen = message::gruppiere_nach_chat(&req.nachrichten);
    let mut zusammengefasste_chats = Vec::new();

    for (chat, nachrichten) in gruppen {
        let prompt = message::zusammenfassen_prompt(&chat, &nachrichten);
        let zusammenfassung = ollama::generate(&req.model, &prompt)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
        postfach::ablegen(&state.postfach, chat.clone(), zusammenfassung);
        zusammengefasste_chats.push(chat);
    }

    Ok(Json(NachrichtenResponse {
        zusammengefasste_chats,
    }))
}

#[derive(Deserialize)]
struct AskRequest {
    model: String,
    prompt: String,
}

#[derive(Serialize)]
struct PermissionRequest {
    id: String,
    anzeigetext: String,
    scope: Option<String>,
}

#[derive(Serialize)]
struct AskResponse {
    response: String,
    /// true, wenn die Antwort direkt aus der Registry/Berechtigungslogik
    /// kommt (keine Modellanfrage) — der Nutzer soll immer sehen, ob Iris
    /// gerade gehandelt hat oder nur geantwortet hat.
    command_handled: bool,
    /// gesetzt, wenn der Prompt eine Aktion anfordert, für die eine
    /// Berechtigung fehlt. Das Frontend zeigt dann einen echten
    /// Erlauben-Button, der /api/permissions/grant aufruft.
    permission_request: Option<PermissionRequest>,
}

async fn ask_handler(
    State(state): State<AppState>,
    JsonExtract(req): JsonExtract<AskRequest>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    if let Some(notice) = registry::handle_command(&state.verbosity, &req.prompt) {
        return Ok(Json(AskResponse {
            response: notice,
            command_handled: true,
            permission_request: None,
        }));
    }

    if let Some(result) = permissions::handle_command(&state.permissions, &req.prompt) {
        return Ok(Json(AskResponse {
            response: result.message,
            command_handled: true,
            permission_request: result.permission_request.map(|req| PermissionRequest {
                id: req.id.to_string(),
                anzeigetext: req.anzeigetext.to_string(),
                scope: req.scope,
            }),
        }));
    }

    if let Some(url) = mcp::find_server_url(&req.prompt) {
        let lower = req.prompt.to_lowercase();
        if lower.contains("verbind") || lower.contains("mcp") || lower.contains("nutz") {
            if permissions::is_granted(&state.permissions, MCP_PERMISSION_ID, Some(&url)) {
                let message = match mcp::list_servers(&state.mcp)
                    .into_iter()
                    .find(|s| s.url == url)
                {
                    Some(server) => {
                        let names: Vec<&str> =
                            server.tools.iter().map(|t| t.name.as_str()).collect();
                        format!("Bereits verbunden mit {url}. Werkzeuge: {}", names.join(", "))
                    }
                    None => match mcp::connect(&state.mcp, &url).await {
                        Ok(tools) => {
                            let names: Vec<String> = tools.into_iter().map(|t| t.name).collect();
                            format!("Verbunden mit {url}. Werkzeuge: {}", names.join(", "))
                        }
                        Err(e) => format!("Verbindung zu {url} fehlgeschlagen: {e}"),
                    },
                };
                return Ok(Json(AskResponse {
                    response: message,
                    command_handled: true,
                    permission_request: None,
                }));
            }

            let def = permissions::def_for(MCP_PERMISSION_ID)
                .expect("mcp.server.verbinden muss in PERMISSIONS stehen");
            return Ok(Json(AskResponse {
                response: format!("[BERECHTIGUNG ANGEFRAGT]\n{}\nServer: {url}", def.anzeigetext),
                command_handled: true,
                permission_request: Some(PermissionRequest {
                    id: def.id.to_string(),
                    anzeigetext: def.anzeigetext.to_string(),
                    scope: Some(url),
                }),
            }));
        }
    }

    if let Some(antwort) = postfach::handle_command(&state.postfach, &req.prompt) {
        return Ok(Json(AskResponse {
            response: antwort,
            command_handled: true,
            permission_request: None,
        }));
    }

    let instruction = registry::current_instruction(&state.verbosity);
    let full_prompt = format!("{instruction}\n\n{}", req.prompt);

    ollama::generate(&req.model, &full_prompt)
        .await
        .map(|response| Json(AskResponse {
            response,
            command_handled: false,
            permission_request: None,
        }))
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/models", get(models_handler))
        .route("/api/registry", get(registry_handler))
        .route("/api/permissions", get(permissions_handler))
        .route("/api/permissions/grant", post(grant_handler))
        .route("/api/postfach", get(postfach_handler))
        .route("/api/nachrichten", post(nachrichten_handler))
        .route("/api/mcp/servers", get(mcp_servers_handler))
        .route("/api/mcp/tools/call", post(mcp_tool_call_handler))
        .route("/api/ask", post(ask_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Startet den Kern und blockiert, bis er beendet wird. Bindet nur an
/// 127.0.0.1 — in M1/M2 ist Iris ausschließlich lokal erreichbar.
pub async fn serve() {
    let state = AppState {
        verbosity: registry::new_state(),
        permissions: permissions::new_state(),
        postfach: postfach::new_state(),
        mcp: mcp::new_state(),
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("Iris-Kern konnte Port nicht binden");
    axum::serve(listener, router(state))
        .await
        .expect("Iris-Kern abgestürzt");
}
