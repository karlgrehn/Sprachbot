//! Gemeinsame Antwortform für alle `handle_command`-Funktionen (registry,
//! permissions, postfach, mcp): entweder eine fertige Nachricht oder eine
//! Berechtigungsanfrage, deren Anzeigetext unverändert aus der jeweiligen
//! Registry stammt (docs/PLAN.md, Abschnitt 2).

use serde::Serialize;

#[derive(Serialize)]
pub struct PermissionRequest {
    pub id: &'static str,
    pub anzeigetext: &'static str,
    pub scope: Option<String>,
}

pub struct CommandResult {
    pub message: String,
    pub permission_request: Option<PermissionRequest>,
}

impl CommandResult {
    pub fn message(message: String) -> Self {
        Self {
            message,
            permission_request: None,
        }
    }
}
