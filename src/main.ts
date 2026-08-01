// Iris-Oberfläche: spricht ausschließlich über HTTP mit dem lokalen Kern
// (src-tauri/src/api.rs), nie über Tauri-`invoke`. Damit ist dieselbe
// Oberfläche später ohne Änderung als Thin Client über Tailscale nutzbar
// (siehe docs/PLAN.md, Abschnitt 3).

// Muss mit PORT in src-tauri/src/api.rs übereinstimmen.
const API_BASE = "http://127.0.0.1:47615";

interface StatusResponse {
  ram_gb: number;
  tier: string;
  recommended_model: string;
  ollama_available: boolean;
}

interface RegistryAction {
  id: string;
  anzeigetext: string;
  reversible: boolean;
}

interface AskResponse {
  response: string;
  command_handled: boolean;
}

let statusEl: HTMLElement | null;
let outputEl: HTMLElement | null;
let modelSelect: HTMLSelectElement | null;
let promptInput: HTMLTextAreaElement | null;
let sendButton: HTMLButtonElement | null;

function setStatus(text: string) {
  if (statusEl) statusEl.textContent = text;
}

function populateModelSelect(installed: string[], recommended: string) {
  if (!modelSelect) return;
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

async function init() {
  setStatus("Prüfe Iris-Kern …");
  const statusResp = await fetchWithRetry("/api/status");
  const status: StatusResponse = await statusResp.json();

  if (!status.ollama_available) {
    setStatus(
      `Ollama läuft nicht auf diesem Gerät. Empfohlenes Modell für ${status.ram_gb.toFixed(
        1,
      )} GB RAM: ${status.recommended_model}. Bitte Ollama starten.`,
    );
    if (sendButton) sendButton.disabled = true;
    populateModelSelect([], status.recommended_model);
    return;
  }

  const modelsResp = await fetch(`${API_BASE}/api/models`);
  const installed: string[] = await modelsResp.json();
  populateModelSelect(installed, status.recommended_model);

  if (installed.length === 0) {
    setStatus(
      `Ollama läuft, aber kein Modell installiert. Empfehlung für ${status.ram_gb.toFixed(
        1,
      )} GB RAM: ollama pull ${status.recommended_model}`,
    );
    if (sendButton) sendButton.disabled = true;
    return;
  }

  setStatus(
    `Bereit. ${status.ram_gb.toFixed(1)} GB RAM erkannt, empfohlenes Modell: ${
      status.recommended_model
    }.`,
  );
}

// Erzeugt aus der Registry, nicht aus dem Modellgedächtnis — sonst
// erfindet Iris Funktionen, die es nicht gibt (docs/PLAN.md, Abschnitt 2).
async function showHelp() {
  if (!outputEl) return;
  const resp = await fetch(`${API_BASE}/api/registry`);
  const { actions }: { actions: RegistryAction[] } = await resp.json();

  const lines = [
    "Was Iris gerade kann (aus der Registry, nicht vom Modell erfunden):",
    "",
    ...actions.map(
      (a) => `- ${a.anzeigetext}${a.reversible ? " (rückgängig machbar)" : ""}`,
    ),
    "",
    "Sag Iris einfach, was du willst — z. B. \"antworte ab jetzt kurz\" oder \"mach das rückgängig\".",
  ];
  outputEl.textContent = lines.join("\n");
}

async function ask() {
  if (!outputEl || !promptInput || !modelSelect || !sendButton) return;
  const prompt = promptInput.value.trim();
  if (!prompt) return;

  if (prompt.toLowerCase() === "/help") {
    promptInput.value = "";
    return showHelp();
  }

  sendButton.disabled = true;
  outputEl.textContent = "Iris denkt nach …";

  try {
    const resp = await fetch(`${API_BASE}/api/ask`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model: modelSelect.value, prompt }),
    });
    if (!resp.ok) {
      throw new Error(await resp.text());
    }
    const { response, command_handled }: AskResponse = await resp.json();
    outputEl.textContent = command_handled ? `✓ ${response}` : response;
  } catch (err) {
    outputEl.textContent = `Fehler: ${err}`;
  } finally {
    sendButton.disabled = false;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  statusEl = document.querySelector("#status-line");
  outputEl = document.querySelector("#output");
  modelSelect = document.querySelector("#model-select");
  promptInput = document.querySelector("#prompt-input");
  sendButton = document.querySelector("#send-button");

  document.querySelector("#prompt-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    ask();
  });

  init().catch((err) => setStatus(`Fehler beim Start: ${err}`));
});
