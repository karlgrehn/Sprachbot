//! Ein einziger Ort für "jetzt als Unix-Sekunden" statt der gleichen
//! `SystemTime`-Umrechnung an mehreren Stellen.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
