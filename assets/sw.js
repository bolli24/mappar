const cacheName = 'mappar-pwa';
const filesToCache = [
  './',
  './index.html',
    "./dummy_worker.js",
    "./dummy_worker_bg.wasm"
];

self.addEventListener("message", e => {
  caches.open(cacheName).then(function (cache) {
    return cache.addAll(e.data + filesToCache);
  })
})

/* Serve cached content when offline */
self.addEventListener('fetch', function (e) {
  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    })
  );
});
