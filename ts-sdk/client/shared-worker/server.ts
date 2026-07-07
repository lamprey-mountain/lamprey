/// <reference no-default-lib="true"/>
/// <reference lib="es2024" />
/// <reference lib="webworker" />

import { Syncer } from "../syncer";
import {
	HEATBEAT_INTERVAL_REQUIRED,
	type WorkerCommand,
	type WorkerEvent,
} from "./shared";

declare const self: SharedWorkerGlobalScope;

// console.log(self.name); // maybe don't parse session id from this?

// const clients = new Map();
//
// clients.set("123", {
// 	ports: new Set(),
//  syncer: Syncer,
// })

// maybe combine worker state into a single object?
const ports = new Set<MessagePort>();
const heartbeatTimers = new Map<MessagePort, ReturnType<typeof setTimeout>>();

let syncer: Syncer;
let apiUrl: string;
let token: string;

// const db = openDB("public", 1); // shared cache
// const db = openDB("private", 1); // per session cache

function disconnectPort(port: MessagePort) {
	ports.delete(port);
	const timer = heartbeatTimers.get(port);
	if (timer) {
		clearTimeout(timer);
		heartbeatTimers.delete(port);
	}
	port.close();
}

function resetHeartbeat(port: MessagePort) {
	const oldTimer = heartbeatTimers.get(port);
	if (oldTimer) clearTimeout(oldTimer);

	const timer = setTimeout(() => {
		disconnectPort(port);
	}, HEATBEAT_INTERVAL_REQUIRED);

	heartbeatTimers.set(port, timer);
}

function broadcast(message: WorkerEvent) {
	for (const p of ports) {
		p.postMessage(message);
	}
}

function setupSyncer(url: string, t: string) {
	apiUrl = url;
	token = t;

	syncer = new Syncer({
		apiUrl,
		token,

		// TODO: allow configuring
		compress: "deflate",
		format: "msgpack",
	});

	syncer.on("sync", (msg) => {
		broadcast({ type: "sync", message: msg });
	});

	// TODO: handle other events
	// syncer.on("error")
	// syncer.on("ready")
	// syncer.on("state")

	syncer.start();
}

self.addEventListener("connect", (e) => {
	const port = e.ports[0] as MessagePort;

	const send = (m: WorkerEvent) => port.postMessage(m);

	port.addEventListener("message", (e: MessageEvent<WorkerCommand>) => {
		const m = e.data;
		switch (m.type) {
			case "connect": {
				if (!syncer) {
					setupSyncer(m.api_url, m.token);
				}

				ports.add(port);

				// TODO: calculate Ready, Ambient, and other initial events
				// then send it to the port
				// NOTE: how do i handle fake `seq`s?
				// port.postMessage({ type: "sync", message: { op: "Ready", user, conn, seq } } as WorkerEvent)
				// port.postMessage({ type: "sync", message: { op: "Sync", data, seq, nonce } } as WorkerEvent)
				break;
			}

			case "heartbeat": {
				resetHeartbeat(port);
				break;
			}

			// TODO: replace `send` with more specific commands
			// then eg. make sure member list subscriptions are handled correctly without conflicts
			case "send": {
				if (!syncer) {
					console.warn(
						"got `send` command but syncer isn't open yet",
						m.message,
					);
				}

				syncer.send(m.message);
				break;
			}

			// TODO: deduplicate requests
			case "fetch": {
				const { method, path, params, query, headers, body } = m.request;

				// 1. Path param interpolation
				let urlPath = path;
				for (const [key, value] of Object.entries(params ?? {})) {
					urlPath = urlPath.replace(`{${key}}`, encodeURIComponent(value));
				}

				// 2. URL construction
				const url = new URL(urlPath, apiUrl);
				for (const [key, value] of Object.entries(query ?? {})) {
					url.searchParams.set(key, value);
				}

				// 3. Headers
				const requestHeaders = new Headers(headers);
				requestHeaders.set("Authorization", `Bearer ${token}`);
				if (body) {
					// NOTE: body doesnt necessarily mean application/json?
					// well, this only really applies for the media upload route
					// and technically you can use form data for inline media uploads but eh
					requestHeaders.set("Content-Type", "application/json");
				}

				// 4. Fetch
				self
					.fetch(url.toString(), {
						method,
						headers: requestHeaders,
						body: body ? JSON.stringify(body) : undefined,
					})
					.then(async (res) => {
						send({
							type: "fetched",
							nonce: m.nonce,
							body: res,
						});
					})
					.catch((error) => {
						send({
							type: "error",
							error,
						});
					});
				break;
			}
		}
	});

	port.start();
});
