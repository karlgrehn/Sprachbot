use serde::{Deserialize, Serialize};

const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagEntry>,
}

#[derive(Deserialize)]
struct TagEntry {
    name: String,
}

/// True if a local Ollama instance answers, i.e. Iris can run fully offline.
pub async fn is_available() -> bool {
    reqwest::Client::new()
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .send()
        .await
        .is_ok()
}

pub async fn list_installed_models() -> Result<Vec<String>, String> {
    let resp = reqwest::Client::new()
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .send()
        .await
        .map_err(|e| format!("Ollama nicht erreichbar: {e}"))?
        .json::<TagsResponse>()
        .await
        .map_err(|e| format!("Antwort von Ollama nicht lesbar: {e}"))?;
    Ok(resp.models.into_iter().map(|m| m.name).collect())
}

pub async fn generate(model: &str, prompt: &str) -> Result<String, String> {
    let body = GenerateRequest {
        model,
        prompt,
        stream: false,
    };
    let resp = reqwest::Client::new()
        .post(format!("{OLLAMA_BASE_URL}/api/generate"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama nicht erreichbar: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama-Fehler ({status}): {text}"));
    }

    let parsed = resp
        .json::<GenerateResponse>()
        .await
        .map_err(|e| format!("Antwort von Ollama nicht lesbar: {e}"))?;
    Ok(parsed.response)
}
