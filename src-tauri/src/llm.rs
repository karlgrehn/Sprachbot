//! Zweites LLM-Backend neben Ollama: ein Cloud-API-Key, den der Nutzer im
//! Chat hinzufügt (kein Einstellungsmenü, keine Provider-Auswahl — der
//! Anbieter ergibt sich aus dem Format des Keys). Der Grund: nicht jeder
//! will/kann sich Ollama selbst einrichten.
//!
//! Die einzige Regel, die eingehalten wird: es steht Iris immer mindestens
//! ein nutzbares Modell zur Verfügung. Deshalb sind beide Wechsel rote
//! Aktionen (permissions.rs) und gegenseitig gesperrt:
//! - Ollama deinstallieren geht nur, solange ein geprüfter API-Key da ist.
//! - Den API-Key entfernen geht nur, solange Ollama wieder aktiv ist.
//! - "Ollama installieren" gibt es als Aktion nur, solange Ollama gerade
//!   deaktiviert ist — sonst bräuchte es die Aktion gar nicht.

use crate::command::{CommandResult, PermissionRequest};
use crate::http_client::client;
use crate::permissions;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Anthropic,
}

impl Provider {
    /// Erkennt den Anbieter am Format des Keys — das ersetzt eine
    /// Provider-Auswahl, die es sonst als eigenes Eingabefeld bräuchte.
    fn from_key(key: &str) -> Option<Self> {
        if key.starts_with("sk-ant-") {
            Some(Provider::Anthropic)
        } else if key.starts_with("sk-") {
            Some(Provider::Openai)
        } else {
            None
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Openai => "OpenAI",
            Provider::Anthropic => "Anthropic",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Provider::Openai => "gpt-4o-mini",
            Provider::Anthropic => "claude-haiku-4-5-20251001",
        }
    }
}

struct CloudKey {
    provider: Provider,
    key: String,
}

pub struct LlmState {
    ollama_active: bool,
    cloud: Option<CloudKey>,
}

impl Default for LlmState {
    fn default() -> Self {
        Self {
            ollama_active: true,
            cloud: None,
        }
    }
}

pub type SharedLlm = Arc<Mutex<LlmState>>;

pub fn new_state() -> SharedLlm {
    Arc::new(Mutex::new(LlmState::default()))
}

/// Was `/api/ask` tatsächlich für die nächste echte Modellanfrage benutzen
/// soll — als eigenständiger Wert statt eines Locks, der über ein `.await`
/// hinweg gehalten werden müsste.
pub enum Backend {
    Cloud { provider: Provider, key: String },
    Ollama,
}

pub fn current_backend(state: &SharedLlm) -> Backend {
    let s = state.lock().unwrap();
    match &s.cloud {
        Some(c) => Backend::Cloud {
            provider: c.provider,
            key: c.key.clone(),
        },
        None => Backend::Ollama,
    }
}

pub fn ollama_active(state: &SharedLlm) -> bool {
    state.lock().unwrap().ollama_active
}

#[derive(Serialize)]
pub struct LlmStatus {
    pub ollama_active: bool,
    pub cloud_provider: Option<&'static str>,
}

pub fn status(state: &SharedLlm) -> LlmStatus {
    let s = state.lock().unwrap();
    LlmStatus {
        ollama_active: s.ollama_active,
        cloud_provider: s.cloud.as_ref().map(|c| c.provider.label()),
    }
}

async fn generate_openai(key: &str, prompt: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Msg<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Serialize)]
    struct Req<'a> {
        model: &'a str,
        messages: Vec<Msg<'a>>,
    }
    #[derive(Deserialize)]
    struct RespMsg {
        content: String,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: RespMsg,
    }
    #[derive(Deserialize)]
    struct Resp {
        choices: Vec<Choice>,
    }

    let resp = client()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&Req {
            model: Provider::Openai.default_model(),
            messages: vec![Msg {
                role: "user",
                content: prompt,
            }],
        })
        .send()
        .await
        .map_err(|e| format!("OpenAI nicht erreichbar: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI-Fehler ({status}): {text}"));
    }

    let parsed = resp
        .json::<Resp>()
        .await
        .map_err(|e| format!("Antwort von OpenAI nicht lesbar: {e}"))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "OpenAI-Antwort ohne Inhalt".to_string())
}

async fn generate_anthropic(key: &str, prompt: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Msg<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Serialize)]
    struct Req<'a> {
        model: &'a str,
        max_tokens: u32,
        messages: Vec<Msg<'a>>,
    }
    #[derive(Deserialize)]
    struct Block {
        text: String,
    }
    #[derive(Deserialize)]
    struct Resp {
        content: Vec<Block>,
    }

    let resp = client()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&Req {
            model: Provider::Anthropic.default_model(),
            max_tokens: 1024,
            messages: vec![Msg {
                role: "user",
                content: prompt,
            }],
        })
        .send()
        .await
        .map_err(|e| format!("Anthropic nicht erreichbar: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic-Fehler ({status}): {text}"));
    }

    let parsed = resp
        .json::<Resp>()
        .await
        .map_err(|e| format!("Antwort von Anthropic nicht lesbar: {e}"))?;
    parsed
        .content
        .into_iter()
        .next()
        .map(|b| b.text)
        .ok_or_else(|| "Anthropic-Antwort ohne Inhalt".to_string())
}

pub async fn cloud_generate(provider: Provider, key: &str, prompt: &str) -> Result<String, String> {
    match provider {
        Provider::Openai => generate_openai(key, prompt).await,
        Provider::Anthropic => generate_anthropic(key, prompt).await,
    }
}

/// Wird beim Erteilen von `llm.api_key.hinzufuegen` aufgerufen (siehe
/// api.rs::grant_handler). Prüft den Key mit einer echten, minimalen
/// Anfrage, bevor er gespeichert wird — ein falscher Key soll nicht erst
/// bei der nächsten echten Frage aus Versehen auffallen.
pub async fn add_key(state: &SharedLlm, key: &str) -> Result<&'static str, String> {
    let provider = Provider::from_key(key).ok_or_else(|| {
        "Unbekanntes Key-Format (erkannt werden OpenAI-Keys \"sk-...\" und Anthropic-Keys \"sk-ant-...\")"
            .to_string()
    })?;

    cloud_generate(provider, key, "Antworte nur mit OK.").await?;

    let mut s = state.lock().unwrap();
    s.cloud = Some(CloudKey {
        provider,
        key: key.to_string(),
    });
    Ok(provider.label())
}

pub fn has_valid_key(state: &SharedLlm) -> bool {
    state.lock().unwrap().cloud.is_some()
}

pub fn remove_key(state: &SharedLlm) {
    state.lock().unwrap().cloud = None;
}

pub fn set_ollama_active(state: &SharedLlm, active: bool) {
    state.lock().unwrap().ollama_active = active;
}

/// Sucht ein Wort im Prompt, das wie ein API-Key aussieht — dasselbe Muster
/// wie `mcp::find_server_url` für URLs, nur für Keys statt Adressen.
fn find_api_key(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .find(|w| w.starts_with("sk-") && w.len() > 12)
        .map(|w| w.trim_matches(|c: char| ".,;)]\"'".contains(c)).to_string())
}

/// Erkennt LLM-Backend-Absichten im freien Prompt-Text: API-Key
/// hinzufügen/entfernen, Ollama deinstallieren/installieren. `None`, wenn
/// der Text keine solche Absicht anfordert. Anders als bei WhatsApp/MCP
/// steckt der jeweilige Freigabezustand nicht in permissions.rs, sondern
/// direkt im `LlmState` (`cloud`/`ollama_active`) — die IDs dort dienen nur
/// noch dem Button-Anzeigetext und der grant_handler-Verdrahtung.
pub fn handle_command(llm_state: &SharedLlm, prompt: &str) -> Option<CommandResult> {
    let lower = prompt.to_lowercase();

    if let Some(key) = find_api_key(prompt) {
        if has_valid_key(llm_state) {
            return Some(CommandResult::message(
                "Es ist schon ein API-Key hinterlegt. Erst \"entferne den API-Key\" sagen, dann einen neuen hinzufügen.".to_string(),
            ));
        }
        let def = permissions::def_for(permissions::LLM_API_KEY_HINZUFUEGEN)
            .expect("llm.api_key.hinzufuegen muss in PERMISSIONS stehen");
        return Some(CommandResult {
            message: format!("[BERECHTIGUNG ANGEFRAGT]\n{}", def.anzeigetext),
            permission_request: Some(PermissionRequest {
                id: def.id,
                anzeigetext: def.anzeigetext,
                scope: Some(key),
            }),
        });
    }

    let mentions_key = lower.contains("api-key") || lower.contains("api key") || lower.contains("apikey") || lower.contains("schlüssel");
    let wants_remove_key =
        mentions_key && (lower.contains("entfern") || lower.contains("lösch") || lower.contains("widerruf"));
    if wants_remove_key {
        if !has_valid_key(llm_state) {
            return Some(CommandResult::message("Es ist kein API-Key hinterlegt.".to_string()));
        }
        if !ollama_active(llm_state) {
            return Some(CommandResult::message(
                "Der API-Key lässt sich erst entfernen, wenn Ollama wieder aktiv ist. Sag \"installiere Ollama\", dann geht's.".to_string(),
            ));
        }
        remove_key(llm_state);
        return Some(CommandResult::message(
            "API-Key entfernt. Iris nutzt wieder nur Ollama.".to_string(),
        ));
    }

    let mentions_ollama = lower.contains("ollama");
    let wants_uninstall = mentions_ollama
        && (lower.contains("deinstall") || lower.contains("brauch") && lower.contains("nicht mehr"));
    if wants_uninstall {
        if !ollama_active(llm_state) {
            return Some(CommandResult::message("Ollama ist bereits deaktiviert.".to_string()));
        }
        if !has_valid_key(llm_state) {
            return Some(CommandResult::message(
                "Ohne funktionierenden API-Key kann Ollama nicht deinstalliert werden — Iris braucht immer ein nutzbares Modell.".to_string(),
            ));
        }
        let def = permissions::def_for(permissions::LLM_OLLAMA_DEINSTALLIEREN)
            .expect("llm.ollama.deinstallieren muss in PERMISSIONS stehen");
        return Some(CommandResult {
            message: format!("[BERECHTIGUNG ANGEFRAGT]\n{}", def.anzeigetext),
            permission_request: Some(PermissionRequest {
                id: def.id,
                anzeigetext: def.anzeigetext,
                scope: None,
            }),
        });
    }

    let wants_install = mentions_ollama && (lower.contains("installier") || lower.contains("aktivier"));
    if wants_install {
        if ollama_active(llm_state) {
            return Some(CommandResult::message("Ollama ist schon aktiv.".to_string()));
        }
        let def = permissions::def_for(permissions::LLM_OLLAMA_INSTALLIEREN)
            .expect("llm.ollama.installieren muss in PERMISSIONS stehen");
        return Some(CommandResult {
            message: format!("[BERECHTIGUNG ANGEFRAGT]\n{}", def.anzeigetext),
            permission_request: Some(PermissionRequest {
                id: def.id,
                anzeigetext: def.anzeigetext,
                scope: None,
            }),
        });
    }

    None
}
