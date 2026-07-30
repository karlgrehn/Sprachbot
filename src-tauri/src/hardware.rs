use serde::Serialize;
use sysinfo::System;

/// Model tiers Iris picks between, ordered from smallest to largest.
/// Chosen from the Gemma 3 family (Ollama tags) per the M1 spec:
/// "Ollama-Anbindung mit Modellwahl nach erkanntem RAM".
const TIER_LOW: &str = "gemma3:1b";
const TIER_MID: &str = "gemma3:4b";
const TIER_HIGH: &str = "gemma3:12b";

#[derive(Serialize)]
pub struct ModelRecommendation {
    pub ram_gb: f64,
    pub tier: &'static str,
    pub model: &'static str,
}

fn tier_for_ram(ram_gb: f64) -> (&'static str, &'static str) {
    if ram_gb >= 16.0 {
        ("high", TIER_HIGH)
    } else if ram_gb >= 8.0 {
        ("mid", TIER_MID)
    } else {
        ("low", TIER_LOW)
    }
}

pub fn recommend_model() -> ModelRecommendation {
    let mut sys = System::new();
    sys.refresh_memory();
    let ram_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let (tier, model) = tier_for_ram(ram_gb);
    ModelRecommendation { ram_gb, tier, model }
}
