// Vercel-Serverless-Funktion: /api/downloads-info
//
// Liefert nur Metadaten (Dateigröße je Plattform) für die Anzeige auf der
// Landingpage — keine URLs, die zeigt der Browser nie direkt. Der eigentliche
// Download läuft über /api/download/<platform> (siehe dort).
const { ASSET_PATTERNS, fetchLatestRelease, findAsset } = require("./_github");

module.exports = async function handler(req, res) {
  const token = process.env.GITHUB_DOWNLOAD_TOKEN;
  if (!token) {
    res.setHeader("Cache-Control", "no-store");
    res.status(200).json({});
    return;
  }

  const release = await fetchLatestRelease(token);
  const result = {};
  if (release) {
    for (const platform of Object.keys(ASSET_PATTERNS)) {
      const asset = findAsset(release, platform);
      if (asset) result[platform] = { bytes: asset.size };
    }
  }

  res.setHeader("Cache-Control", "public, max-age=60");
  res.status(200).json(result);
};
