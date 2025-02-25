/// <reference no-default-lib="true"/>
/// <reference lib="webworker" />

declare const self: ServiceWorkerGlobalScope;

const CACHE_VALID = ["v2.media", "v1.assets"];

const makeError = (error: string, status = 400) => {
	return new Response(JSON.stringify({ error }), {
		status,
		headers: { "content-type": "application/json" },
	});
};

const deleteOldCaches = async () => {
	const c = await caches.keys();

	console.log("prune caches", {
		existing: c,
		current: CACHE_VALID,
	});

	return Promise.all(
		c.filter((i) => !CACHE_VALID.includes(i)).map((i) => caches.delete(i)),
	);
};

const shouldCache = (req: Request) => {
	if (req.method !== "GET" && req.method !== "HEAD") return false;
	// const url = new URL(req.url, self.location.href);
	// console.log("should cache?", url.href);
	return false;
};

self.addEventListener("install", () => {
	console.log("serviceworker installed");
	self.skipWaiting();
});

self.addEventListener("activate", (e) => {
	console.log("serviceworker activated");
	e.waitUntil(Promise.all([
		deleteOldCaches(),
		self.registration.navigationPreload.enable(),
		self.clients.claim(),
	]));
});

self.addEventListener("fetch", (e) => {
	const req = e.request;
	const url = new URL(req.url);
	if (url.pathname.startsWith("/api")) return;

	e.respondWith(
		(async () => {
			// const client = await self.clients.get(e.clientId);
			// client?.postMessage("hi!! helloo!!!!");

			const cached = await caches.match(req);
			if (cached) return cached;

			if (req.method === "GET" && url.pathname === "/_media") {
				const target = url.searchParams.get("url");
				if (!target) return makeError("missing url");

				const cached = await caches.match(target, { ignoreSearch: true });
				if (cached) return cached;

				const res = await fetch(target, { mode: "cors" });
				if (res.status === 206) return res; // range requests are a bit h right now
				if (res.ok) {
					const res2 = res.clone();
					e.waitUntil((async () => {
						const cache = await caches.open("v2.media");
						cache.put(target, res2);
					})());
				} else {
					console.error(res);
				}

				return res;
			}

			const preload = await e.preloadResponse;
			if (preload) return preload;

			const res = await fetch(req);

			if (res.ok && shouldCache(req)) {
				const res2 = res.clone();
				e.waitUntil((async () => {
					const cache = await caches.open("v1.assets");
					await cache.put(req, res2);
				})());
			}

			return res;
		})().catch((err) => {
			console.error(err);
			return makeError("network error", 408);
		}),
	);
});
