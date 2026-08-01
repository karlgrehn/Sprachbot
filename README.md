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
Toggle-Hack). `/help` läuft wie jeder andere Befehl über `/api/ask` (erkannt
im Kern, nicht im Frontend — sonst würde es über einen Messenger-Kanal in
M4 nicht funktionieren) und setzt sich aus Registry, Berechtigungen und
verbundenen MCP-Servern zusammen, nie aus dem Modellgedächtnis.

Berechtigungspflichtige („rote") Aktionen leben in `src-tauri/src/permissions.rs`
— dieselbe Idee, aber dauerhaft und mit einem echten Button statt Sofort-
Ausführung. Die erste ist `bridge.whatsapp.koppeln`: fragt der Nutzer nach
WhatsApp, ohne dass die Berechtigung erteilt ist, antwortet Iris mit dem
festen Anzeigetext aus der Registry und das Frontend zeigt einen echten
„Erlauben"-Button. Erst der Klick erteilt die Berechtigung — das Modell kann
sie nicht selbst setzen, nur anfordern. Die zweite ist `mcp.server.verbinden`:
der Anzeigetext ist immer derselbe, nur der `scope` (die Server-URL) ist frei
— genau das `geltung: { chat_id }`-Schema aus dem Plan, nur mit einer URL
statt einer Chat-ID.

## MCP-Client (M3)

`src-tauri/src/mcp.rs` spricht MCP über den "Streamable HTTP"-Transport:
Initialize-Handshake, `tools/list`, `tools/call`, Session-Header. Einen
Server zu verbinden ist die rote Aktion oben — sagt der Nutzer „verbinde
mich mit https://…", fragt Iris die Berechtigung an; ist sie erteilt, folgt
die Verbindung sofort und die gefundenen Werkzeuge landen in der Antwort.

Verlangt der Server eine Autorisierung (401 mit `WWW-Authenticate`), läuft
`src-tauri/src/oauth.rs` den vollen OAuth-2.0-Authorization-Code-Fluss mit
PKCE: Entdeckung der Endpunkte (RFC 9728 + RFC 8414), Systembrowser öffnen,
Code über einen kurzlebigen lokalen Redirect-Listener auf Port 47616
abfangen, gegen ein Token tauschen.

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

**M3 — MCP-Client: fertig, gegen einen selbstgebauten Referenzserver
end-to-end verifiziert.**

- [x] Initialize-Handshake, `tools/list`, `tools/call`, Session-Header
      (`mcp.rs`) über den Streamable-HTTP-Transport
- [x] Server hinzufügen ist eine rote Aktion (`mcp.server.verbinden`), scope
      = Server-URL; Erteilen verbindet sofort und zeigt die Werkzeuge
- [x] Voller OAuth-2.0-Authorization-Code-Fluss mit PKCE (`oauth.rs`):
      RFC-9728/8414-Entdeckung, Browser öffnen, lokaler Redirect-Listener,
      Token-Tausch — inklusive Negativtest (falscher PKCE-Verifier wird vom
      Server abgelehnt)
- [ ] **Gegen einen echten dritten MCP-Server verifiziert.** Getestet wurde
      gegen einen selbstgebauten Referenzserver (siehe unten) — das erfüllt
      "ein *fremder* MCP-Server" nur im Sinne von "der Client kennt seine
      Implementierung nicht", nicht im Sinne von "ein echter Betreiber hat
      das genutzt". Ein echter dritter Server (mit oder ohne OAuth) ist der
      nächste sinnvolle Test.

M1 gilt erst als fertig, wenn das auf einem echten Rechner mit installiertem
Ollama getestet wurde — das kann in dieser Sandbox nicht verifiziert werden
(siehe Abschnitt „Bekannte Einschränkung dieser Sandbox" unten).

**Thin Clients (Android/iOS/Desktop-Installation): fertig, ungetestet auf
echten Mobilgeräten.** Vorgezogen aus M6, auf Wunsch.

- [x] `public/manifest.json` + `public/sw.js`: dieselbe Oberfläche ist als
      PWA installierbar — auf Android/iOS per „Zum Home-Bildschirm", auf
      Desktop-Chrome/Edge per Install-Button in der Adressleiste. Das *ist*
      die "Installationsversion" fürs Laptop, zusätzlich zur nativen
      `.exe`/`.AppImage` aus dem Release-Build.
- [x] Der Kern liefert die gebaute Oberfläche jetzt selbst aus
      (`tower_http::services::ServeDir` als Fallback in `api.rs`) — ein
      Handy, das über Tailscale denselben Port erreicht, bekommt Oberfläche
      und API von einer Adresse, ohne eigenen Webserver
- [x] `API_BASE` im Frontend ist nicht mehr fest auf `127.0.0.1` verdrahtet,
      sondern ergibt sich aus dem Host, von dem die Seite selbst geladen
      wurde — kein Einstellungsfeld für die Server-Adresse nötig
- [x] Mit echtem Playwright/Chromium end-to-end getestet: Seite lädt vom
      Kern, Manifest verlinkt, Service Worker registriert sich wirklich,
      `/help` funktioniert im echten Browser
- [ ] **Auf einem echten Android-/iOS-Gerät installiert und benutzt.** Kann
      keine Sandbox der Welt prüfen — Safari/Chrome auf echter Hardware,
      echtes „Zum Home-Bildschirm hinzufügen" nötig.
- [ ] **Tailscale tatsächlich eingerichtet und ein Handy hat den Laptop
      darüber erreicht.** Siehe „Thin Client einrichten" unten für die
      genauen Schritte.

## Download / Release-Build

`.github/workflows/release.yml` baut auf echten GitHub-Runnern (Windows +
Linux) — kein Sandbox-Build kann eine echte Windows-`.exe` erzeugen (kein
Windows-Toolchain) oder auch nur den Linux-Build durchlaufen lassen
(fehlendes WebKitGTK, siehe unten).

Auslösen:
- **Manuell**: Im GitHub-Repo unter „Actions" → „Release" → „Run workflow".
- **Per Tag**: `git tag v0.1.0 && git push origin v0.1.0`.

### Ein Download pro Plattform

Der Workflow baut intern zwei Varianten (`normal`: nur die App, setzt eine
separat installierte Ollama voraus; `offline`: App + Ollama + Modell
(`gemma3:1b`) als Sidecar, läuft ohne Internet). Die Landingpage
(`web/index.html`) zeigt davon aber bewusst nur **einen** Button pro
Plattform — die Offline-Variante, weil sie ohne weitere Entscheidung oder
Installation einfach funktioniert. Eine Normal/Offline-Auswahl wäre für
die breite Nutzerschaft unnötige Komplexität; das Manifest enthält beide
Einträge weiterhin (für einen möglichen späteren „erweiterte Optionen"-Link),
aktuell wird nur `offline` verlinkt.

### Repo bleibt privat — Downloads laufen über eigenen Objektspeicher

Auf Wunsch bleibt der Quellcode privat (`docs/PLAN.md`: „Closed Source.
Ausgeliefert werden Binaries, kein Quellcode.") — GitHub Releases sind bei
privaten Repos nicht öffentlich abrufbar, deshalb lädt der Workflow beide
Varianten stattdessen zu einem **S3-kompatiblen Objektspeicher** hoch
(AWS S3, Cloudflare R2, Backblaze B2, o. Ä. — deine Wahl, dein Konto; das
kann diese Sitzung nicht für dich einrichten). Nötige Repo-Secrets:

| Secret | Bedeutung |
|---|---|
| `S3_BUCKET` | Bucket-Name |
| `S3_ACCESS_KEY_ID` / `S3_SECRET_ACCESS_KEY` | Zugangsdaten |
| `S3_ENDPOINT` | Leer lassen für echtes AWS S3; sonst die Endpunkt-URL (z. B. R2) |
| `S3_PUBLIC_BASE_URL` | Öffentliche Basis-Adresse, unter der Dateien abrufbar sind |

Ohne diese Secrets läuft der Build durch, der Upload-Schritt wird nur
übersprungen. **Wichtig:** die Bucket-Berechtigung für öffentliches Lesen
muss auf Bucket-Ebene eingerichtet werden (Bucket-Policy / „Public Bucket"-
Schalter je nach Anbieter) — das Skript setzt keine Objekt-ACLs, weil R2
(anders als AWS S3) diese gar nicht unterstützt.

Nach jedem Build liegen unter `<S3_PUBLIC_BASE_URL>/iris/manifest-windows.json`
und `.../manifest-linux.json` die aktuellen Download-Adressen + Dateigrößen
für beide Varianten — die Landingpage liest genau das, keine fest
verdrahteten Dateinamen.

**Um das Repo tatsächlich auf privat zu stellen:** GitHub → Settings →
General → Danger Zone → „Change visibility". Das kann keines meiner
Werkzeuge für dich übernehmen.

### Vercel: nur die Landingpage, nicht die App selbst

Vercel baut und hostet Web-Apps/statische Seiten, keine nativen
Desktop-Programme. `web/index.html` ist eine schlichte, fertige
Download-Landingpage (reines HTML/CSS, kein Build-Schritt, **kein Link auf
das Quellcode-Repository**) mit einem einzigen Download-Button pro
Plattform (kein Normal/Offline-Entscheidungszwang für die Nutzerschaft).
Um sie auf Vercel zu deployen:

1. Vercel-Connector unter den claude.ai-Verbindungseinstellungen
   autorisieren (das kann diese Sitzung nicht selbst tun).
2. Repo in Vercel importieren, **Root Directory** auf `web` setzen,
   kein Build-Command nötig — oder die Datei direkt hochladen, wenn das
   Repo privat ist und nicht mit Vercel verknüpft werden soll.
3. In `web/index.html` die Konstante `DOWNLOAD_BASE` auf
   `S3_PUBLIC_BASE_URL` setzen, sobald der Objektspeicher eingerichtet ist
   — vorher bleibt der Download-Button bewusst deaktiviert
   („Noch kein Download eingerichtet").

### Behobener Build-Fehler: `shell:allow-execute` in beiden Varianten

Der erste echte CI-Lauf der Offline-Variante (Run #3) schlug auf **beiden**
Plattformen schon beim normalen Build fehl: `Permission shell:allow-execute
not found`. Ursache: Tauris Build-Skript validiert alle Dateien unter
`capabilities/` gegen die Permissions der tatsächlich kompilierten Plugins —
unabhängig davon, ob die jeweilige Capability-Datei über
`security.capabilities` überhaupt aktiv ist. `tauri-plugin-shell` war als
optionale Abhängigkeit hinter dem Feature `offline-bundle` versteckt; im
normalen Build (ohne dieses Feature) kannte das Build-Skript die Permission
aus `capabilities/offline.json` deshalb gar nicht. Behoben, indem
`tauri-plugin-shell` eine normale (immer kompilierte) Abhängigkeit ist —
nur die tatsächliche Sidecar-Ausführung (`ollama_sidecar::spawn`) bleibt
hinter dem Feature.

### Offener Punkt bei der Offline-Variante

Ollama sucht seine nativen Laufzeitbibliotheken relativ zur eigenen
Programmdatei (`<exe_dir>/lib/ollama` unter Windows,
`<exe_dir>/../lib/ollama` unter Linux — keine verlässliche
Umgebungsvariable dafür, siehe [ollama/ollama#13535](https://github.com/ollama/ollama/issues/13535)).
Der Workflow bündelt dieses `lib`-Verzeichnis als Tauri-Ressource, aber ob
es nach dem Bündeln tatsächlich an der richtigen Stelle relativ zum
Sidecar landet, ist **nicht auf einem echten Gerät bestätigt** — dafür
gibt es im Workflow einen Diagnose-Schritt („Gestagte Verzeichnisstruktur
prüfen"), dessen Ausgabe nach dem ersten Lauf zeigt, ob eine Korrektur
nötig ist.

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

# MCP-Server verbinden (rote Aktion), dann Werkzeug aufrufen
curl -X POST http://127.0.0.1:47615/api/ask \
  -H "Content-Type: application/json" \
  -d '{"model":"gemma3:4b","prompt":"verbinde mich mit https://beispiel.de/mcp"}'
curl -X POST http://127.0.0.1:47615/api/permissions/grant \
  -H "Content-Type: application/json" \
  -d '{"id":"mcp.server.verbinden","scope":"https://beispiel.de/mcp"}'
curl http://127.0.0.1:47615/api/mcp/servers
curl -X POST http://127.0.0.1:47615/api/mcp/tools/call \
  -H "Content-Type: application/json" \
  -d '{"url":"https://beispiel.de/mcp","tool":"irgendein_werkzeug","arguments":{}}'
```

## Thin Client einrichten (Android, iOS, weiteres Desktop-Gerät)

Kein eigener Server nötig — der Kern liefert die Oberfläche selbst mit
(siehe Status oben). Auf dem Worker-Laptop, auf dem Iris bereits läuft:

1. [Tailscale](https://tailscale.com/) installieren und anmelden (auf dem
   Laptop und auf dem Handy/Zweitgerät, im selben Tailnet).
2. Auf dem Laptop, während Iris läuft:
   ```bash
   tailscale serve --bg 47615
   ```
   Das reicht `127.0.0.1:47615` verschlüsselt über das Tailnet durch —
   Iris selbst muss dafür an keiner zusätzlichen Netzwerkschnittstelle
   lauschen. `tailscale serve status` zeigt die resultierende Adresse
   (etwas wie `https://<laptopname>.<tailnet>.ts.net`).
3. Auf dem Handy: diese Adresse im Browser öffnen, dann
   „Zum Home-Bildschirm hinzufügen" (iOS Safari) bzw. den
   Installieren-Hinweis von Chrome (Android) bestätigen.
4. Auf einem weiteren Laptop/Desktop genügt der normale
   Install-Button in der Adressleiste (Chrome/Edge) — das ist zugleich
   die "Installationsversion" für Rechner, auf denen die native App
   nicht installiert werden soll.

**Ohne eigene Authentifizierung:** Wer die Tailnet-Adresse erreicht, kann
den Kern benutzen — Tailscales eigene Geräte-ACLs sind die Sicherheitsgrenze,
nicht Iris selbst (docs/PLAN.md, Abschnitt 7: „Fernzugriff: Tailscale").
Das folgt bewusst der Vorgabe „ohne Hosting, ohne Portfreigabe".

**Nicht in dieser Sandbox verifizierbar:** dass ein echtes Telefon per
Tailscale tatsächlich verbindet und die Installation auf echtem iOS/Android
funktioniert. Getestet wurde die Web-Seite selbst (Manifest, Service
Worker, gleiche Oberfläche vom Kern ausgeliefert) mit echtem
Playwright/Chromium — das Verhalten von Safaris „Zum Home-Bildschirm" oder
Chromes Install-Prompt auf echter Hardware kann nur ein echtes Gerät zeigen.

**Bewusst nicht gebaut: Handys als Fat Client.** Ein Handy mit genug Power
könnte technisch selbst rechnen (siehe `docs/PLAN.md`, Abschnitt 3:
Geräteerkennung über Tokens/Sekunde, nicht über Gerätetyp) — das würde aber
eine echte native Android/iOS-App bedeuten (Tauri Mobile), für iOS zwingend
einen Mac mit Xcode und Apple-Entwickler-Konto, und eine ungeklärte Frage,
wie Ollama überhaupt in einer mobilen App-Sandbox laufen soll. Auf
ausdrücklichen Wunsch zurückgestellt — Handys bleiben vorerst Thin Clients.

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
| Werkzeuge | MCP-Client (`mcp.rs`) + OAuth/PKCE (`oauth.rs`) gegen Remote-MCP-Server |

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
echtes Telefon zum Koppeln, das in keiner Sandbox existiert.

Für M3 galt dieselbe Grenze in die andere Richtung: es gibt keinen Zugang zu
einem echten dritten MCP-Server mit echten OAuth-Zugangsdaten. Verifiziert
wurde deshalb gegen zwei selbstgebaute, minimale Referenzserver (Node,
`http`-Modul, keine Abhängigkeiten) — einer offen, einer mit erzwungener
Autorisierung. Real getestet, alles über echtes HTTP, echte JSON-RPC-
Nachrichten, echte PKCE-Kryptografie: Initialize/Notifications/Liste/Aufruf
gegen den offenen Server; 401-Erkennung, RFC-9728/8414-Entdeckung, ein
echter lokaler Redirect-Listener, der einen echten HTTP-Redirect abfängt,
Token-Tausch, und ein Negativtest mit falschem PKCE-Verifier, den der
Referenzserver korrekt ablehnt. Dabei kam ein echter Bug ans Licht (ein
Rust-Borrow-Checker-Fehler durch einen partiellen Move in `mcp.rs`), der vor
diesem Test unbemerkt geblieben wäre. Was so ein Test nicht ersetzen kann:
einen echten Systembrowser, der einen echten Nutzer zur Zustimmung zeigt,
und einen echten Betreiber, der Iris als "fremden" Client akzeptiert. Auf
einer normalen Linux-Arbeitsstation mit installierten Tauri-Voraussetzungen
sollte `npm run tauri dev` ohne Weiteres funktionieren.

Für die Thin Clients (PWA) galt die Grenze umgekehrt zu sonst: das
vorinstallierte, headless Chromium in dieser Sandbox erlaubte zum ersten
Mal einen echten Browser-Test der Oberfläche selbst. `npm run build` hat
das reale `dist/` erzeugt, der reale `api.rs`-Kern (nicht Tauri — die
Rust-Logik hängt nirgends vom `tauri`-Crate ab) hat es über
`ServeDir`-Fallback ausgeliefert, und Playwright hat gegen `127.0.0.1:47615`
genau das nachgestellt, was ein Handy über Tailscale sähe: Titel korrekt,
Manifest verlinkt und gültig, Service Worker registriert sich wirklich,
`/help` in echtem Chromium eingetippt und die echte Antwort im DOM
geprüft. Dabei kam ein echter UX-Fehler ans Licht — der Senden-Button war
ohne laufendes Ollama komplett gesperrt, obwohl `/help` und alle anderen
Registry-/Berechtigungs-Befehle kein Ollama brauchen — behoben. Nicht
verifizierbar bleiben echtes Tailscale-Netzwerk, echtes iOS/Android-Gerät
und das jeweilige "Zum Home-Bildschirm hinzufügen".
