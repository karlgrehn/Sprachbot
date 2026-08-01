//! Ergebnis-Postfach statt Push-Benachrichtigung (docs/PLAN.md, Abschnitt 6:
//! Push-Benachrichtigungen sind ein Nicht-Ziel). Zusammenfassungen landen
//! hier und werden abgeholt, wenn der Nutzer danach fragt — nicht sofort
//! zugestellt.

use crate::clock::now_unix;
use crate::command::CommandResult;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone)]
pub struct PostfachEintrag {
    pub titel: String,
    pub inhalt: String,
    pub erstellt_unix: u64,
}

#[derive(Default)]
pub struct Postfach {
    eintraege: Vec<PostfachEintrag>,
}

pub type SharedPostfach = Arc<Mutex<Postfach>>;

pub fn new_state() -> SharedPostfach {
    Arc::new(Mutex::new(Postfach::default()))
}

pub fn ablegen(state: &SharedPostfach, titel: String, inhalt: String) {
    let eintrag = PostfachEintrag {
        titel,
        inhalt,
        erstellt_unix: now_unix(),
    };
    let mut s = state.lock().unwrap();
    s.eintraege.insert(0, eintrag);
}

pub fn liste(state: &SharedPostfach) -> Vec<PostfachEintrag> {
    state.lock().unwrap().eintraege.clone()
}

/// Erkennt eine Nachfrage nach dem Postfach im freien Prompt-Text
/// ("Was ist heute reingekommen?"). `None`, wenn der Text keine solche
/// Nachfrage ist.
pub fn handle_command(state: &SharedPostfach, prompt: &str) -> Option<CommandResult> {
    let lower = prompt.to_lowercase();
    let fragt_nach_neuigkeiten = lower.contains("reingekommen")
        || lower.contains("was gibt es neues")
        || lower.contains("postfach");
    if !fragt_nach_neuigkeiten {
        return None;
    }

    let eintraege = liste(state);
    if eintraege.is_empty() {
        return Some(CommandResult::message(
            "Noch keine Nachrichten verbunden. Sag \"verbinde mein WhatsApp\", um zu starten."
                .to_string(),
        ));
    }

    let mut zeilen = vec!["Im Postfach:".to_string()];
    for e in eintraege.iter().take(5) {
        zeilen.push(format!("- {}: {}", e.titel, e.inhalt));
    }
    Some(CommandResult::message(zeilen.join("\n")))
}
