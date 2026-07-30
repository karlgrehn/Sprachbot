# Iris

Ein persönlicher Agent als Desktop-App. Ein einziges Textfeld steuert alles.
Zielmetrik: die gesamte Gerätezeit des Nutzers minimieren — nicht Antwortlatenz,
nicht Funktionsumfang.

Der vollständige Umsetzungsplan steht in [`docs/PLAN.md`](docs/PLAN.md).
Ausbaustufen werden dort einzeln beschrieben; **erst M1, dann M2, nichts
parallel.**

## Status

**M1 — Hülle und lokales Modell.** In Arbeit.

- [x] Tauri-App-Grundgerüst (Rust-Backend + Vanilla-TS-Frontend)
- [x] Ein Textfeld, ein Ausgabebereich
- [x] RAM-Erkennung (`sysinfo`) mit Modellempfehlung (Gemma-3-Klasse: 1B / 4B / 12B je nach RAM)
- [x] Ollama-Anbindung (`/api/tags`, `/api/generate`) ohne Internet
- [ ] Im Alltag getestet: Prompt rein, lokale Antwort raus, ohne Internet

M1 gilt erst als fertig, wenn das auf einem echten Rechner mit installiertem
Ollama getestet wurde — das kann in dieser Sandbox nicht verifiziert werden
(siehe Abschnitt „Bekannte Einschränkung dieser Sandbox" unten).

## Voraussetzungen

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- [Ollama](https://ollama.com/) lokal installiert und laufend (`ollama serve`)
- Linux: `webkit2gtk-4.1`, `librsvg2`, `libayatana-appindicator3` (Dev-Pakete),
  siehe [Tauri-Voraussetzungen](https://tauri.app/start/prerequisites/)

## Entwickeln

```bash
npm install
npm run tauri dev
```

Vor dem ersten Start ein Modell laden, passend zur RAM-Empfehlung, die die
App anzeigt, z. B.:

```bash
ollama pull gemma3:4b
```

## Architektur (Kurzfassung)

| Baustein | Wahl |
|---|---|
| App-Hülle | Tauri (Rust + Web-Frontend) |
| Modell lokal | Ollama, Gemma-Klasse, RAM-abhängig gewählt |
| Messaging (ab M2) | Conduit (Matrix) + mautrix-Bridges als Sidecars |
| Werkzeuge (ab M3) | MCP-Client gegen Remote-MCP-Server |

Details, Nicht-Ziele und die Ausbaustufen M1–M7 stehen in
[`docs/PLAN.md`](docs/PLAN.md).

## Bekannte Einschränkung dieser Sandbox

In der Entwicklungsumgebung, in der dieser Code entstanden ist, ließen sich
die Linux-Systempakete für WebKitGTK nicht über den Paketspiegel laden (404
auf `security.ubuntu.com`). Die Rust-Kernlogik (RAM-Erkennung, Ollama-Client)
wurde isoliert gegen echte Abhängigkeiten kompiliert und lief korrekt; ein
vollständiger `cargo tauri build`/`dev` mit echtem Fenster wurde dort nicht
verifiziert. Auf einer normalen Linux-Arbeitsstation mit installierten
Tauri-Voraussetzungen sollte `npm run tauri dev` ohne Weiteres funktionieren.
