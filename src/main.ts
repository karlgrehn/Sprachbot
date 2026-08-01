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
      } else {
        setOutput(`✓ Erteilt: ${request.anzeigetext}.`);
      }
    } catch (err) {
      setOutput(`Fehler beim Erteilen: ${err}`);
    }
  });
  outputActionsEl.appendChild(button);
}

function populateModelSelect(installed: string[], recommended: string) {
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
  sendButton.textContent = "Verbinden";
}

async function configureAddress() {
  const address = promptInput.value.trim();
  if (!address) return;
  API_BASE = normalizeServerAddress(address);
  storeApiBase(API_BASE);
  promptInput.value = "";
  promptInput.placeholder = "Frag Iris etwas …";
  sendButton.textContent = "Senden";
  mode = "ask";
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

  trySetup();
});
