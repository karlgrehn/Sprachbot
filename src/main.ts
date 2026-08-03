// Iris-Oberfläche: spricht ausschließlich über HTTP mit dem lokalen Kern
// (src-tauri/src/api.rs), nie über Tauri-`invoke`. Damit ist dieselbe
// Oberfläche unverändert als Thin Client nutzbar (docs/PLAN.md, Abschnitt 3):
// im Tauri-Fenster läuft sie von "localhost", auf einem Handy über Tailscale
// vom Hostnamen/der IP des Worker-Laptops — in beiden Fällen läuft der Kern
// auf demselben Host wie die Seite selbst, nur der Port ist fest. Auch
// "/help" ist kein Sonderfall hier im Frontend — es läuft wie jeder andere
// Prompt über /api/ask, der Kern erkennt es (siehe api.rs::help_command).

// Der Port muss mit PORT in src-tauri/src/api.rs übereinstimmen. Der Host
// ergibt sich normalerweise aus der Seite selbst (Browser/PWA/Desktop-Fenster
// laufen ja vom Host des Kerns). Eine native Android-App lädt die Oberfläche
// aber aus dem App-Bundle, ohne dass die URL etwas über den entfernten
// Iris-Rechner verrät — dafür merkt sich Iris einmalig eine manuell
// eingetragene Adresse (siehe requestServerAddress unten), statt dafür ein
// Einstellungsmenü zu brauchen: dasselbe eine Textfeld wie für jeden Prompt.
import { qrcodegen } from "./qrcodegen";

const SERVER_ADDRESS_KEY = "iris-server-address";

function computeDefaultApiBase(): string {
  return `${location.protocol}//${location.hostname}:47615`;
}

function getStoredApiBase(): string | null {
  try {
    return localStorage.getItem(SERVER_ADDRESS_KEY);
  } catch {
    return null;
  }
}

function storeApiBase(base: string) {
  try {
    localStorage.setItem(SERVER_ADDRESS_KEY, base);
  } catch {
    // localStorage kann in seltenen Kontexten fehlen — dann fragt Iris bei
    // jedem Start erneut, statt abzustürzen.
  }
}

function normalizeServerAddress(input: string): string {
  let value = input.trim();
  if (!/^https?:\/\//.test(value)) {
    value = `http://${value}`;
  }
  value = value.replace(/\/+$/, "");
  if (!/:\d+$/.test(value)) {
    value = `${value}:47615`;
  }
  return value;
}

let API_BASE = getStoredApiBase() ?? computeDefaultApiBase();

// Macht dieselbe Oberfläche auf Android/iOS/Desktop installierbar
// (docs/PLAN.md, Abschnitt 4). Nur die App-Shell wird zwischengespeichert,
// nie /api/* (siehe public/sw.js).
if ("serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js").catch(() => {
      // Kein Absturz, falls z. B. über http:// ohne Secure-Context geladen —
      // die App funktioniert auch ganz ohne Service Worker.
    });
  });
}

interface StatusResponse {
  ram_gb: number;
  tier: string;
  recommended_model: string;
  ollama_available: boolean;
  installed_models: string[];
  llm_backend: "ollama" | "cloud";
  cloud_provider: string | null;
}

interface PermissionRequest {
  id: string;
  anzeigetext: string;
  scope: string | null;
}

interface AskResponse {
  response: string;
  command_handled: boolean;
  permission_request: PermissionRequest | null;
}

interface GrantResponse {
  id: string;
  scope: string | null;
  granted_at_unix: number;
  tools: string[] | null;
  connect_error: string | null;
  llm_note: string | null;
}

function el<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Element #${id} fehlt im DOM`);
  return found as T;
}

let statusEl: HTMLElement;
let outputTextEl: HTMLElement;
let outputActionsEl: HTMLElement;
let modelSelect: HTMLSelectElement;
let promptInput: HTMLTextAreaElement;
let sendButton: HTMLButtonElement;

function setOutput(text: string) {
  outputTextEl.textContent = text;
  outputActionsEl.innerHTML = "";
}

// Lässt das Eingabefeld mit dem Text mitwachsen (bis zum in styles.css
// gesetzten max-height, danach übernimmt dessen eigenes Scrollen) - der
// "Gemini-Stil" braucht das, sonst bliebe die Pille bei jeder Zeilenzahl
// gleich hoch mit abgeschnittenem Text.
function autosizeTextarea() {
  promptInput.style.height = "auto";
  promptInput.style.height = `${promptInput.scrollHeight}px`;
}

function setStatus(text: string) {
  statusEl.textContent = text;
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const resp = await fetch(`${API_BASE}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}

// Der Button ist keine Bestätigung für eine einzelne Handlung, sondern der
// Moment, in dem eine Berechtigung dauerhaft erteilt wird (docs/PLAN.md,
// Abschnitt 2). Anzeigetext und ID kommen unverändert aus dem Kern.
function showPermissionButton(request: PermissionRequest) {
  outputActionsEl.innerHTML = "";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Erlauben";
  button.addEventListener("click", async () => {
    button.disabled = true;
    try {
      const granted = await postJson<GrantResponse>("/api/permissions/grant", {
        id: request.id,
        scope: request.scope,
      });
      if (granted.tools) {
        setOutput(
          `✓ Erteilt: ${request.anzeigetext}. Verbunden. Werkzeuge: ${
            granted.tools.length > 0 ? granted.tools.join(", ") : "keine"
          }.`,
        );
      } else if (granted.connect_error) {
        setOutput(`✓ Erteilt: ${request.anzeigetext}. Verbindung fehlgeschlagen: ${granted.connect_error}`);
      } else if (granted.llm_note) {
        setOutput(granted.llm_note);
      } else {
        setOutput(`✓ Erteilt: ${request.anzeigetext}.`);
      }
    } catch (err) {
      setOutput(`Fehler beim Erteilen: ${err}`);
    }
  });
  outputActionsEl.appendChild(button);
}

interface PairingResponse {
  available: boolean;
  address: string | null;
}

// Zeichnet den QR-Code als SVG-Pfad (ein "M...h1v1h-1z" pro dunklem Modul,
// zusammengefasst in einem einzigen <path>). Kein Canvas nötig, skaliert
// verlustfrei per CSS.
function qrToSvg(qr: qrcodegen.QrCode, border: number): string {
  const parts: string[] = [];
  for (let y = 0; y < qr.size; y++) {
    for (let x = 0; x < qr.size; x++) {
      if (qr.getModule(x, y)) parts.push(`M${x + border},${y + border}h1v1h-1z`);
    }
  }
  const dim = qr.size + border * 2;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${dim} ${dim}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="#ffffff"/><path d="${parts.join(" ")}" fill="#000000"/></svg>`;
}

// Zeigt einen QR-Code mit der eigenen Tailscale-Adresse, damit sich ein
// Handy verbinden kann, ohne die Adresse von Hand abzutippen (siehe
// api.rs::detect_tailscale_address). Rein informativ, kein Einstellungsmenü
// — blendet sich einfach aus, wenn tailscale nicht installiert/angemeldet ist.
async function showPairingIfAvailable() {
  try {
    const resp = await fetch(`${API_BASE}/api/pairing`);
    if (!resp.ok) return;
    const data: PairingResponse = await resp.json();
    const panel = el<HTMLElement>("pairing-panel");
    if (!data.available || !data.address) {
      panel.hidden = true;
      return;
    }
    const qr = qrcodegen.QrCode.encodeText(data.address, qrcodegen.QrCode.Ecc.MEDIUM);
    el<HTMLElement>("pairing-qr").innerHTML = qrToSvg(qr, 2);
    el<HTMLElement>("pairing-address").textContent = data.address;
    panel.hidden = false;
  } catch {
    // Kein Absturz, falls /api/pairing (noch) nicht erreichbar ist.
  }
}

function populateModelSelect(installed: string[], recommended: string) {
  modelSelect.disabled = false;
  modelSelect.innerHTML = "";

  const options = installed.length > 0 ? installed : [recommended];
  for (const name of options) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent =
      name === recommended && installed.includes(name)
        ? `${name} (empfohlen)`
        : name;
    modelSelect.appendChild(option);
  }
  if (options.includes(recommended)) {
    modelSelect.value = recommended;
  }
}

// Im Cloud-Backend gibt es keine lokale Modellwahl (das Modell ist pro
// Anbieter fest, siehe llm.rs::Provider::default_model) — die Chip zeigt
// stattdessen nur, welcher Anbieter gerade antwortet.
function showCloudProviderChip(provider: string) {
  modelSelect.innerHTML = "";
  const option = document.createElement("option");
  option.value = provider;
  option.textContent = provider;
  modelSelect.appendChild(option);
  modelSelect.disabled = true;
}

// Der Kern startet als eigener HTTP-Dienst nebenläufig zum Fenster; ein
// kurzer Retry-Loop überbrückt die seltene Sekunde, bevor er antwortet.
async function fetchWithRetry(path: string, attempts = 10): Promise<Response> {
  for (let i = 0; i < attempts; i++) {
    try {
      return await fetch(`${API_BASE}${path}`);
    } catch (err) {
      if (i === attempts - 1) throw err;
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
  }
  throw new Error("unreachable");
}

// "ask" ist der Normalfall (Textfeld = Prompt an Iris). "configure-address"
// übernimmt kurzzeitig dasselbe Textfeld für die einmalige Server-Adresse,
// wenn der Kern unter der automatisch ermittelten Adresse nicht erreichbar
// ist (typischerweise: native Android-Thin-Client-App, siehe oben).
type Mode = "ask" | "configure-address";
let mode: Mode = "ask";

function requestServerAddress() {
  mode = "configure-address";
  setStatus(
    "Kein Iris-Kern unter dieser Adresse gefunden. Adresse des Rechners eintragen, auf dem Iris läuft (z. B. eine Tailscale-Adresse) — einmalig, wird gemerkt.",
  );
  promptInput.value = "";
  promptInput.placeholder = "z. B. mein-rechner.tailxxxx.ts.net";
  // Icon bleibt (Pfeil passt für "Senden" wie "Verbinden") - nur der
  // barrierefreie Name ändert sich, damit der Button nicht seine Form
  // verliert (er ist jetzt ein reiner Icon-Button, kein Text mehr).
  sendButton.setAttribute("aria-label", "Verbinden");
  autosizeTextarea();
}

async function configureAddress() {
  const address = promptInput.value.trim();
  if (!address) return;
  API_BASE = normalizeServerAddress(address);
  storeApiBase(API_BASE);
  promptInput.value = "";
  promptInput.placeholder = "Frag Iris etwas …";
  sendButton.setAttribute("aria-label", "Senden");
  mode = "ask";
  autosizeTextarea();
  await trySetup();
}

async function trySetup() {
  try {
    await init();
  } catch {
    requestServerAddress();
  }
}

async function init() {
  setStatus("Prüfe Iris-Kern …");
  const statusResp = await fetchWithRetry("/api/status");
  const status: StatusResponse = await statusResp.json();

  showPairingIfAvailable();

  // Läuft gerade über einen Online-Anbieter (API-Key hinterlegt): der
  // Ollama-Status ist dann kein Warnsignal mehr, sondern erwartet — die
  // Statuszeile soll das nicht wie einen Fehler behandeln.
  if (status.llm_backend === "cloud" && status.cloud_provider) {
    showCloudProviderChip(status.cloud_provider);
    setStatus(`Bereit. Antworten laufen über ${status.cloud_provider}.`);
    return;
  }

  // Der Button bleibt in jedem Fall bedienbar: Registry-Aktionen,
  // Berechtigungen, Postfach und /help brauchen kein Ollama (siehe
  // api.rs::ask_handler — die werden vor dem Ollama-Fallback erkannt).
  // Nur eine echte Modellanfrage ohne laufendes Ollama scheitert dann mit
  // einer klaren Fehlermeldung.
  if (!status.ollama_available) {
    setStatus(
      `Ollama läuft nicht auf diesem Gerät. Empfohlenes Modell für ${status.ram_gb.toFixed(
        1,
      )} GB RAM: ${status.recommended_model}. Bitte Ollama starten — Befehle wie /help funktionieren trotzdem.`,
    );
    populateModelSelect([], status.recommended_model);
    return;
  }

  populateModelSelect(status.installed_models, status.recommended_model);

  if (status.installed_models.length === 0) {
    setStatus(
      `Ollama läuft, aber kein Modell installiert. Empfehlung für ${status.ram_gb.toFixed(
        1,
      )} GB RAM: ollama pull ${status.recommended_model}`,
    );
    return;
  }

  setStatus(
    `Bereit. ${status.ram_gb.toFixed(1)} GB RAM erkannt, empfohlenes Modell: ${
      status.recommended_model
    }.`,
  );
}

async function ask() {
  const prompt = promptInput.value.trim();
  if (!prompt) return;

  sendButton.disabled = true;
  setOutput("Iris denkt nach …");

  try {
    const { response, command_handled, permission_request } = await postJson<AskResponse>(
      "/api/ask",
      { model: modelSelect.value, prompt },
    );
    setOutput(command_handled ? `✓ ${response}` : response);
    if (permission_request) {
      showPermissionButton(permission_request);
    }
  } catch (err) {
    setOutput(`Fehler: ${err}`);
  } finally {
    sendButton.disabled = false;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  statusEl = el("status-line");
  outputTextEl = el("output-text");
  outputActionsEl = el("output-actions");
  modelSelect = el("model-select");
  promptInput = el("prompt-input");
  sendButton = el("send-button");

  el("prompt-form").addEventListener("submit", (e) => {
    e.preventDefault();
    if (mode === "configure-address") {
      configureAddress();
    } else {
      ask();
    }
  });

  promptInput.addEventListener("input", autosizeTextarea);

  // Enter sendet (wie bei Gemini/ChatGPT), Umschalt+Enter fügt einen
  // Zeilenumbruch ein - sonst bräuchte man für mehrzeilige Prompts die
  // Maus, nur weil das Feld jetzt einzeilig startet statt mit rows="4".
  promptInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      el<HTMLFormElement>("prompt-form").requestSubmit();
    }
  });

  el<HTMLButtonElement>("pairing-copy").addEventListener("click", async () => {
    const address = el<HTMLElement>("pairing-address").textContent ?? "";
    try {
      await navigator.clipboard.writeText(address);
    } catch {
      // Adresse steht ohnehin sichtbar da - manuell markieren geht immer.
    }
  });

  trySetup();
});
