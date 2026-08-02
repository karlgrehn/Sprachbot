// Vercel-Serverless-Funktion: /api/download/windows|linux|android
//
// Läuft server-seitig, nie im Browser sichtbar. Holt das passende Release-
// Asset per authentifiziertem GitHub-API-Aufruf (funktioniert dadurch auch
// bei privatem Repo — siehe README "Downloads ohne öffentliches Repo") und
// leitet auf die von GitHub signierte, zeitlich begrenzte CDN-URL weiter.
// Kein Byte läuft durch diese Funktion selbst — bei Dateien von mehreren
// hundert MB (die Offline-Installer) würde ein direktes Durchschleifen an
// Vercels Ausführungszeit-/Antwortgrößen-Limits scheitern; ein Redirect auf
// die signierte URL hat dieses Problem nicht.
const { githubHeaders, fetchLatestRelease, findAsset } = require("../_github");

module.exports = async function handler(req, res) {
  const platform = String(req.query.platform || "");

  // GITHUB_DOWNLOAD_TOKEN ist nur nötig, sobald das Repo privat ist - so
  // lange es öffentlich ist (aktueller Stand), funktioniert die GitHub-API
  // auch unauthentifiziert. Kein hartes Erfordernis mehr, damit Downloads
  // schon heute funktionieren, ohne dass ein Token angelegt sein muss.
  const token = process.env.GITHUB_DOWNLOAD_TOKEN;

  const release = await fetchLatestRelease(token);
  if (!release) {
    res.status(404).send("Keine veröffentlichte Version gefunden.");
    return;
  }

  const asset = findAsset(release, platform);
  if (!asset) {
    res.status(404).send("Keine Datei für diese Plattform gefunden.");
    return;
  }

  res.setHeader("Cache-Control", "no-store");

  if (!token) {
    // Öffentliches Repo: die normale Download-URL ist bereits öffentlich
    // erreichbar, kein Umweg über die authentifizierte Asset-API nötig.
    res.redirect(302, asset.browser_download_url);
    return;
  }

  // Privates Repo: Accept: application/octet-stream auf den Asset-API-
  // Endpunkt liefert einen 302 auf eine signierte, unauthentifizierte
  // CDN-URL — auch für private Repos, sobald der Redirect einmal mit
  // gültigem Token geholt wurde. redirect: "manual", um diesen Redirect
  // selbst zu bekommen statt ihm zu folgen.
  const assetResp = await fetch(asset.url, {
    headers: githubHeaders(token, "application/octet-stream"),
    redirect: "manual",
  });

  const location = assetResp.headers.get("location");
  if (assetResp.status >= 300 && assetResp.status < 400 && location) {
    res.redirect(302, location);
    return;
  }

  res.status(502).send("Download derzeit nicht verfügbar.");
};
