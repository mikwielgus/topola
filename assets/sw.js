var cacheName = 'topola-egui';
var filesToCache = [
  './',
  './index.html',
  // I'm not sure if these two work, as teir filenames in dist/ have an alphanumeric string appended.
  './topola-egui.js',
  './topola-egui_bg.wasm',
];

// Start the service worker and cache all of the app's content.
self.addEventListener('install', function (e) {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    })
  );
});

// Serve cached content when offline.
self.addEventListener('fetch', function (e) {
  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    })
  );
});
