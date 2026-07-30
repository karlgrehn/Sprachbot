# Iris

Ein persönlicher Agent. Ein einziges Textfeld steuert alles.
Zielmetrik: die gesamte Gerätezeit des Nutzers minimieren — nicht Antwortlatenz,
nicht Funktionsumfang.

Der vollständige Umsetzungsplan steht in [`docs/PLAN.md`](docs/PLAN.md).
Ausbaustufen werden dort einzeln beschrieben; **erst M1, dann M2, nichts
parallel.**

## Die eine Architekturgrenze, die nicht verschoben wird

**Die gesamte Logik liegt in einem lokalen HTTP-Service (`src-tauri/src/api.rs`,
Port 47615). Die Tauri-Oberfläche ist nur ein Client davon** — sie spricht
ausschließlich über `fetch()` mit dem Kern, nie über Tauri-`invoke`. Das
Frontend hat keine Tauri-spezifischen Abhängigkeiten mehr. Damit ist der
spätere Thin Client (M6) dieselbe Web-Oberfläche über Tailscale, keine
Neuentwicklung. Diese Grenze ist die einzige Entscheidung, die laut Plan
später nicht mehr nachrüstbar ist — alles andere darf sich ändern.

## Status

**M1 — Kern und lokales Modell.** In Arbeit.

- [x] Lokaler HTTP-Kern (Axum) mit `/api/status`, `/api/models`, `/api/ask`
- [x] Tauri-Fenster als erster Client, spricht nur über HTTP mit dem Kern
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

Der Kern läuft auf `http://127.0.0.1:47615` und lässt sich unabhängig vom
Tauri-Fenster mit `curl` prüfen, z. B. `curl http://127.0.0.1:47615/api/status`.

## Architektur (Kurzfassung)

Drei Schichten (Details in [`docs/PLAN.md`](docs/PLAN.md), Abschnitt 2):

| Schicht | Läuft auf | Aufgabe | Immer an? |
|---|---|---|---|
| Hub | Pi / kleine Cloud-Instanz | Bridges, Nachrichteneingang, Warteschlange | ja |
| Worker | Laptop des Nutzers | Inferenz, Zusammenfassen, Entwürfe | nein |
| View | beliebiges Gerät | Anzeige und Eingabe | nein |

M1 baut den Anfang des Workers: den lokalen HTTP-Kern samt Ollama-Anbindung,
mit dem Tauri-Fenster als erstem (Fat-Client-)View.

| Baustein | Wahl |
|---|---|
| App-Hülle | Tauri (Rust + Web-Frontend) |
| Kern | lokaler HTTP-Service (Axum) |
| Modell lokal | Ollama, Gemma-Klasse, RAM-abhängig gewählt |
| Messaging (ab M2) | Conduit (Matrix) + mautrix-Bridges als Sidecars |
| Werkzeuge (ab M3) | MCP-Client gegen Remote-MCP-Server |

## Bekannte Einschränkung dieser Sandbox

In der Entwicklungsumgebung, in der dieser Code entstanden ist, ließen sich
die Linux-Systempakete für WebKitGTK nicht über den Paketspiegel laden (404
auf `security.ubuntu.com`). Die Rust-Kernlogik (RAM-Erkennung, Ollama-Client,
Axum-HTTP-Kern mit allen drei Routen) wurde isoliert gegen echte Abhängigkeiten
kompiliert und lief korrekt — inklusive eines echten HTTP-Roundtrips gegen
`/api/status` und `/api/models`. Ein vollständiger `cargo tauri build`/`dev`
mit echtem Fenster wurde dort nicht verifiziert. Auf einer normalen
Linux-Arbeitsstation mit installierten Tauri-Voraussetzungen sollte
`npm run tauri dev` ohne Weiteres funktionieren.
