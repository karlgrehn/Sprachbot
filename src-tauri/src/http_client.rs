//! Ein geteilter `reqwest::Client` statt einem neuen pro Aufruf — reqwest
//! hält intern einen Verbindungspool, der sonst bei jedem Aufruf verworfen
//! und neu aufgebaut würde (TLS-Handshake inklusive bei entfernten
//! MCP-Servern).

use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}
