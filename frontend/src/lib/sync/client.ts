import {
	type Client,
	type ClientOptions,
	type ClientState,
	createClient,
	createFetch,
	createObservable,
	type DocumentOptions,
	type MessageClient,
	type MessageEnvelope,
	type MessageReady,
	type MessageSync,
	type paths,
	ratelimitMiddleware,
	type Stream,
	type SubscribeChannelOptions,
	type SubscribeRoomOptions,
} from "sdk";
import { openApiDb } from "./db";
import { applyReadyToDb, applySyncToDb } from "./idb-sync";
import {
	HEARTBEAT_INTERVAL,
	type StreamSubscribeOptions,
	type WorkerRequest,
	type WorkerResponse,
} from "./protocol";

type StreamOptions = {
	onSync: (event: MessageSync, raw: MessageEnvelope, streamId: number) => void;
	onMessage?: (raw: MessageEnvelope, streamId: number) => void;
	onError?: (error: Error, streamId: number) => void;
	onSend?: (data: unknown, streamId: number) => void;
};

function subscribeOptions(
	kind: StreamSubscribeOptions["kind"],
	options: DocumentOptions | SubscribeChannelOptions | SubscribeRoomOptions,
): StreamSubscribeOptions {
	switch (kind) {
		case "document": {
			const doc = options as DocumentOptions;
			return {
				kind,
				channel_id: doc.channel_id,
				branch_id: doc.branch_id,
				state_vector: doc.state_vector,
			};
		}
		case "channel": {
			const channel = options as SubscribeChannelOptions;
			return { kind, channel_id: channel.channel_id };
		}
		case "room": {
			const room = options as SubscribeRoomOptions;
			return { kind, room_id: room.room_id };
		}
	}
}

export function createSharedClient(opts: ClientOptions): Client {
	if (!("SharedWorker" in globalThis)) return createDirectClient(opts);

	try {
		return createWorkerClient(opts);
	} catch (error) {
		console.error("failed to create shared sync worker", error);
		return createDirectClient(opts);
	}
}

function createDirectClient(opts: ClientOptions): Client {
	const dbPromise = openApiDb().catch((error: unknown) => {
		console.error("failed to open api database", error);
		return undefined;
	});
	let ready: MessageReady | null = null;
	let queue = Promise.resolve();

	const enqueue = (task: () => Promise<void>) => {
		queue = queue.then(task).catch((error) => {
			console.error("failed to update api database", error);
		});
	};

	return createClient({
		...opts,
		onReady(event) {
			ready = event;
			enqueue(async () => {
				const db = await dbPromise;
				if (db) await applyReadyToDb(db, event);
			});
			opts.onReady(event);
		},
		onSync(event, raw) {
			enqueue(async () => {
				const db = await dbPromise;
				if (!db) return;
				const user_id =
					ready?.session.status !== "Unauthorized"
						? ready?.session.user_id
						: undefined;
				await applySyncToDb(db, event, user_id);
			});
			opts.onSync(event, raw);
		},
	});
}

function createWorkerClient(opts: ClientOptions): Client {
	const worker = new SharedWorker(new URL("./worker.ts", import.meta.url), {
		type: "module",
		name: "lamprey-sync",
	});
	const port = worker.port;
	const state = createObservable<ClientState>("stopped");
	const http = createFetch<paths>({ baseUrl: opts.apiUrl });
	const syncListeners = new Set<
		(msg: MessageSync, raw: MessageEnvelope) => void
	>();
	const streams = new Map<
		number,
		{ options: StreamOptions; closed: boolean }
	>();

	let streamIdCounter = 0;
	let webtransport = false;

	http.use({
		onRequest(request) {
			if (opts.token) {
				request.request.headers.set("authorization", `Bearer ${opts.token}`);
			}
			return request.request;
		},
	});

	http.use(ratelimitMiddleware({ onRatelimit: opts.onRatelimit }));

	const send = (message: WorkerRequest) => port.postMessage(message);

	const closeStream = (streamId: number) => {
		const stream = streams.get(streamId);
		if (!stream || stream.closed) return;
		stream.closed = true;
		streams.delete(streamId);
		opts.onStreamClose?.(streamId);
	};

	port.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
		const message = event.data;
		switch (message.type) {
			case "state":
				state.set(message.state);
				break;
			case "capabilities":
				webtransport = message.webtransport;
				break;
			case "ready":
				opts.onReady(message.event);
				break;
			case "sync": {
				opts.onMessage?.(message.raw);
				opts.onSync(message.event, message.raw);
				for (const listener of syncListeners) {
					listener(message.event, message.raw);
				}
				break;
			}
			case "stream-sync": {
				const stream = streams.get(message.streamId);
				if (!stream || stream.closed) break;
				stream.options.onMessage?.(message.raw, message.streamId);
				stream.options.onSync(message.event, message.raw, message.streamId);
				break;
			}
			case "stream-error": {
				const stream = streams.get(message.streamId);
				if (!stream || stream.closed) {
					opts.onError?.(new Error(message.error));
					break;
				}
				stream.options.onError?.(new Error(message.error), message.streamId);
				break;
			}
			case "stream-close":
				closeStream(message.streamId);
				break;
			case "error":
				opts.onError?.(new Error(message.error));
				break;
		}
	});

	port.start();

	const heartbeat = setInterval(
		() => send({ type: "heartbeat" }),
		HEARTBEAT_INTERVAL,
	);

	const subscribe = (
		kind: StreamSubscribeOptions["kind"],
		options: DocumentOptions | SubscribeChannelOptions | SubscribeRoomOptions,
	): Stream => {
		if (!webtransport) throw new Error("WebTransport is not available");

		const streamId = streamIdCounter++;
		const handlers: StreamOptions = {
			onSync: options.onSync,
			onMessage: options.onMessage,
			onError: options.onError,
			onSend: options.onSend,
		};

		streams.set(streamId, { options: handlers, closed: false });
		opts.onStreamOpen?.(streamId);
		send({
			type: "stream-open",
			streamId,
			options: subscribeOptions(kind, options),
		});

		return {
			send(message: MessageClient) {
				const stream = streams.get(streamId);
				if (!stream || stream.closed) throw new Error("transport closed");
				handlers.onSend?.(message as unknown, streamId);
				send({ type: "stream-send", streamId, message });
			},
			close() {
				const stream = streams.get(streamId);
				if (!stream || stream.closed) return;
				stream.closed = true;
				streams.delete(streamId);
				opts.onStreamClose?.(streamId);
				send({ type: "stream-close", streamId });
			},
		};
	};

	return {
		opts,
		http,
		state: state.observable,
		start(token?: string) {
			if (token) opts.token = token;
			state.set("connecting");
			send({
				type: "connect",
				options: {
					apiUrl: opts.apiUrl,
					token: opts.token,
					format: opts.format,
					compress: opts.compress,
					preferWebtransport: opts.preferWebtransport,
				},
			});
		},
		stop() {
			clearInterval(heartbeat);
			state.set("stopped");
			send({ type: "stop" });
			port.close();
		},
		stopAggressive() {
			opts.token = undefined;
			state.set("stopped");
			send({ type: "stop-aggressive" });
		},
		send(message: MessageClient) {
			send({ type: "send", message });
		},
		onSync(listener) {
			syncListeners.add(listener);
			return () => {
				syncListeners.delete(listener);
			};
		},
		get isWebtransport() {
			return webtransport;
		},
		subscribeDocument(options) {
			return subscribe("document", options);
		},
		subscribeChannel(options) {
			return subscribe("channel", options);
		},
		subscribeRoom(options) {
			return subscribe("room", options);
		},
	};
}
