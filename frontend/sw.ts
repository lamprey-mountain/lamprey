/// <reference no-default-lib="true"/>
/// <reference lib="webworker" />
/// <reference no-default-lib="true"/>

// FIXME: firefox doesnt like it when i import?
// import { stripMarkdownAndResolveMentions } from "./src/notification-util.ts";

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

		const mockApi = {
			users: { cache: new Map() },
			channels: { cache: new Map([[data.channel_id, channel]]) },
			roles: { cache: new Map() },
			client: {
				http: {
					GET: async (url: string, options: any) => {
						let finalUrl = url;
						if (options.params?.path) {
							for (const [key, value] of Object.entries(options.params.path)) {
								finalUrl = finalUrl.replace(`{${key}}`, value as string);
							}
						}
						const res = await fetch(`${api_url}${finalUrl}`, { headers });
						if (res.ok) {
							return { data: await res.json() };
						}
						return { data: null };
					},
				},
			},
		};

		const processedContent = await stripMarkdownAndResolveMentions(
			message.content || "",
			data.channel_id,
			mockApi,
			message.mentions,
		);
		const body = processedContent.substring(0, 200);

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

async function stripMarkdownAndResolveMentions(
	content: string,
	thread_id: string,
	api: any,
	mentions?: Message["mentions"],
) {
	const { users, channels, roles, client } = api;
	let processedContent = content;

	// Replace user mentions <@user-id> with user names
	const userMentionRegex =
		/<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	processedContent = processedContent.replace(
		userMentionRegex,
		(match, userId) => {
			const mentioned = (mentions?.users as any[])?.find((u) =>
				u.id === userId
			);
			if (mentioned) return `@${mentioned.resolved_name}`;
			const user = users.cache.get(userId);
			return user ? `@${user.name}` : match; // Keep original if user not found
		},
	);

	// Replace channel mentions <#channel-id> with channel names
	const channelMentionRegex =
		/<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	processedContent = processedContent.replace(
		channelMentionRegex,
		(match, channelId) => {
			const mentioned = (mentions?.channels as any[])?.find((c) =>
				c.id === channelId
			);
			if (mentioned) return `#${mentioned.name}`;
			const channel = channels.cache.get(channelId);
			return channel ? `#${channel.name}` : match; // Keep original if channel not found
		},
	);

	// Replace role mentions <@&role-id> with role names
	const roleMentionRegex =
		/<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	const roleMatches = Array.from(processedContent.matchAll(roleMentionRegex));
	for (const match of roleMatches) {
		const roleId = match[1];
		const thread = channels.cache.get(thread_id);
		if (!thread?.room_id) continue;

		let roleName: string | undefined;
		const cached = roles.cache.get(roleId);
		if (cached) {
			roleName = cached.name;
		} else {
			const { data } = await client.http.GET(
				"/api/v1/room/{room_id}/role/{role_id}",
				{
					params: { path: { room_id: thread.room_id, role_id: roleId } },
				},
			);
			if (data) {
				roles.cache.set(roleId, data);
				roleName = data.name;
			}
		}

		if (roleName) {
			processedContent = processedContent.replace(match[0], `@${roleName}`);
		}
	}

	// Replace emoji mentions <:name:id> with emoji name
	const emojiMentionRegex =
		/<:(\w+):[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}>/g;
	processedContent = processedContent.replace(
		emojiMentionRegex,
		(match, emojiName) => {
			return `:${emojiName}:`;
		},
	);

	// Remove basic markdown formatting
	// Bold: **text** -> text
	processedContent = processedContent.replace(/\*\*(.*?)\*\*/g, "$1");
	// Italic: *text* or _text_ -> text
	processedContent = processedContent.replace(/([*_])(.*?)\1/g, "$2");
	// Strikethrough: ~~text~~ -> text
	processedContent = processedContent.replace(/~~(.*?)~~/g, "$1");
	// Code: `text` -> text
	processedContent = processedContent.replace(/`(.*?)`/g, "$1");
	// Code blocks: ```language\ntext\n``` -> text
	processedContent = processedContent.replace(
		/```(?:\w+\n)?\n?([\s\S]*?)\n?```/g,
		"$1",
	);
	// Blockquotes: > text on new lines -> text
	processedContent = processedContent.replace(/^ *>(.*)$/gm, "$1");
	// Links: [text](url) -> text
	processedContent = processedContent.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");

	// Clean up extra whitespace
	processedContent = processedContent.replace(/\s+/g, " ").trim();

	return processedContent;
}
