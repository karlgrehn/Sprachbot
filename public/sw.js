// Service Worker für die Installierbarkeit als PWA (Android/iOS/Desktop,
// docs/PLAN.md Abschnitt 4: "iOS bekommt keine native App... PWA ersetzt
// vier Plattform-Portierungen"). Cacht ausschließlich die statische
// Oberfläche (App-Shell) für Offline-Start — niemals /api/*, das muss
// immer live vom Kern kommen, sonst zeigt der Thin Client veraltete
// Berechtigungen/Postfach-Stände an.

const CACHE_NAME = "iris-shell-v1";

self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== "GET" || url.pathname.startsWith("/api/")) {
    return;
  }

  event.respondWith(
    fetch(event.request)
      .then((response) => {
        const copy = response.clone();
        caches.open(CACHE_NAME).then((cache) => cache.put(event.request, copy));
        return response;
      })
      .catch(() => caches.match(event.request)),
  );
});
