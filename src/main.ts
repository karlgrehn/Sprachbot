import { invoke } from "@tauri-apps/api/core";

interface ModelRecommendation {
  ram_gb: number;
  tier: string;
  model: string;
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

async function init() {
  const recommendation = await invoke<ModelRecommendation>("recommend_model");
  const available = await invoke<boolean>("ollama_status");

  if (!available) {
    setStatus(
      `Ollama läuft nicht auf diesem Gerät. Empfohlenes Modell für ${recommendation.ram_gb.toFixed(
        1,
      )} GB RAM: ${recommendation.model}. Bitte Ollama starten.`,
    );
    if (sendButton) sendButton.disabled = true;
    populateModelSelect([], recommendation.model);
    return;
  }

  const installed = await invoke<string[]>("list_models");
  populateModelSelect(installed, recommendation.model);

  if (installed.length === 0) {
    setStatus(
      `Ollama läuft, aber kein Modell installiert. Empfehlung für ${recommendation.ram_gb.toFixed(
        1,
      )} GB RAM: ollama pull ${recommendation.model}`,
    );
    if (sendButton) sendButton.disabled = true;
    return;
  }

  setStatus(
    `Bereit. ${recommendation.ram_gb.toFixed(1)} GB RAM erkannt, empfohlenes Modell: ${
      recommendation.model
    }.`,
  );
}

async function ask() {
  if (!outputEl || !promptInput || !modelSelect || !sendButton) return;
  const prompt = promptInput.value.trim();
  if (!prompt) return;

  sendButton.disabled = true;
  outputEl.textContent = "Iris denkt nach …";

  try {
    const answer = await invoke<string>("ask", {
      model: modelSelect.value,
      prompt,
    });
    outputEl.textContent = answer;
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
