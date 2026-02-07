/// <reference no-default-lib="true"/>
/// <reference lib="webworker" />
/// <reference no-default-lib="true"/>

declare const self: ServiceWorkerGlobalScope;

const CACHE_VALID: Array<string> = [];

const makeError = (error: string, status = 400) => {
	return new Response(JSON.stringify({ error }), {
		status,
		headers: { "content-type": "application/json" },
	});
};

const deleteOldCaches = async () => {
	const c = await caches.keys();

	console.log("[sw] prune caches", {
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
	console.log("[sw] serviceworker installed");
	self.skipWaiting();
});

self.addEventListener("activate", (e) => {
	console.log("[sw] activated");
	e.waitUntil(Promise.all([
		deleteOldCaches(),
		self.registration.navigationPreload.enable(),
		self.clients.claim(),
	]));
});

self.addEventListener("push", e => {
	console.log("[sw] pushed", e.data);
	// TODO: fetch full message, display notif similarly to api.tsx
});
