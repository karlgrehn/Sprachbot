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
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::command::{CommandResult, PermissionRequest};
use crate::{hardware, mcp, message, ollama, permissions, postfach, registry};

/// Muss mit `API_BASE` in src/main.ts übereinstimmen.
pub const PORT: u16 = 47615;

#[derive(Clone)]
struct AppState {
    verbosity: registry::SharedState,
    permissions: permissions::SharedPermissions,
    postfach: postfach::SharedPostfach,
    mcp: mcp::SharedMcp,
}

/// Downstream-Dienst (Ollama, MCP-Server) nicht erreichbar oder fehlerhaft
/// — derselbe Antwort-Shape an jeder Stelle, die einen davon aufruft.
fn bad_gateway(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, e.to_string())
}

#[derive(Serialize)]
struct StatusResponse {
    ram_gb: f64,
    tier: &'static str,
    recommended_model: &'static str,
    ollama_available: bool,
    installed_models: Vec<String>,
}

async fn status_handler() -> Json<StatusResponse> {
    let recommendation = hardware::recommend_model();
    let models_result = ollama::list_installed_models().await;
    let ollama_available = models_result.is_ok();
    Json(StatusResponse {
        ram_gb: recommendation.ram_gb,
        tier: recommendation.tier,
        recommended_model: recommendation.model,
        ollama_available,
        installed_models: models_result.unwrap_or_default(),
    })
}

async fn models_handler() -> Result<Json<Vec<String>>, (StatusCode, String)> {
    ollama::list_installed_models().await.map(Json).map_err(bad_gateway)
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
    if granted.id == permissions::MCP_SERVER_VERBINDEN {
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
        .map_err(bad_gateway)
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
        let zusammenfassung = ollama::generate(&req.model, &prompt).await.map_err(bad_gateway)?;
        postfach::ablegen(&state.postfach, chat.clone(), zusammenfassung);
        zusammengefasste_chats.push(chat);
    }

    Ok(Json(NachrichtenResponse {
        zusammengefasste_chats,
    }))
}

/// "/help": zeigt, was möglich ist und was erteilt/verbunden ist —
/// erzeugt aus den Registries, nicht aus dem Modellgedächtnis (docs/PLAN.md,
/// Abschnitt 2). Läuft über denselben `/api/ask`-Weg wie jeder andere
/// Befehl, damit es später auch über einen Messenger-Kanal (M4) funktioniert.
fn help_command(state: &AppState, prompt: &str) -> Option<CommandResult> {
    if prompt.trim().to_lowercase() != "/help" {
        return None;
    }

    let active: std::collections::HashSet<String> = permissions::list_active(&state.permissions)
        .into_iter()
        .map(|g| g.id)
        .collect();
    let servers = mcp::list_servers(&state.mcp);

    let mut zeilen = vec!["Was Iris gerade kann (aus der Registry, nicht vom Modell erfunden):".to_string(), String::new()];
    zeilen.extend(registry::REGISTRY.iter().map(|a| {
        format!("- {}{}", a.anzeigetext, if a.reversible { " (rückgängig machbar)" } else { "" })
    }));
    zeilen.push(String::new());
    zeilen.push("Berechtigungen:".to_string());
    zeilen.extend(permissions::PERMISSIONS.iter().map(|p| {
        format!("- [{}] {}", if active.contains(p.id) { "erteilt" } else { "nicht erteilt" }, p.anzeigetext)
    }));
    zeilen.push(String::new());
    zeilen.push("Verbundene MCP-Server:".to_string());
    if servers.is_empty() {
        zeilen.push("- keine".to_string());
    } else {
        zeilen.extend(servers.iter().map(|s| {
            let namen: Vec<&str> = s.tools.iter().map(|t| t.name.as_str()).collect();
            format!("- {}: {}", s.url, if namen.is_empty() { "keine Werkzeuge".to_string() } else { namen.join(", ") })
        }));
    }
    zeilen.push(String::new());
    zeilen.push(
        "Sag Iris einfach, was du willst — z. B. \"antworte ab jetzt kurz\", \"mach das rückgängig\", \
         \"verbinde mein WhatsApp\" oder \"verbinde mich mit https://beispiel.de/mcp\"."
            .to_string(),
    );

    Some(CommandResult::message(zeilen.join("\n")))
}

#[derive(Deserialize)]
struct AskRequest {
    model: String,
    prompt: String,
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

impl From<CommandResult> for AskResponse {
    fn from(result: CommandResult) -> Self {
        AskResponse {
            response: result.message,
            command_handled: true,
            permission_request: result.permission_request,
        }
    }
}

async fn ask_handler(
    State(state): State<AppState>,
    JsonExtract(req): JsonExtract<AskRequest>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    if let Some(result) = help_command(&state, &req.prompt) {
        return Ok(Json(result.into()));
    }
    if let Some(result) = registry::handle_command(&state.verbosity, &req.prompt) {
        return Ok(Json(result.into()));
    }
    if let Some(result) = permissions::handle_command(&state.permissions, &req.prompt) {
        return Ok(Json(result.into()));
    }
    if let Some(result) = mcp::handle_command(&state.mcp, &state.permissions, &req.prompt).await {
        return Ok(Json(result.into()));
    }
    if let Some(result) = postfach::handle_command(&state.postfach, &req.prompt) {
        return Ok(Json(result.into()));
    }

    let instruction = registry::current_instruction(&state.verbosity);
    let full_prompt = format!("{instruction}\n\n{}", req.prompt);

    ollama::generate(&req.model, &full_prompt)
        .await
        .map(|response| {
            Json(AskResponse {
                response,
                command_handled: false,
                permission_request: None,
            })
        })
        .map_err(bad_gateway)
}

/// `dist_dir`: die gebaute Oberfläche (Vite-Output). Wird als Fallback nach
/// allen `/api/*`-Routen ausgeliefert — dasselbe HTML/JS/CSS, das auch das
/// Tauri-Fenster zeigt, jetzt zusätzlich über HTTP erreichbar. Das ist die
/// Voraussetzung für Thin Clients (docs/PLAN.md, Abschnitt 3/4): ein Handy,
/// das über Tailscale denselben Port anspricht, bekommt Oberfläche und
/// Kern aus einer Quelle, ohne dass sich am Frontend-Code etwas ändert.
/// Existiert `dist_dir` nicht (z. B. während `tauri dev` ohne vorherigen
/// `npm run build`), liefert der Fallback schlicht 404 — das Tauri-Fenster
/// selbst ist davon nicht betroffen, es lädt sein Frontend unabhängig davon.
fn router(state: AppState, dist_dir: PathBuf) -> Router {
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
        .fallback_service(ServeDir::new(dist_dir))
}

/// Startet den Kern und blockiert, bis er beendet wird. Bindet nur an
/// 127.0.0.1 — Fernzugriff (Thin Client) läuft über `tailscale serve`,
/// nicht darüber, dass Iris selbst auf allen Netzwerkschnittstellen
/// lauscht. Es gibt keine eigene Authentifizierung; wer die Adresse im
/// Tailnet erreicht, kann den Kern benutzen (siehe README).
pub async fn serve(dist_dir: PathBuf) {
    let state = AppState {
        verbosity: registry::new_state(),
        permissions: permissions::new_state(),
        postfach: postfach::new_state(),
        mcp: mcp::new_state(),
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("Iris-Kern konnte Port nicht binden");
    axum::serve(listener, router(state, dist_dir))
        .await
        .expect("Iris-Kern abgestürzt");
}
