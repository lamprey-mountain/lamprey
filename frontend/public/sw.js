console.log("hello from service worker");

const shouldCache = (req) => {
	if (req.method !== "GET" && req.method !== "HEAD") return false;
	const url = new URL(req.url, self.location.href);
	console.log(url);
	if (url.hostname === "chat-files.celery.eu.org") return true;
	return false;
};

self.addEventListener("fetch", (e) => {
	e.respondWith((async () => {
		const req = e.request;
		const cached = await caches.match(req);
		if (cached) return cached;

		const preload = await e.preloadResponse;
		if (preload) return preload;

		try {
			const res = await fetch(req);
			if (res.ok && shouldCache(req)) {
				console.log("cache", req.url);
				const cache = await caches.open("testing");
				await cache.put(req, res.clone());
			}
			return res;
		} catch (err) {
			console.error(err);
			return new Response(JSON.stringify({ error: "network error" }), {
				status: 408,
				headers: { "content-type": "application/json" },
			});
		}
	})());
});
