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

Berechtigungspflichtige („rote") Aktionen leben in `src-tauri/src/permissions.rs`
— dieselbe Idee, aber dauerhaft und mit einem echten Button statt Sofort-
Ausführung. Die erste ist `bridge.whatsapp.koppeln`: fragt der Nutzer nach
WhatsApp, ohne dass die Berechtigung erteilt ist, antwortet Iris mit dem
festen Anzeigetext aus der Registry und das Frontend zeigt einen echten
„Erlauben"-Button. Erst der Klick erteilt die Berechtigung — das Modell kann
sie nicht selbst setzen, nur anfordern.

## Status

**M1 — Kern, lokales Modell, Aktions-Registry: fertig, ungetestet am echten Gerät.**

- [x] Lokaler HTTP-Kern (Axum) mit `/api/status`, `/api/models`, `/api/registry`, `/api/permissions`, `/api/ask`
- [x] Tauri-Fenster als erster Client, spricht nur über HTTP mit dem Kern
- [x] Ein Textfeld, ein Ausgabebereich, `/help`-Hinweis auf dem Startbildschirm
- [x] RAM-Erkennung (`sysinfo`) mit Modellempfehlung (Gemma-3-Klasse: 1B / 4B / 12B je nach RAM)
- [x] Ollama-Anbindung (`/api/tags`, `/api/generate`) ohne Internet
- [x] Eine grüne Aktion (Antwortstil kurz/normal/ausführlich), per Prompt änderbar und mit echter Undo-Historie rückgängig machbar
- [x] Eine rote Aktion (WhatsApp-Kopplung anfragen/erteilen/widerrufen) mit echtem Erlauben-Button
- [ ] Im Alltag getestet: Prompt rein, lokale Antwort raus, ohne Internet

**M2 — Nachrichten lesen: Backend-Pipeline fertig, Bridge fehlt noch.**

- [x] Nachrichten-Modell mit Herkunft (`message.rs`): Besitzer/Fremd wird pro
      Nachricht mitgeführt, auch nach der Zusammenfassung
- [x] `POST /api/nachrichten` gruppiert nach Chat, baut pro Chat einen
      Zusammenfassungs-Prompt, der Fremdinhalte explizit als Daten markiert
      (nie als Anweisung), ruft Ollama auf und legt das Ergebnis im Postfach ab
- [x] Postfach (`postfach.rs`) statt Push-Benachrichtigung; `GET /api/postfach`
      und die Textfrage „Was ist heute reingekommen?" liefern denselben Stand
- [ ] **Conduit + mautrix-whatsapp als echte Sidecar-Prozesse.** Das ist der
      einzige verbleibende Baustein für M2 — und er lässt sich nicht in einer
      Sandbox ohne echtes Telefon und WhatsApp-Konto fertigstellen, weil das
      Koppeln einen echten QR-Scan mit einem echten Account braucht. Siehe
      „Nächster Schritt: echte Bridge anbinden" unten.

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

# Rote Aktion anfragen, dann erteilen (WhatsApp-Kopplung)
curl -X POST http://127.0.0.1:47615/api/ask \
  -H "Content-Type: application/json" \
  -d '{"model":"gemma3:4b","prompt":"verbinde mein whatsapp"}'
curl -X POST http://127.0.0.1:47615/api/permissions/grant \
  -H "Content-Type: application/json" \
  -d '{"id":"bridge.whatsapp.koppeln"}'

# Nachrichten einspeisen (das, was später die Bridge tut) und Postfach lesen
curl -X POST http://127.0.0.1:47615/api/nachrichten \
  -H "Content-Type: application/json" \
  -d '{"model":"gemma3:4b","nachrichten":[
        {"chat":"Familie","von":"Mama","text":"Kommst du heute?","herkunft":"fremd","zeit_unix":1}
      ]}'
curl http://127.0.0.1:47615/api/postfach
```

## Architektur (Kurzfassung)

Drei Schichten (Details in [`docs/PLAN.md`](docs/PLAN.md), Abschnitt 5):

| Schicht | Läuft auf | Aufgabe | Immer an? |
|---|---|---|---|
| Hub | Laptop (Option 1+2) oder Server (Option 3) | Bridges, Nachrichteneingang, Warteschlange | je nach Option |
| Worker | Laptop des Nutzers | Inferenz, Zusammenfassen, Entwürfe | nein |
| View | beliebiges Gerät | Textfeld, Ausgabe, Buttons | nein |

M1 baut den Anfang des Workers: den lokalen HTTP-Kern samt Ollama-Anbindung
und Aktions-Registry, mit dem Tauri-Fenster als erstem (Option-1-)View. M2
baut die Gruppierungs-/Zusammenfassungs-/Postfach-Pipeline, die eine echte
Bridge speisen wird.

| Baustein | Wahl |
|---|---|
| App-Hülle | Tauri (Rust + Web-Frontend) |
| Kern | lokaler HTTP-Service (Axum) |
| Aktionen | Registry im Code, festes Schema, feste Anzeigetexte |
| Berechtigungen | `permissions.rs`, gleiche Regel, aber dauerhaft + Button |
| Nachrichten | `message.rs` + `postfach.rs`, Herkunft bleibt immer markiert |
| Modell lokal | Ollama, Gemma-Klasse, RAM-abhängig gewählt |
| Messaging (Bridge fehlt noch) | Conduit (Matrix) + mautrix-Bridges als Sidecars |
| Werkzeuge (ab M3) | MCP-Client gegen Remote-MCP-Server |

## Nächster Schritt: echte Bridge anbinden

Das ist der einzige Teil von M2, der nicht in einer Sandbox ohne echtes
Telefon fertigzustellen ist. Konkret, in dieser Reihenfolge:

1. Conduit (bzw. den aktiv gepflegten Fork [continuwuity](https://continuwuity.org/))
   als eigenständigen Prozess besorgen (fertiges Binary/Docker-Image, nicht
   selbst bauen — siehe `docs/PLAN.md`, Abschnitt 6) und lokal starten.
2. [mautrix-whatsapp](https://github.com/mautrix/whatsapp) besorgen, mit
   Conduit als Homeserver registrieren (`registration.yaml`), starten.
3. In der App auf `/api/ask` „verbinde mein whatsapp" schreiben →
   Berechtigung erteilen (Button) → in der Bridge den QR-Code scannen, der
   beim Koppeln angezeigt wird (echtes Telefon, echter WhatsApp-Account
   nötig).
4. Sobald die Bridge Nachrichten liefert, an `POST /api/nachrichten`
   weiterreichen (kleiner Adapter, der Matrix-Timeline-Events in das
   `Nachricht`-Schema aus `message.rs` übersetzt und `herkunft` anhand des
   Matrix-Absenders setzt: eigener Account → `besitzer`, sonst `fremd`).
5. Erst danach ist M2s „Fertig, wenn"-Kriterium prüfbar: „Was ist heute
   reingekommen?" im Alltag, zwei Wochen lang.

## Bekannte Einschränkung dieser Sandbox

In der Entwicklungsumgebung, in der dieser Code entstanden ist, ließen sich
die Linux-Systempakete für WebKitGTK nicht über den Paketspiegel laden (404
auf `security.ubuntu.com`). Die Rust-Kernlogik (RAM-Erkennung, Ollama-Client,
Axum-HTTP-Kern mit allen Routen inklusive Registry/Undo, Berechtigungen und
Nachrichten-Pipeline) wurde isoliert gegen echte Abhängigkeiten kompiliert
und lief korrekt — inklusive echter HTTP-Roundtrips gegen `/api/status`,
`/api/registry`, `/api/permissions` (Anfragen, erfundene IDs werden mit 400
abgewiesen, Erteilen/Widerruf per Prompt), mehrfacher Aktionswechsel samt
Undo-Historie, und der Nachrichten-Pipeline (Gruppierung nach Chat,
Herkunfts-Markierung im Zusammenfassungs-Prompt, sauberer 502 bei fehlendem
Ollama ohne Teilzustand im Postfach). Ein vollständiger `cargo tauri
build`/`dev` mit echtem Fenster wurde dort nicht verifiziert, und Conduit /
mautrix-whatsapp wurden nicht real gestartet — Letzteres braucht ohnehin ein
echtes Telefon zum Koppeln, das in keiner Sandbox existiert. Auf einer
normalen Linux-Arbeitsstation mit installierten Tauri-Voraussetzungen sollte
`npm run tauri dev` ohne Weiteres funktionieren.
