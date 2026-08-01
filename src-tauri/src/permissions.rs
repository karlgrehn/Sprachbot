//! Berechtigungen: rote, dauerhafte Freigaben (docs/PLAN.md, Abschnitt 2).
//! Anders als die grünen Aktionen in registry.rs sind das Rechte, die Iris
//! *braucht*, bevor sie etwas Nicht-triviales tun darf. IDs und Anzeigetexte
//! sind fest im Code definiert — das Modell kann nur eine vorhandene ID
//! anfordern, nie einen eigenen Text oder eine neue Berechtigung erfinden.
//! Der `scope` ist die einzige freie Angabe (z. B. eine Server-URL bei MCP)
//! und verändert nie den Anzeigetext, nur worauf sich die Berechtigung
//! bezieht — das entspricht dem `geltung: { chat_id }`-Schema aus dem Plan.
//!
//! Erteilt werden Berechtigungen ausschließlich in der App (M1–M3 kennen
//! ohnehin noch keinen anderen Kanal). Der Chat-mit-sich-selbst-Kanal aus
//! dem Plan kommt erst mit M4 hinzu.

use crate::clock::now_unix;
use crate::command::{CommandResult, PermissionRequest};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Copy)]
pub struct PermissionDef {
    pub id: &'static str,
    pub anzeigetext: &'static str,
}

/// IDs als Konstanten statt wiederholter String-Literale — jede Stelle, die
/// eine Berechtigung anfragt oder prüft, referenziert dieselbe Konstante
/// wie der Registry-Eintrag selbst.
pub const WHATSAPP_KOPPELN: &str = "bridge.whatsapp.koppeln";
pub const MCP_SERVER_VERBINDEN: &str = "mcp.server.verbinden";

pub const PERMISSIONS: &[PermissionDef] = &[
    PermissionDef {
        id: WHATSAPP_KOPPELN,
        anzeigetext: "Iris darf mein WhatsApp-Konto koppeln, um Nachrichten zu lesen",
    },
    PermissionDef {
        id: MCP_SERVER_VERBINDEN,
        anzeigetext: "Iris darf sich mit diesem MCP-Server verbinden und dessen Werkzeuge benutzen",
    },
];

pub fn def_for(id: &str) -> Option<&'static PermissionDef> {
    PERMISSIONS.iter().find(|p| p.id == id)
}

#[derive(Serialize, Clone)]
pub struct GrantedPermission {
    pub id: String,
    pub scope: Option<String>,
    pub granted_at_unix: u64,
}

#[derive(Default)]
pub struct PermissionState {
    granted: Vec<GrantedPermission>,
}

pub type SharedPermissions = Arc<Mutex<PermissionState>>;

pub fn new_state() -> SharedPermissions {
    Arc::new(Mutex::new(PermissionState::default()))
}

pub fn grant(
    state: &SharedPermissions,
    id: &str,
    scope: Option<String>,
) -> Result<GrantedPermission, String> {
    let def = def_for(id).ok_or_else(|| format!("Unbekannte Berechtigungs-ID: {id}"))?;
    let granted = GrantedPermission {
        id: def.id.to_string(),
        scope,
        granted_at_unix: now_unix(),
    };
    let mut s = state.lock().unwrap();
    s.granted
        .retain(|g| !(g.id == granted.id && g.scope == granted.scope));
    s.granted.push(granted.clone());
    Ok(granted)
}

pub fn revoke(state: &SharedPermissions, id: &str, scope: Option<&str>) -> bool {
    let mut s = state.lock().unwrap();
    let before = s.granted.len();
    s.granted
        .retain(|g| !(g.id == id && g.scope.as_deref() == scope));
    s.granted.len() != before
}

pub fn is_granted(state: &SharedPermissions, id: &str, scope: Option<&str>) -> bool {
    state
        .lock()
        .unwrap()
        .granted
        .iter()
        .any(|g| g.id == id && g.scope.as_deref() == scope)
}

pub fn list_active(state: &SharedPermissions) -> Vec<GrantedPermission> {
    state.lock().unwrap().granted.clone()
}

/// Erkennt eine WhatsApp-Kopplungsabsicht im freien Prompt-Text. `None`,
/// wenn der Text keine solche Absicht anfordert. Die MCP-Erkennung lebt in
/// mcp.rs, weil sie zusätzlich eine URL aus dem Text lesen muss.
pub fn handle_command(state: &SharedPermissions, prompt: &str) -> Option<CommandResult> {
    let lower = prompt.to_lowercase();
    if !lower.contains("whatsapp") {
        return None;
    }

    let wants_revoke = lower.contains("widerruf") || lower.contains("trenn");
    if wants_revoke {
        let revoked = revoke(state, WHATSAPP_KOPPELN, None);
        let message = if revoked {
            "WhatsApp-Berechtigung widerrufen.".to_string()
        } else {
            "WhatsApp war nicht verbunden.".to_string()
        };
        return Some(CommandResult::message(message));
    }

    let wants_connect =
        lower.contains("verbind") || lower.contains("koppl") || lower.contains("anbind");
    if !wants_connect {
        return None;
    }

    if is_granted(state, WHATSAPP_KOPPELN, None) {
        return Some(CommandResult::message(
            "WhatsApp-Kopplung ist bereits erlaubt. Die Bridge selbst ist in dieser Version \
             noch nicht angebunden (siehe docs/PLAN.md, M2)."
                .to_string(),
        ));
    }

    let def =
        def_for(WHATSAPP_KOPPELN).expect("bridge.whatsapp.koppeln muss in PERMISSIONS stehen");
    Some(CommandResult {
        message: format!("[BERECHTIGUNG ANGEFRAGT]\n{}", def.anzeigetext),
        permission_request: Some(PermissionRequest {
            id: def.id,
            anzeigetext: def.anzeigetext,
            scope: None,
        }),
    })
}
