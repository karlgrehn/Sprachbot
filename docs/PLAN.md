# Iris — Umsetzungsplan v2

*Briefing für Claude Code. Stand: 30.07.2026*

---

## 1. Was Iris ist

Ein persönlicher Agent. **Ein einziges Textfeld** steuert alles. Iris bündelt Nachrichten aus allen Messengern und erledigt Aufgaben über angebundene Dienste.

**Zielmetrik:** Gesamte Gerätezeit des Nutzers minimieren. Nicht Antwortlatenz, nicht Funktionsumfang.

**Gegner ist das Smartphone, nicht Claude.**

**Zielgruppe:** Erwachsene. Kinder sind vorerst ausgeschlossen — bewusste Entscheidung wegen Einwilligungspflicht, Altersgrenzen der Modellanbieter und Zugriff auf fremde Nachrichteninhalte.

---

## 2. Drei Schichten

Der Kern der Architektur. Alles andere folgt daraus.

| Schicht | Läuft auf | Aufgabe | Immer an? | Kosten |
|---|---|---|---|---|
| **Hub** | Pi oder kleine Cloud-Instanz | Bridges, Nachrichteneingang, Warteschlange | ja | Cent-Bereich, fix |
| **Worker** | Laptop des Nutzers (Fat Client) | Inferenz, Zusammenfassen, Entwürfe, Werkzeugaufrufe | nein | null |
| **View** | beliebiges Gerät (Thin Client) | Anzeige und Eingabe | nein | null |

**Die entscheidende Idee:** Der Fat Client des Nutzers ist der Rechenknoten für seine *eigenen* Thin Clients. Sein Nokia spricht mit seinem eigenen Laptop, nicht mit einem fremden Server.

Der Hub ist absichtlich dumm: Er nimmt Nachrichten an, legt sie in eine Warteschlange, hält Verbindungen. Er denkt nicht. Deshalb kostet er fast nichts und deshalb kann er verschenkt werden.

Ist der Laptop aus, staut sich die Warteschlange. Beim nächsten Einschalten wird sie abgearbeitet. **Das ist kein Fehler, sondern beabsichtigt** — die Zielmetrik ist Gerätezeit, nicht Latenz. Eine Zusammenfassung, die zwei Stunden später fertig ist, verliert nichts.

Optional als Bezahlmerkmal: Fällt der Worker dauerhaft aus, übernimmt ein Cloud-Modell mit dem Key des Nutzers.

---

## 3. Nur eine Client-Klasse bauen?

Ja — und zwar zuerst **nur Fat Client**.

Aber mit einer Bedingung, die jetzt entschieden werden muss und später nicht mehr nachrüstbar ist:

> **Die gesamte Logik liegt in einem lokalen HTTP-Service. Die Desktop-Oberfläche ist nur ein Client davon.**

Kein Monolith, in dem UI und Logik verwoben sind. Wenn diese Grenze von Anfang an sauber ist, ist der Thin Client später **dieselbe Web-Oberfläche über Tailscale** — ein paar Tage Arbeit statt einer Neuentwicklung.

Wird stattdessen ein Monolith gebaut, kostet iOS-Unterstützung später eine Umschreibung des ganzen Kerns.

---

## 4. Plattformen

| Plattform | Klasse | Weg | Status |
|---|---|---|---|
| Windows | Fat | Tauri | M1 |
| Linux | Fat | Tauri | M1 |
| macOS | Fat | Tauri | eingefroren — braucht Developer-Account und Signier-Mac |
| iOS | Thin | **PWA**, keine native App | M6 |
| Android | Thin | PWA | M6 |
| Nokia / KaiOS | Thin | Web-UI oder SMS-Brücke | M7 |
| BlackBerry | — | existiert nicht mehr, Dienste 2022 abgeschaltet; letzte Modelle waren Android | entfällt |

**iOS bekommt keine native App.** Die Sandbox erlaubt weder dauerhafte Hintergrundprozesse noch mitgelieferte Sidecar-Binaries — Conduit und die Bridges können dort nicht laufen. Eine responsive Web-Oberfläche, als PWA auf den Homescreen gelegt, ersetzt vier Plattform-Portierungen und braucht weder Review noch Mac noch Developer-Gebühr.

---

## 5. Technische Festlegungen

| Baustein | Wahl | Begründung |
|---|---|---|
| App-Hülle | Tauri | kleines Bundle, Sidecar-Prozesse, Windows + Linux aus einer Codebasis |
| Kern | lokaler HTTP-Service | Voraussetzung für spätere Thin Clients |
| Messaging | Conduit + mautrix-Bridges als Sidecars | existiert und wird gepflegt — nicht selbst bauen |
| Werkzeuge | **MCP-Client** gegen Remote-Server | offenes Protokoll, gesamtes Ökosystem ohne eigene Konnektoren |
| Modell lokal | Ollama, Gemma- oder 7–8B-Klasse in 4-Bit | kostenlos, offline, deckt Stufe 1–2 |
| Modell Cloud | BYOK | keine Kosten und keine Haftung auf Betreiberseite |
| Fernzugriff | Tailscale | ohne Hosting, ohne Portfreigabe |
| Vault | verschlüsselt, lokal | API-Keys und Zugangsdaten |

---

## 6. Nicht-Ziele

Nicht anfangen, auch nicht nebenbei:

- **Selbstmodifikation des Kerns.** Später ausschließlich als Plugin-Generierung im Sandbox-Ordner, mit Testsuite als Freigabe-Gate und Git-Commit pro Generation. Niemals am laufenden Kern.
- **GUI-Automatisierung.** Nur letzter Fallback, wenn API und Bridge fehlen.
- **Native iOS-App.**
- **macOS.**
- **Push-Benachrichtigungen.** Ergebnisse landen im Postfach, das der Nutzer öffnet.
- **Kinder als Zielgruppe.**

---

## 7. Ausbaustufen

Jede Stufe ist eigenständig benutzbar. Nicht weitergehen, bevor die vorige täglich im Einsatz ist.

**M1 — Kern und lokales Modell**
Lokaler HTTP-Service. Tauri-Fenster als erster Client. Ein Textfeld, ein Ausgabebereich. Ollama-Anbindung mit Modellwahl nach erkanntem RAM.
*Fertig, wenn:* Prompt rein, lokale Antwort raus, ohne Internet — und die Oberfläche spricht ausschließlich über die HTTP-Schnittstelle mit dem Kern.

**M2 — Nachrichten lesen (der Wertbeweis)**
Conduit als Sidecar, dazu **eine** Bridge (WhatsApp), Kopplung per QR im Onboarding. Nur lesen: holen, gruppieren, lokal zusammenfassen. Ergebnis-Postfach statt Benachrichtigung.
*Fertig, wenn:* „Was ist heute reingekommen?" liefert eine brauchbare Übersicht, ohne dass ein Messenger geöffnet wird.
**Hier mindestens zwei Wochen im täglichen Eigengebrauch bleiben.** Vorher ist jede weitere Entscheidung eine Vermutung.

**M3 — MCP-Client**
Client-Spezifikation, OAuth-Flow, Server-Registry in den Einstellungen. Start mit einem Remote-Server als Referenz.
*Fertig, wenn:* ein fremder MCP-Server ohne Codeänderung eingebunden und benutzt werden kann.

**M4 — Hub und Warteschlange**
Bridges vom Laptop auf den Pi verlagern. Warteschlange, die der Worker abarbeitet, sobald er da ist. Ab hier ist Iris nicht mehr an einen eingeschalteten Laptop gebunden.

**M5 — Entwerfen und Senden**
Antwortentwürfe lokal. Senden **nur nach ausdrücklicher Bestätigung**. Modell-Routing lokal/Cloud. Hartes Aufruflimit pro Aufgabe gegen Endlosschleifen.

**M6 — Thin Clients**
Responsive Web-UI über Tailscale, als PWA installierbar. Zwei Modi: Worker erreichbar → voller Funktionsumfang; Worker offline → nur Warteschlange und Postfach, mit klarer Ansage, was gerade nicht geht.

**M7 — Weitere Bridges und Nokia**
Signal, Telegram, Discord, Slack. Nokia-Anbindung. Jede Bridge kostet Onboarding-Aufwand und Bundle-Größe.

---

## 8. Geld

**Kostenlos, dauerhaft, ohne Konto:** Messenger gebündelt, lesen, sortieren, zusammenfassen, Entwürfe, Ergebnis-Postfach — alles lokal auf dem Gerät des Nutzers. Kostet den Betreiber null, weil es auf fremder Hardware läuft.

**Kostenpflichtig:** dass Iris *handelt* statt berichtet. Mehrstufige Aufgaben, Dienste-Anbindungen, Cloud-Reasoning bei abwesendem Worker, Thin-Client-Betrieb ohne eigenen Laptop.

Die Grenze ist keine künstliche Beschränkung, sondern die tatsächliche Kostengrenze: Berichten läuft lokal, Handeln braucht fremde Rechenzeit.

**Zwei Regeln, ohne die es nicht trägt:**

1. **BYOK auch für Thin-Client-Nutzer.** Der Hub ruft die API mit *ihrem* Key auf. Damit bleibt beim Betreiber nur die Hub-Kosten, die planbar und klein sind.
2. **Freiplätze zahlenmäßig begrenzen, nicht nach Kategorie.** „Ich finanziere zehn Plätze" ist ein Budget. „Alle Nokias gratis" ist ein Blankoscheck, dessen Höhe die Nutzer bestimmen.

**Abrechnung:** Erst ab M5 relevant. Dann Stripe Billing mit Abo plus Verbrauchskomponente und hartem Deckel pro Nutzer. Der Deckel ist nicht optional — er ist der Schutz gegen einen einzelnen Nutzer, dessen Agent in einer Schleife die Monatsrechnung verbrennt.

---

## 9. Vor dem ersten fremden Nutzer zu klären

- **AGPL.** Die mautrix-Bridges stehen unter AGPL-3.0. Als Dienst angeboten greift die Netzwerkklausel — eigener Quellcode muss offengelegt werden. Lizenzlage prüfen, bevor jemand aufgeschaltet wird.
- **Datenschutz.** Mit fremden Nachrichten auf eigener Hardware wird man Verantwortlicher im Sinne der DSGVO.
- **Die Sensibilitätsgrenze ist eine Architekturgrenze, keine Richtlinie.** Fat Client: Nachrichten verlassen das Gerät nie, der Betreiber kann nichts einsehen. Thin Client über fremden Hub: Inhalte liegen dort im Klartext vor, damit ein Modell sie lesen kann. Das ist der Preis für iOS und Nokia und sollte ausgesprochen werden.
- **Weiterverkauf.** Fremde MCP-Server gehören ihren Betreibern. Iris kann sich verbinden, nicht mitverkaufen.

---

## 10. Was zuerst zu tun ist

**M1 und M2. Nichts anderes.**

Die eine Entscheidung, die dabei nicht verschoben werden darf: **Logik im HTTP-Service, Oberfläche als Client.** Alles andere in diesem Dokument lässt sich später ändern. Diese Grenze nicht.
