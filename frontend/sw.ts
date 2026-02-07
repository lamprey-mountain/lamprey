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

async function getState(): Promise<
	{ api_url: string | null; token: string | null }
> {
	return new Promise((resolve) => {
		const request = indexedDB.open("sw-state", 1);
		request.onsuccess = () => {
			const db = request.result;
			const tx = db.transaction("state", "readonly");
			const store = tx.objectStore("state");
			const apiUrlReq = store.get("api_url");
			const tokenReq = store.get("token");
			tx.oncomplete = () => {
				resolve({
					api_url: apiUrlReq.result,
					token: tokenReq.result,
				});
			};
		};
		request.onerror = () => resolve({ api_url: null, token: null });
	});
}

self.addEventListener("push", (e) => {
	console.log("[sw] pushed", e.data?.json());
	const data = e.data?.json();
	if (!data) return;

	e.waitUntil((async () => {
		const { api_url, token } = await getState();
		if (!api_url || !token) return;

		const headers = {
			"Authorization": `Bearer ${token}`,
		};

		const [notif, channel] = await Promise.all([
			fetch(
				`${api_url}/api/v1/channel/${data.channel_id}/message/${data.message_id}`,
				{ headers },
			).then((res) => res.json()),
			fetch(`${api_url}/api/v1/channel/${data.channel_id}`, { headers }).then(
				(res) => res.json(),
			),
		]);

		const message = "latest_version" in notif ? notif.latest_version : notif;
		const author = await fetch(`${api_url}/api/v1/user/${message.author_id}`, {
			headers,
		}).then((res) => res.json());

		const title = `${author.name} in #${channel.name}`;
		const body = message.content?.substring(0, 200) || "";

		let icon: string | undefined;
		if (author.avatar) {
			icon = `${api_url}/api/v1/media/${author.avatar}/blob`;
		}

		await self.registration.showNotification(title, {
			body,
			icon,
			data: {
				channel_id: data.channel_id,
				message_id: data.message_id,
			},
		});
	})());
});

self.addEventListener("notificationclick", (event) => {
	event.notification.close();
	const { channel_id, message_id } = event.notification.data;
	const url = `/channel/${channel_id}/message/${message_id}`;

	event.waitUntil(
		self.clients.matchAll({ type: "window", includeUncontrolled: true }).then(
			(clientList) => {
				for (const client of clientList) {
					if (client.url.includes(self.location.origin) && "focus" in client) {
						return (client as WindowClient).navigate(url).then((c) =>
							c.focus()
						);
					}
				}
				if (self.clients.openWindow) {
					return self.clients.openWindow(url);
				}
			},
		),
	);
});
