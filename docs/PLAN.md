# Iris — Umsetzungsplan

*Briefing für Claude Code. Stand: 30.07.2026*

---

## 1. Was Iris ist

Ein persönlicher Agent als Desktop-App. **Ein einziges Textfeld** steuert alles. Iris bündelt Nachrichten aus allen Messengern und erledigt Aufgaben über angebundene Dienste.

**Zielmetrik:** Gesamte Gerätezeit des Nutzers minimieren. Nicht Antwortlatenz, nicht Funktionsumfang.

**Gegner ist das Smartphone, nicht Claude.** Jede Entscheidung wird daran gemessen, ob sie dem Nutzer Zeit zurückgibt.

---

## 2. Architektur — festgelegt

| Baustein | Wahl | Begründung |
|---|---|---|
| App-Hülle | Tauri (Rust + Web-Frontend) | kleines Bundle, Sidecar-Prozesse, Windows + Linux aus einer Codebasis |
| Messaging | Conduit (Matrix-Homeserver) + mautrix-Bridges, beide als gebündelte Sidecars | Bridges existieren und werden gepflegt — nicht selbst bauen |
| Werkzeuge/Dienste | **MCP-Client** gegen Remote-MCP-Server | offenes Protokoll, gesamtes Ökosystem ohne eigene Konnektoren |
| Modell lokal | Ollama, Gemma- oder 7–8B-Klasse in 4-Bit | kostenlos, offline, deckt Stufe 1–2 |
| Modell Cloud | BYOK — Nutzer hinterlegt eigenen Key | keine Kosten und keine Haftung auf Betreiberseite |
| Fernzugriff | Tailscale auf lokale Web-UI | „von überall" ohne Hosting, ohne Portfreigabe |
| Dauerbetrieb | Raspberry Pi als Nachrichten-Hub | Matrix + Bridges brauchen kaum CPU, aber 24/7 |

**Der Pi rechnet nicht.** Er hält Verbindungen. Inferenz läuft auf dem Laptop des Nutzers oder in der Cloud.

---

## 3. Nicht-Ziele — bewusst ausgeschlossen

Diese Punkte sind **nicht** Teil der ersten Ausbaustufen. Nicht anfangen, auch nicht „schnell nebenbei":

- **Selbstmodifikation des Kerns.** Später als Plugin-Generierung in einem Sandbox-Ordner, mit Testsuite als Freigabe-Gate und Git-Historie pro Generation. Niemals Änderungen am laufenden Kern.
- **GUI-Automatisierung.** Nur als letzter Fallback, wenn API und Bridge beide fehlen. Nicht als Fundament.
- **macOS.** Eingefroren bis Developer-Account und Signier-Mac vorhanden sind.
- **Nokia/SMS-Interface.** Setzt vollständigen Serverbetrieb voraus und macht den kostenfreien Tier unmöglich (siehe Abschnitt 5).
- **Push-Benachrichtigungen.** Ergebnisse landen in einem Postfach, das der Nutzer öffnet, wenn er will.

---

## 4. Ausbaustufen

Jede Stufe ist eigenständig benutzbar. Nicht mit der nächsten anfangen, bevor die vorige täglich im Einsatz ist.

### M1 — Hülle und lokales Modell
Tauri-App, ein Textfeld, ein Ausgabebereich. Ollama-Anbindung mit Modellwahl nach erkanntem RAM. Kein Messaging, keine Werkzeuge.

*Fertig, wenn:* Prompt rein, lokale Modellantwort raus, ohne Internet.

### M2 — Nachrichten lesen (Stufe 1, der eigentliche Wertbeweis)
Conduit als Sidecar bündeln, dazu **eine** Bridge (WhatsApp). Kopplung per QR-Code im Onboarding. Nur lesen: Nachrichten holen, gruppieren, lokal zusammenfassen.

*Fertig, wenn:* „Was ist heute reingekommen?" liefert eine brauchbare Übersicht, ohne dass irgendein Messenger geöffnet wird.

Dies ist der Punkt, an dem Iris zum ersten Mal echte Zeit spart. Hier mindestens zwei Wochen im täglichen Eigengebrauch bleiben, bevor es weitergeht.

### M3 — MCP-Client
Client-Spezifikation implementieren, OAuth-Flow, Server-Registry in den Einstellungen. Start mit **einem** Remote-Server als Referenz.

*Fertig, wenn:* ein fremder MCP-Server ohne Codeänderung eingebunden und benutzt werden kann.

### M4 — Zwei Modi und Fernzugriff
Ein Client, unterschiedliche Server-Registrierung: Gerät erreichbar → lokale Werkzeuge plus Remote; Gerät offline → nur Remote. Iris benennt fehlende Fähigkeiten explizit. Tailscale-Anleitung im Onboarding.

### M5 — Entwerfen und Senden (Stufe 2–3)
Antwortentwürfe lokal. Senden nur nach ausdrücklicher Bestätigung. Modell-Routing: lokal für einfache Fälle, Cloud für komplexe. BYOK-Key-Verwaltung im verschlüsselten Vault. Hartes Aufruflimit pro Aufgabe gegen Endlosschleifen.

### M6 — Pi-Hub für wenige Nutzer
Conduit und Bridges auf den Pi. Mehrbenutzer-Trennung. Vorher Abschnitt 6 lesen.

### M7 — Weitere Bridges
Signal, Telegram, Discord, Slack. Erst hier, nicht früher — jede Bridge ist Onboarding-Aufwand und Bundle-Größe.

---

## 5. Der Widerspruch im Geschäftsmodell

**Kostenfreie Stufe 1–2 funktioniert nur, wenn das Modell auf der Hardware des Nutzers läuft.**

Ein Nokia-Nutzer hat keine Hardware. Bei ihm läuft jede Zusammenfassung serverseitig, also kostenpflichtig — auch Stufe 1. Damit sind „gratis" und „Nokia" nicht gleichzeitig möglich.

Zwei saubere Nutzergruppen:

| | Desktop | Nokia/SMS |
|---|---|---|
| Inferenz Stufe 1–2 | lokal | Server |
| Kosten für den Betreiber | null | dauerhaft |
| Tier | kostenfrei möglich | nur im Abo |

**Empfehlung:** Nokia-Betrieb als kostenpflichtiges Merkmal führen, nicht als Zugangsvariante. Sonst subventionieren die zahlenden Nutzer eine Gruppe, die strukturell nie kostenlos sein kann.

Vor jeder Quersubventionierung die Kosten pro kostenfreiem Nutzer pro Monat ausrechnen. Bei 5 % Umwandlungsquote trägt jeder zahlende Nutzer rund zwanzig kostenfreie.

---

## 6. Zu klären, bevor gehostet wird

- **AGPL.** Die mautrix-Bridges stehen unter AGPL-3.0. Als Dienst angeboten greift die Netzwerkklausel — der eigene Quellcode muss offengelegt werden. Lizenzlage prüfen, bevor der erste fremde Nutzer aufgeschaltet wird.
- **Datenschutz.** Mit fremden Nachrichten auf eigener Hardware wird man Verantwortlicher im Sinne der DSGVO. Keine Randnotiz.
- **AWS.** Für den Hub taugt eine kleine CPU-Instanz und kostet wenig. GPU-Instanzen für Inferenz kosten pro Monat ein Vielfaches und kommen für einen kostenfreien Tier nicht in Frage. AWS ändert nichts an der Rechnung, es verschiebt sie nur.
- **Weiterverkauf.** Fremde MCP-Server gehören ihren Betreibern. Iris kann sich verbinden — mitverkaufen kann sie nichts. Für den Weiterverkauf von API-Zugang gelten die Bedingungen des jeweiligen Anbieters; vor dem Verkaufsstart lesen.

---

## 7. Was zuerst zu tun ist

**M1 und M2.** Nichts anderes.

Erst wenn Iris zwei Wochen lang täglich die eigenen Nachrichten zusammenfasst und dabei spürbar Zeit spart, steht fest, welche Erweiterung sich lohnt. Vorher ist jede Entscheidung über Abo-Stufen, Nokia-Anbindung und Selbstverbesserung eine Vermutung.
