// Gemeinsame Hilfsfunktionen für die Download-Proxy-Endpunkte
// (download/[platform].js, downloads-info.js). Läuft serverseitig auf
// Vercel — der Token bleibt hier, nie im Browser. Das erlaubt, das
// GitHub-Repository privat zu stellen, ohne die Downloads zu verlieren:
// die Website selbst zeigt oder verlinkt GitHub nirgends, nur dieser
// Server-Code spricht (authentifiziert) mit der GitHub-API.
const REPO = "karlgrehn/Sprachbot";

// "-offline" bündelt Ollama + Modell (kein weiterer Install nötig, aber
// deutlich größer - siehe web/index.html). "-online" ist nur die App
// selbst, erwartet eine separat installierte Ollama-Instanz (viel kleiner).
// Android hat nur einen Weg (reiner Thin Client, kein Ollama auf dem Handy).
const ASSET_PATTERNS = {
  "windows-offline": /^Iris-Offline_.*_x64-setup\.exe$/i,
  "windows-online": /^Iris_.*_x64-setup\.exe$/i,
  "linux-offline": /^Iris-Offline_.*\.AppImage$/i,
  "linux-online": /^Iris_.*\.AppImage$/i,
  android: /^Iris\.apk$/i,
};

function githubHeaders(token, accept) {
  const headers = {
    Accept: accept || "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "iris-download-proxy",
  };
  // Ohne Token (aktuell: Repo ist noch öffentlich) funktioniert die
  // GitHub-API für Releases/Assets auch unauthentifiziert - Bearer
  // undefined mitzuschicken würde die Anfrage nur unnötig ablehnen lassen.
  if (token) headers.Authorization = `Bearer ${token}`;
  return headers;
}

async function fetchLatestRelease(token) {
  const resp = await fetch(`https://api.github.com/repos/${REPO}/releases`, {
    headers: githubHeaders(token),
  });
  if (!resp.ok) return null;
  const releases = await resp.json();
  return releases.find((r) => !r.draft) || null;
}

function findAsset(release, platform) {
  const pattern = ASSET_PATTERNS[platform];
  if (!pattern || !release) return null;
  return (release.assets || []).find((a) => pattern.test(a.name)) || null;
}

module.exports = { REPO, ASSET_PATTERNS, githubHeaders, fetchLatestRelease, findAsset };
