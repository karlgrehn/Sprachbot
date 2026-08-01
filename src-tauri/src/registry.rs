//! Die Aktions-Registry: fest im Code definiert, nie vom Modell erzeugt
//! (docs/PLAN.md, Abschnitt 2). Anzeigetexte und IDs stehen hier, nicht im
//! Kontext des Modells — eine präparierte Nachricht kann höchstens eine
//! vorhandene ID anfordern, nie einen neuen Text erfinden.
//!
//! M1 enthält ausschließlich grüne (reversible) Aktionen: der Nutzer
//! beschreibt eine Änderung in freier Sprache, Iris setzt sie ohne
//! Rückfrage um, und "mach das rückgängig" nimmt sie zuverlässig zurück.
//! Rote (berechtigungspflichtige) Aktionen kommen erst mit M2, wenn es
//! überhaupt etwas gibt, das nicht trivial rückgängig zu machen ist.

use crate::command::CommandResult;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Copy)]
pub struct Action {
    pub id: &'static str,
    pub anzeigetext: &'static str,
    pub reversible: bool,
}

const KURZ: Action = Action {
    id: "antwortstil.kurz",
    anzeigetext: "Iris antwortet kurz und knapp",
    reversible: true,
};
const NORMAL: Action = Action {
    id: "antwortstil.normal",
    anzeigetext: "Iris antwortet in normaler Länge",
    reversible: true,
};
const AUSFUEHRLICH: Action = Action {
    id: "antwortstil.ausfuehrlich",
    anzeigetext: "Iris antwortet ausführlich",
    reversible: true,
};

pub const REGISTRY: &[Action] = &[KURZ, NORMAL, AUSFUEHRLICH];

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Kurz,
    Normal,
    Ausfuehrlich,
}

impl Verbosity {
    fn instruction(self) -> &'static str {
        match self {
            Verbosity::Kurz => "Antworte in maximal zwei Sätzen.",
            Verbosity::Normal => "Antworte in normaler Länge.",
            Verbosity::Ausfuehrlich => "Antworte ausführlich und mit Beispielen.",
        }
    }

    fn action(self) -> &'static Action {
        match self {
            Verbosity::Kurz => &KURZ,
            Verbosity::Normal => &NORMAL,
            Verbosity::Ausfuehrlich => &AUSFUEHRLICH,
        }
    }
}

pub struct VerbosityState {
    current: Verbosity,
    history: Vec<Verbosity>,
}

impl Default for VerbosityState {
    fn default() -> Self {
        Self {
            current: Verbosity::Normal,
            history: Vec::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<VerbosityState>>;

pub fn new_state() -> SharedState {
    Arc::new(Mutex::new(VerbosityState::default()))
}

pub fn current_instruction(state: &SharedState) -> String {
    state.lock().unwrap().current.instruction().to_string()
}

/// Erkennt eine grüne Einstellungsänderung oder ein Rückgängig im freien
/// Prompt-Text und führt sie sofort aus. `None`, wenn der Text keine
/// bekannte Aktion anfordert — dann läuft der Prompt normal ans Modell.
pub fn handle_command(state: &SharedState, prompt: &str) -> Option<CommandResult> {
    let lower = prompt.to_lowercase();

    let is_undo = lower.contains("rückgängig") || lower.contains("rueckgaengig");
    if is_undo {
        let mut s = state.lock().unwrap();
        let message = match s.history.pop() {
            Some(previous) => {
                s.current = previous;
                format!("Rückgängig gemacht. {}.", previous.action().anzeigetext)
            }
            None => "Nichts zum Rückgängigmachen vorhanden.".to_string(),
        };
        return Some(CommandResult::message(message));
    }

    let mentions_antwort = lower.contains("antwort");
    let target = if mentions_antwort && lower.contains("kurz") {
        Some(Verbosity::Kurz)
    } else if mentions_antwort && (lower.contains("ausführlich") || lower.contains("ausfuehrlich"))
    {
        Some(Verbosity::Ausfuehrlich)
    } else if mentions_antwort && lower.contains("normal") {
        Some(Verbosity::Normal)
    } else {
        None
    };

    target.map(|new_style| {
        let mut s = state.lock().unwrap();
        if s.current != new_style {
            let previous = s.current;
            s.history.push(previous);
            s.current = new_style;
        }
        CommandResult::message(format!("{}.", new_style.action().anzeigetext))
    })
}
