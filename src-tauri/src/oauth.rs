//! Minimaler OAuth-2.0-Client (Authorization Code + PKCE), wie ihn MCP für
//! entfernte Server verlangt, die eine Autorisierung fordern (docs/PLAN.md,
//! Abschnitt 7: "MCP-Client, OAuth-Flow"). Entdeckung folgt RFC 9728
//! (Protected Resource Metadata) und RFC 8414 (Authorization Server
//! Metadata). Der Redirect läuft über einen kurzlebigen lokalen
//! HTTP-Listener, kein Custom-URL-Scheme.

use axum::{extract::Query, routing::get, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

/// Fester lokaler Redirect-Port. Muss bei einem echten MCP-Server als
/// erlaubte redirect_uri registriert sein (`http://127.0.0.1:47616/callback`).
pub const REDIRECT_PORT: u16 = 47616;

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

pub fn new_pkce() -> Pkce {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    Pkce { verifier, challenge }
}

pub fn new_state_token() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub struct AuthEndpoints {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

#[derive(Deserialize)]
struct ProtectedResourceMetadata {
    authorization_servers: Vec<String>,
}

#[derive(Deserialize)]
struct AuthServerMetadata {
    authorization_endpoint: String,
    token_endpoint: String,
}

fn extract_resource_metadata_url(www_authenticate: &str) -> Option<String> {
    let marker = "resource_metadata=\"";
    let start = www_authenticate.find(marker)? + marker.len();
    let rest = &www_authenticate[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Liest `WWW-Authenticate: Bearer resource_metadata="..."` aus einer
/// 401-Antwort und ermittelt daraus Autorisierungs- und Token-Endpunkt.
pub async fn discover(www_authenticate: &str) -> Result<AuthEndpoints, String> {
    let resource_metadata_url = extract_resource_metadata_url(www_authenticate)
        .ok_or_else(|| "WWW-Authenticate ohne resource_metadata".to_string())?;

    let client = reqwest::Client::new();
    let resource_meta: ProtectedResourceMetadata = client
        .get(&resource_metadata_url)
        .send()
        .await
        .map_err(|e| format!("Resource-Metadaten nicht erreichbar: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Resource-Metadaten ungültig: {e}"))?;

    let as_url = resource_meta
        .authorization_servers
        .first()
        .ok_or_else(|| "Keine Autorisierungsserver in den Resource-Metadaten".to_string())?;

    let as_meta: AuthServerMetadata = client
        .get(format!("{as_url}/.well-known/oauth-authorization-server"))
        .send()
        .await
        .map_err(|e| format!("Authorization-Server-Metadaten nicht erreichbar: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Authorization-Server-Metadaten ungültig: {e}"))?;

    Ok(AuthEndpoints {
        authorization_endpoint: as_meta.authorization_endpoint,
        token_endpoint: as_meta.token_endpoint,
    })
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Startet einen kurzlebigen lokalen HTTP-Server für genau einen
/// Redirect-Callback und liefert den Autorisierungscode, sobald er
/// eintrifft. `expected_state` schützt gegen CSRF (RFC 6749 §10.12).
pub async fn wait_for_redirect(expected_state: String) -> Result<String, String> {
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let app = Router::new().route(
        "/callback",
        get(move |Query(params): Query<CallbackParams>| {
            let tx = tx.clone();
            let expected_state = expected_state.clone();
            async move {
                let result = if let Some(error) = params.error {
                    Err(format!("Autorisierung abgelehnt: {error}"))
                } else {
                    match (params.code, params.state) {
                        (Some(code), Some(state)) if state == expected_state => Ok(code),
                        (Some(_), Some(_)) => {
                            Err("state stimmt nicht überein — möglicher CSRF-Versuch".to_string())
                        }
                        _ => Err("Callback ohne code/state".to_string()),
                    }
                };
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(result);
                }
                "Autorisierung abgeschlossen, dieses Fenster kann geschlossen werden."
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
        .await
        .map_err(|e| format!("Redirect-Port {REDIRECT_PORT} konnte nicht gebunden werden: {e}"))?;

    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let outcome = match tokio::time::timeout(Duration::from_secs(120), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Redirect-Callback wurde nie ausgelöst".to_string()),
        Err(_) => Err("Zeitüberschreitung beim Warten auf die Autorisierung".to_string()),
    };
    server.abort();
    outcome
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn exchange_code(
    token_endpoint: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<String, String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", redirect_uri),
        ("client_id", "iris"),
    ];
    let resp = reqwest::Client::new()
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token-Endpunkt nicht erreichbar: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Token-Austausch fehlgeschlagen ({status}): {text}"));
    }

    resp.json::<TokenResponse>()
        .await
        .map(|t| t.access_token)
        .map_err(|e| format!("Token-Antwort nicht lesbar: {e}"))
}

/// Kompletter Ablauf ab einer 401-Antwort: Entdeckung, PKCE, Browser
/// öffnen, auf den Redirect warten, Code gegen Token tauschen.
pub async fn authorize_and_get_token(www_authenticate: &str) -> Result<String, String> {
    let endpoints = discover(www_authenticate).await?;
    let pkce = new_pkce();
    let state_token = new_state_token();
    let redirect_uri = format!("http://127.0.0.1:{REDIRECT_PORT}/callback");
    let redirect_uri_enc: String =
        url::form_urlencoded::byte_serialize(redirect_uri.as_bytes()).collect();

    let auth_url = format!(
        "{}?response_type=code&client_id=iris&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        endpoints.authorization_endpoint, redirect_uri_enc, pkce.challenge, state_token
    );

    open_browser(&auth_url);

    let code = wait_for_redirect(state_token).await?;
    exchange_code(&endpoints.token_endpoint, &code, &pkce.verifier, &redirect_uri).await
}
