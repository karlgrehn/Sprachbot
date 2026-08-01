//! Berechtigungen: rote, dauerhafte Freigaben (docs/PLAN.md, Abschnitt 2).
//! Anders als die grünen Aktionen in registry.rs sind das Rechte, die Iris
//! *braucht*, bevor sie etwas Nicht-triviales tun darf. IDs und Anzeigetexte
//! sind fest im Code definiert — das Modell kann nur eine vorhandene ID
//! anfordern, nie einen eigenen Text oder eine neue Berechtigung erfinden.
//!
//! Erteilt werden Berechtigungen ausschließlich in der App (M1/M2 kennen
//! ohnehin noch keinen anderen Kanal). Der Chat-mit-sich-selbst-Kanal aus
//! dem Plan kommt erst mit M4 hinzu.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone, Copy)]
pub struct PermissionDef {
    pub id: &'static str,
    pub anzeigetext: &'static str,
}

pub const PERMISSIONS: &[PermissionDef] = &[PermissionDef {
    id: "bridge.whatsapp.koppeln",
    anzeigetext: "Iris darf mein WhatsApp-Konto koppeln, um Nachrichten zu lesen",
}];

fn def_for(id: &str) -> Option<&'static PermissionDef> {
    PERMISSIONS.iter().find(|p| p.id == id)
}

#[derive(Serialize, Clone)]
pub struct GrantedPermission {
    pub id: String,
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

pub fn grant(state: &SharedPermissions, id: &str) -> Result<GrantedPermission, String> {
    let def = def_for(id).ok_or_else(|| format!("Unbekannte Berechtigungs-ID: {id}"))?;
    let granted = GrantedPermission {
        id: def.id.to_string(),
        granted_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    let mut s = state.lock().unwrap();
    s.granted.retain(|g| g.id != granted.id);
    s.granted.push(granted.clone());
    Ok(granted)
}

pub fn revoke(state: &SharedPermissions, id: &str) -> bool {
    let mut s = state.lock().unwrap();
    let before = s.granted.len();
    s.granted.retain(|g| g.id != id);
    s.granted.len() != before
}

pub fn is_granted(state: &SharedPermissions, id: &str) -> bool {
    state.lock().unwrap().granted.iter().any(|g| g.id == id)
}

pub fn list_active(state: &SharedPermissions) -> Vec<GrantedPermission> {
    state.lock().unwrap().granted.clone()
}

pub struct CommandResult {
    pub message: String,
    pub permission_request: Option<PermissionDef>,
}

/// Erkennt Absichten, die eine Berechtigung brauchen, im freien Prompt-Text.
/// `None`, wenn der Text keine solche Absicht anfordert.
pub fn handle_command(state: &SharedPermissions, prompt: &str) -> Option<CommandResult> {
    const WHATSAPP_ID: &str = "bridge.whatsapp.koppeln";
    let lower = prompt.to_lowercase();
    if !lower.contains("whatsapp") {
        return None;
    }

    let wants_revoke = lower.contains("widerruf") || lower.contains("trenn");
    if wants_revoke {
        let revoked = revoke(state, WHATSAPP_ID);
        return Some(CommandResult {
            message: if revoked {
                "WhatsApp-Berechtigung widerrufen.".to_string()
            } else {
                "WhatsApp war nicht verbunden.".to_string()
            },
            permission_request: None,
        });
    }

    let wants_connect =
        lower.contains("verbind") || lower.contains("koppl") || lower.contains("anbind");
    if !wants_connect {
        return None;
    }

    if is_granted(state, WHATSAPP_ID) {
        return Some(CommandResult {
            message: "WhatsApp-Kopplung ist bereits erlaubt. Die Bridge selbst ist in dieser \
                      Version noch nicht angebunden (siehe docs/PLAN.md, M2)."
                .to_string(),
            permission_request: None,
        });
    }

    let def = def_for(WHATSAPP_ID).expect("bridge.whatsapp.koppeln muss in PERMISSIONS stehen");
    Some(CommandResult {
        message: format!("[BERECHTIGUNG ANGEFRAGT]\n{}", def.anzeigetext),
        permission_request: Some(*def),
    })
}
