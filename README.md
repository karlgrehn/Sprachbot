# Iris

Ein persönlicher Agent. Ein einziges Textfeld steuert alles — kein
Einstellungsmenü. Zielmetrik: die gesamte Gerätezeit des Nutzers minimieren,
nicht Antwortlatenz, nicht Funktionsumfang.

Der vollständige Umsetzungsplan steht in [`docs/PLAN.md`](docs/PLAN.md).
Ausbaustufen werden dort einzeln beschrieben; **erst M1, dann M2, nichts
parallel.**

## Die eine Architekturgrenze, die nicht verschoben wird

**Die gesamte Logik liegt in einem lokalen HTTP-Service (`src-tauri/src/api.rs`,
Port 47615). Die Tauri-Oberfläche ist nur ein Client davon** — sie spricht
ausschließlich über `fetch()` mit dem Kern, nie über Tauri-`invoke`. Das
Frontend hat keine Tauri-spezifischen Abhängigkeiten mehr. Damit sind Option 2
(Thin Client über Tailscale) und Option 3 (Server) später dieselbe Oberfläche
an einer anderen Adresse — Tage statt Monate Arbeit.

## Kein Einstellungsmenü — eine Aktions-Registry

Statt Schaltern gibt es `src-tauri/src/registry.rs`: feste IDs und
Anzeigetexte im Code, nie vom Modell erzeugt. Der Nutzer beschreibt in freier
Sprache, was er will (z. B. „antworte ab jetzt kurz"); ist die Änderung
problemlos rückgängig zu machen („grün"), setzt Iris sie ohne Rückfrage um.
„Mach das rückgängig" nimmt sie zuverlässig zurück (echte Historie, kein
Toggle-Hack). `/api/registry` ist die Datenquelle für `/help` — auch das
kommt aus der Registry, nicht aus dem Modellgedächtnis.

Berechtigungspflichtige („rote") Aktionen kommen erst mit M2 (WhatsApp-
Kopplung ist die erste), wenn es überhaupt etwas gibt, das nicht trivial
rückgängig zu machen ist.

## Status

**M1 — Kern, lokales Modell, Aktions-Registry.** In Arbeit.

- [x] Lokaler HTTP-Kern (Axum) mit `/api/status`, `/api/models`, `/api/registry`, `/api/ask`
- [x] Tauri-Fenster als erster Client, spricht nur über HTTP mit dem Kern
- [x] Ein Textfeld, ein Ausgabebereich, `/help`-Hinweis auf dem Startbildschirm
- [x] RAM-Erkennung (`sysinfo`) mit Modellempfehlung (Gemma-3-Klasse: 1B / 4B / 12B je nach RAM)
- [x] Ollama-Anbindung (`/api/tags`, `/api/generate`) ohne Internet
- [x] Eine grüne Aktion (Antwortstil kurz/normal/ausführlich), per Prompt änderbar und mit echter Undo-Historie rückgängig machbar
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
Tauri-Fenster prüfen:

```bash
curl http://127.0.0.1:47615/api/status
curl http://127.0.0.1:47615/api/registry
curl -X POST http://127.0.0.1:47615/api/ask \
  -H "Content-Type: application/json" \
  -d '{"model":"gemma3:4b","prompt":"antworte ab jetzt kurz"}'
```

## Architektur (Kurzfassung)

Drei Schichten (Details in [`docs/PLAN.md`](docs/PLAN.md), Abschnitt 5):

| Schicht | Läuft auf | Aufgabe | Immer an? |
|---|---|---|---|
| Hub | Laptop (Option 1+2) oder Server (Option 3) | Bridges, Nachrichteneingang, Warteschlange | je nach Option |
| Worker | Laptop des Nutzers | Inferenz, Zusammenfassen, Entwürfe | nein |
| View | beliebiges Gerät | Textfeld, Ausgabe, Buttons | nein |

M1 baut den Anfang des Workers: den lokalen HTTP-Kern samt Ollama-Anbindung
und Aktions-Registry, mit dem Tauri-Fenster als erstem (Option-1-)View.

| Baustein | Wahl |
|---|---|
| App-Hülle | Tauri (Rust + Web-Frontend) |
| Kern | lokaler HTTP-Service (Axum) |
| Aktionen | Registry im Code, festes Schema, feste Anzeigetexte |
| Modell lokal | Ollama, Gemma-Klasse, RAM-abhängig gewählt |
| Messaging (ab M2) | Conduit (Matrix) + mautrix-Bridges als Sidecars |
| Werkzeuge (ab M3) | MCP-Client gegen Remote-MCP-Server |

## Bekannte Einschränkung dieser Sandbox

In der Entwicklungsumgebung, in der dieser Code entstanden ist, ließen sich
die Linux-Systempakete für WebKitGTK nicht über den Paketspiegel laden (404
auf `security.ubuntu.com`). Die Rust-Kernlogik (RAM-Erkennung, Ollama-Client,
Axum-HTTP-Kern mit allen Routen inklusive Registry/Undo) wurde isoliert gegen
echte Abhängigkeiten kompiliert und lief korrekt — inklusive echter
HTTP-Roundtrips gegen `/api/status`, `/api/registry` und mehrfacher
Aktionswechsel samt Undo-Historie. Ein vollständiger `cargo tauri build`/`dev`
mit echtem Fenster wurde dort nicht verifiziert. Auf einer normalen
Linux-Arbeitsstation mit installierten Tauri-Voraussetzungen sollte
`npm run tauri dev` ohne Weiteres funktionieren.
