/// <reference no-default-lib="true"/>
/// <reference lib="webworker" />

declare const self: SharedWorkerGlobalScope;

import type { IDBPDatabase } from "idb";
import {
	type Client,
	type ClientState,
	createClient,
	type MessageClient,
	type MessageEnvelope,
	type MessageReady,
	type MessageSync,
	type PreferencesGlobal,
	type Stream,
} from "sdk";
import { type ApiDB, openApiDb } from "./db";
import {
	applyReadyToDb,
	applySyncToDb,
	getSyncMeta,
	idbDelete,
	readAmbientFromDb,
} from "./idb-sync";
import {
	HEARTBEAT_TIMEOUT,
	type StreamSubscribeOptions,
	type SyncConnectionOptions,
	type WorkerRequest,
	type WorkerResponse,
} from "./protocol";

type PortData = {
	port: MessagePort;
	live: boolean;
	pending: WorkerResponse[];
	heartbeat?: ReturnType<typeof setTimeout>;
};

type StreamSubscription = {
	stream: Stream;
	subscribers: Map<MessagePort, Set<number>>;
	buffer?: Array<{ event: MessageSync; raw: MessageEnvelope }>;
};

const ports = new Map<MessagePort, PortData>();
const portStreams = new Map<MessagePort, Map<number, string>>();
const subscriptions = new Map<string, StreamSubscription>();

let client: Client | null = null;
let connection: SyncConnectionOptions | null = null;
let ready: MessageReady | null = null;
let webtransport = false;

type SyncDb = IDBPDatabase<ApiDB>;

let dbPromise: Promise<SyncDb> | null = null;
let queue = Promise.resolve();

function getDb(): Promise<SyncDb> {
	dbPromise ??= openApiDb();
	return dbPromise;
}

function enqueue(task: () => Promise<void> | void) {
	queue = queue.then(task).catch((error) => {
		console.error("sync worker database task failed", error);
	});
}

function post(port: MessagePort, message: WorkerResponse) {
	try {
		port.postMessage(message);
	} catch {
		closePort(port);
	}
}

function send(port: MessagePort, message: WorkerResponse) {
	const data = ports.get(port);
	if (!data) return;
	if (data.live) post(port, message);
	else data.pending.push(message);
}

function broadcast(message: WorkerResponse) {
	for (const [port, data] of ports) {
		if (data.live) post(port, message);
		else data.pending.push(message);
	}
}

function setState(state: ClientState) {
	broadcast({ type: "state", state });
	const next = client?.isWebtransport ?? false;
	if (next !== webtransport) {
		webtransport = next;
		broadcast({ type: "capabilities", webtransport });
	}
}

function resetConnectionState() {
	for (const [port, local] of portStreams) {
		for (const streamId of local.keys()) {
			send(port, { type: "stream-close", streamId });
		}
	}
	for (const subscription of subscriptions.values())
		subscription.stream.close();
	subscriptions.clear();
	portStreams.clear();

	ready = null;
	webtransport = false;
	broadcast({ type: "capabilities", webtransport: false });
	enqueue(async () => {
		const db = await getDb();
		await idbDelete(db, "sync_meta", "ready");
		await idbDelete(db, "sync_meta", "config");
	});
}

function readReady(event: MessageReady) {
	ready = event;
	enqueue(async () => {
		const db = await getDb();
		await applyReadyToDb(db, event);
		broadcast({ type: "ready", event });
	});
}

function readSync(event: MessageSync, raw: MessageEnvelope) {
	enqueue(async () => {
		const db = await getDb();
		const user_id =
			ready?.session.status !== "Unauthorized"
				? ready?.session.user_id
				: undefined;
		await applySyncToDb(db, event, user_id);
		broadcast({ type: "sync", event, raw });
	});
}

async function replay(port: MessagePort) {
	const data = ports.get(port);
	if (!data) return;
	data.live = false;
	data.pending.length = 0;

	const messages: WorkerResponse[] = [
		{ type: "state", state: client?.state.get() ?? "stopped" },
		{ type: "capabilities", webtransport },
	];
	if (ready) messages.push({ type: "ready", event: ready });

	try {
		const db = await getDb();
		const config = await getSyncMeta<PreferencesGlobal>(db, "config");
		const user_id =
			ready?.session.status !== "Unauthorized"
				? ready?.session.user_id
				: undefined;
		if (user_id && config) {
			const ambient = await readAmbientFromDb(db, user_id, config);
			if (ambient) {
				messages.push({
					type: "sync",
					event: ambient,
					raw: { op: "Sync", data: ambient, seq: 0 },
				});
			}
		}
	} catch (error) {
		console.error("failed to read ambient from database", error);
	}

	data.live = true;
	for (const message of messages) post(data.port, message);
	for (const message of data.pending) post(data.port, message);
	data.pending.length = 0;
}

function streamKey(options: StreamSubscribeOptions): string {
	switch (options.kind) {
		case "document":
			return `document:${options.channel_id}:${options.branch_id}:${options.state_vector ?? ""}`;
		case "channel":
			return `channel:${options.channel_id}`;
		case "room":
			return `room:${options.room_id}`;
	}
}

function removeSubscriber(port: MessagePort, streamId: number) {
	const local = portStreams.get(port);
	const key = local?.get(streamId);
	if (!local || !key) return;

	local.delete(streamId);
	if (local.size === 0) portStreams.delete(port);

	const subscription = subscriptions.get(key);
	if (!subscription) return;

	const ids = subscription.subscribers.get(port);
	if (!ids) return;
	ids.delete(streamId);
	if (ids.size === 0) subscription.subscribers.delete(port);

	if (subscription.subscribers.size === 0) {
		subscription.stream.close();
		subscriptions.delete(key);
	}
}

function fanout(
	key: string,
	message:
		| { type: "stream-sync"; event: MessageSync; raw: MessageEnvelope }
		| { type: "stream-error"; error: string }
		| { type: "stream-close" },
) {
	const subscription = subscriptions.get(key);
	if (!subscription) return;
	if (message.type === "stream-sync") {
		subscription.buffer?.push({ event: message.event, raw: message.raw });
	}
	for (const [port, ids] of subscription.subscribers) {
		for (const streamId of ids) {
			send(port, { ...message, streamId } as WorkerResponse);
		}
	}
}

function openStream(
	port: MessagePort,
	streamId: number,
	options: StreamSubscribeOptions,
) {
	if (!client?.isWebtransport) {
		send(port, {
			type: "stream-error",
			streamId,
			error: "WebTransport is not available",
		});
		return;
	}

	const key = streamKey(options);
	let subscription = subscriptions.get(key);
	const existed = subscription !== undefined;

	if (!subscription) {
		const onSync = (event: MessageSync, raw: MessageEnvelope) => {
			fanout(key, { type: "stream-sync", event, raw });
		};
		const onError = (error: Error) => {
			fanout(key, { type: "stream-error", error: error.message });
		};

		let stream: Stream;
		switch (options.kind) {
			case "document":
				stream = client.subscribeDocument!({
					channel_id: options.channel_id,
					branch_id: options.branch_id,
					state_vector: options.state_vector,
					onSync,
					onError,
				});
				break;
			case "channel":
				stream = client.subscribeChannel!({
					channel_id: options.channel_id,
					onSync,
					onError,
				});
				break;
			case "room":
				stream = client.subscribeRoom!({
					room_id: options.room_id,
					onSync,
					onError,
				});
				break;
		}

		subscription = {
			stream,
			subscribers: new Map(),
			buffer: options.kind === "document" ? [] : undefined,
		};
		subscriptions.set(key, subscription);
	}

	let ids = subscription.subscribers.get(port);
	if (!ids) {
		ids = new Set();
		subscription.subscribers.set(port, ids);
	}
	ids.add(streamId);

	let local = portStreams.get(port);
	if (!local) {
		local = new Map();
		portStreams.set(port, local);
	}
	local.set(streamId, key);

	if (existed && subscription.buffer) {
		for (const event of subscription.buffer) {
			send(port, {
				type: "stream-sync",
				streamId,
				event: event.event,
				raw: event.raw,
			});
		}
	}
}

function closePort(port: MessagePort) {
	const data = ports.get(port);
	if (!data) return;
	if (data.heartbeat) clearTimeout(data.heartbeat);
	ports.delete(port);

	const local = portStreams.get(port);
	if (local) {
		for (const streamId of local.keys()) removeSubscriber(port, streamId);
		portStreams.delete(port);
	}
	port.close();
}

function resetHeartbeat(port: MessagePort) {
	const data = ports.get(port);
	if (!data) return;
	if (data.heartbeat) clearTimeout(data.heartbeat);
	data.heartbeat = setTimeout(() => closePort(port), HEARTBEAT_TIMEOUT);
}

function ensureClient(options: SyncConnectionOptions) {
	if (client && connection) {
		const current = JSON.stringify(connection);
		const next = JSON.stringify(options);
		if (current === next) return;
	}

	if (client) {
		client.stopAggressive();
		client = null;
		connection = null;
		resetConnectionState();
	}

	connection = options;
	const next = createClient({
		apiUrl: options.apiUrl,
		token: options.token,
		format: options.format,
		compress: options.compress,
		preferWebtransport: options.preferWebtransport,
		onReady: readReady,
		onSync: readSync,
		onError(error) {
			broadcast({ type: "error", error: error.message });
		},
	});

	client = next;
	next.state.subscribe((state) => setState(state));
	next.start(options.token);
	webtransport = next.isWebtransport;
	broadcast({ type: "capabilities", webtransport });
}

self.addEventListener("connect", (event) => {
	const port = event.ports[0] as MessagePort;
	ports.set(port, { port, live: false, pending: [] });
	resetHeartbeat(port);

	port.addEventListener("message", (event: MessageEvent<WorkerRequest>) => {
		const message = event.data;
		resetHeartbeat(port);

		switch (message.type) {
			case "connect":
				ensureClient(message.options);
				enqueue(() => replay(port));
				break;
			case "send":
				client?.send(message.message);
				break;
			case "stream-open":
				openStream(port, message.streamId, message.options);
				break;
			case "stream-send": {
				const key = portStreams.get(port)?.get(message.streamId);
				if (!key) break;
				subscriptions.get(key)?.stream.send(message.message);
				break;
			}
			case "stream-close":
				removeSubscriber(port, message.streamId);
				break;
			case "stop":
				closePort(port);
				break;
			case "stop-aggressive":
				client?.stopAggressive();
				client = null;
				connection = null;
				resetConnectionState();
				for (const other of ports.keys()) {
					send(other, { type: "state", state: "stopped" });
				}
				break;
			case "heartbeat":
				break;
		}
	});

	port.start();
});

// keep the worker alive while tabs are connected
setInterval(() => {
	if (ports.size === 0 && client) {
		client.stop();
		client = null;
		connection = null;
		resetConnectionState();
	}
}, 30_000);
