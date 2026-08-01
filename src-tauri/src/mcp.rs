//! MCP-Client (Model Context Protocol) über den "Streamable HTTP"-Transport.
//! Einen Server hinzuzufügen ist eine rote Aktion (`mcp.server.verbinden`,
//! siehe permissions.rs) — kein Einstellungsmenü, keine Konfigurationsdatei.
//! *Fertig, wenn ein fremder MCP-Server ohne Codeänderung eingebunden und
//! benutzt werden kann* (docs/PLAN.md, M3).

use crate::oauth;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Serialize, Clone, Debug)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Serialize, Clone, Debug)]
pub struct McpServerInfo {
    pub url: String,
    pub tools: Vec<McpTool>,
}

struct ConnectedServer {
    session_id: Option<String>,
    access_token: Option<String>,
    tools: Vec<McpTool>,
}

#[derive(Default)]
pub struct McpState {
    servers: HashMap<String, ConnectedServer>,
}

pub type SharedMcp = Arc<Mutex<McpState>>;

pub fn new_state() -> SharedMcp {
    Arc::new(Mutex::new(McpState::default()))
}

pub fn list_servers(state: &SharedMcp) -> Vec<McpServerInfo> {
    state
        .lock()
        .unwrap()
        .servers
        .iter()
        .map(|(url, s)| McpServerInfo {
            url: url.clone(),
            tools: s.tools.clone(),
        })
        .collect()
}

struct RpcResult {
    status: StatusCode,
    session_id: Option<String>,
    www_authenticate: Option<String>,
    body: Value,
}

async fn rpc_call(
    client: &reqwest::Client,
    url: &str,
    session_id: Option<&str>,
    access_token: Option<&str>,
    method: &str,
    params: Value,
    id: Option<u64>,
) -> Result<RpcResult, String> {
    let body = match id {
        Some(id) => json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        None => json!({ "jsonrpc": "2.0", "method": method, "params": params }),
    };

    let mut req = client
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .json(&body);
    if let Some(sid) = session_id {
        req = req.header("Mcp-Session-Id", sid);
    }
    if let Some(token) = access_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("MCP-Server nicht erreichbar: {e}"))?;

    let status = resp.status();
    let new_session = resp
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let www_authenticate = resp
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if status == StatusCode::ACCEPTED || status == StatusCode::UNAUTHORIZED {
        return Ok(RpcResult {
            status,
            session_id: new_session,
            www_authenticate,
            body: Value::Null,
        });
    }

    let text = resp.text().await.unwrap_or_default();
    let body: Value = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).map_err(|e| format!("Ungültige JSON-RPC-Antwort: {e}"))?
    };
    Ok(RpcResult {
        status,
        session_id: new_session,
        www_authenticate,
        body,
    })
}

fn parse_tools(body: &Value) -> Vec<McpTool> {
    body["result"]["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .map(|t| McpTool {
                    name: t["name"].as_str().unwrap_or_default().to_string(),
                    description: t["description"].as_str().map(|s| s.to_string()),
                    input_schema: t["inputSchema"].clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Führt den Initialize-Handshake gegen `url` aus, löst bei Bedarf den
/// OAuth-Fluss aus (docs/PLAN.md, Abschnitt 7) und speichert die
/// gefundenen Werkzeuge unter dieser URL. Kein Modell beteiligt — der
/// Nutzer hat die Berechtigung bereits erteilt, das ist reine Verdrahtung.
pub async fn connect(state: &SharedMcp, url: &str) -> Result<Vec<McpTool>, String> {
    let client = reqwest::Client::new();
    let init_params = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": { "name": "Iris", "version": "0.1.0" }
    });

    let first = rpc_call(&client, url, None, None, "initialize", init_params.clone(), Some(1)).await?;

    let (init, access_token) = if first.status == StatusCode::UNAUTHORIZED {
        let header = first.www_authenticate.clone().ok_or_else(|| {
            "401 ohne WWW-Authenticate — kann keine Autorisierung finden".to_string()
        })?;
        let token = oauth::authorize_and_get_token(&header).await?;
        let retried = rpc_call(
            &client,
            url,
            None,
            Some(&token),
            "initialize",
            init_params,
            Some(1),
        )
        .await?;
        (retried, Some(token))
    } else {
        (first, None)
    };

    if !init.status.is_success() {
        return Err(format!("Initialize fehlgeschlagen: {}", init.status));
    }
    if init.body["error"].is_object() {
        return Err(format!("MCP-Server lehnte Initialize ab: {}", init.body["error"]));
    }

    let session_id = init.session_id;

    // notifications/initialized: Notification ohne id, Server antwortet 202.
    rpc_call(
        &client,
        url,
        session_id.as_deref(),
        access_token.as_deref(),
        "notifications/initialized",
        json!({}),
        None,
    )
    .await?;

    let list = rpc_call(
        &client,
        url,
        session_id.as_deref(),
        access_token.as_deref(),
        "tools/list",
        json!({}),
        Some(2),
    )
    .await?;
    if !list.status.is_success() {
        return Err(format!("tools/list fehlgeschlagen: {}", list.status));
    }
    let tools = parse_tools(&list.body);

    let mut s = state.lock().unwrap();
    s.servers.insert(
        url.to_string(),
        ConnectedServer {
            session_id,
            access_token,
            tools: tools.clone(),
        },
    );
    Ok(tools)
}

pub async fn call_tool(
    state: &SharedMcp,
    url: &str,
    tool: &str,
    arguments: Value,
) -> Result<Value, String> {
    let (session_id, access_token) = {
        let s = state.lock().unwrap();
        let server = s
            .servers
            .get(url)
            .ok_or_else(|| format!("Nicht verbunden mit {url}"))?;
        (server.session_id.clone(), server.access_token.clone())
    };

    let client = reqwest::Client::new();
    let result = rpc_call(
        &client,
        url,
        session_id.as_deref(),
        access_token.as_deref(),
        "tools/call",
        json!({ "name": tool, "arguments": arguments }),
        Some(3),
    )
    .await?;

    if !result.status.is_success() {
        return Err(format!("tools/call fehlgeschlagen: {}", result.status));
    }
    if result.body["error"].is_object() {
        return Err(format!("MCP-Server meldet Fehler: {}", result.body["error"]));
    }
    Ok(result.body["result"].clone())
}

/// Erkennt eine Verbindungsabsicht mit einer http(s)-URL im freien
/// Prompt-Text. `None`, wenn keine URL erkennbar ist.
pub fn find_server_url(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(|w| w.trim_matches(|c: char| ".,;)]\"'".contains(c)).to_string())
}
