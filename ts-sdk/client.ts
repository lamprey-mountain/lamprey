import { pack, unpack } from "msgpackr";
import type * as oapi from "openapi-fetch";
import createFetch from "openapi-fetch";
import { JsonStream } from "./lib/json-stream.ts";
import { MsgpackStream } from "./lib/msgpack-stream.ts";
import { createObservable, type Observer } from "./observable.ts";
import type { paths } from "./schema.d.ts";
import type {
	MessageClient,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	ServerInfo,
} from "./types.ts";

export * from "./observable.ts";

export type ClientState = "stopped" | "connecting" | "connected" | "ready";

export type ClientOptions = {
	apiUrl: string;
	token?: string;
	onReady: (event: MessageReady) => void;
	onSync: (event: MessageSync, raw: MessageEnvelope) => void;
	onError?: (error: Error) => void;
	onSend?: (data: unknown) => void;
	onMessage?: (raw: MessageEnvelope) => void;
	onStreamOpen?: (stream: StreamInfo) => void;
	onStreamClose?: (stream: StreamInfo) => void;
	format?: "json" | "msgpack";
	compress?: "deflate";
};

export type Http = oapi.Client<paths>;

export type Client = {
	opts: ClientOptions;

	/** Typed fetch */
	http: Http;

	/** Start receiving events */
	start: (token?: string) => void;

	/** Stop receiving events */
	stop: () => void;

	/** Stop receiving events, clear token and resume state */
	stopAggressive: () => void;

	state: Observer<ClientState>;

	/** Send a message to the sync server, queueing if not connected */
	send: (data: MessageClient) => void;

	/** Subscribe to sync events */
	onSync: (
		listener: (msg: MessageSync, raw: MessageEnvelope) => void,
	) => () => void;

	// TODO: isWebtransport: this is WebtransportClient;
	isWebtransport: boolean;
};

type Resume = {
	conn: string;
	seq: number;
};

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let resume: null | Resume = null;
	const state = createObservable<ClientState>("stopped");
	const queue: Array<unknown> = [];
	const format = opts.format ?? "json";

	const syncListeners = new Set<
		(msg: MessageSync, raw: MessageEnvelope) => void
	>();

	function handleMessage(msg: MessageEnvelope) {
		opts.onMessage?.(msg);
		switch (msg.op) {
			case "Ping": {
				send({ type: "Pong" }, true);
				break;
			}
			case "Sync": {
				if (resume) resume.seq = msg.seq;
				opts.onSync(msg.data, msg);
				for (const listener of syncListeners) {
					listener(msg.data, msg);
				}
				break;
			}
			case "Ready": {
				opts.onReady(msg);
				resume = { conn: msg.conn, seq: msg.seq };
				state.set("ready");
				flushQueue();
				break;
			}
			case "Resumed": {
				state.set("ready");
				flushQueue();
				break;
			}
			case "Error": {
				opts.onError?.(new Error(msg.error));
				break;
			}
			case "Reconnect": {
				if (!msg.can_resume) resume = null;
				ws?.close();
				break;
			}
		}
	}

	function packData(data: unknown): ArrayBuffer {
		const packed = pack(data);
		return packed.buffer.slice(
			packed.byteOffset,
			packed.byteOffset + packed.byteLength,
		) as ArrayBuffer;
	}

	const http = createFetch<paths>({
		baseUrl: opts.apiUrl,
	});

	http.use({
		onRequest(r) {
			if (opts.token) {
				r.request.headers.set("authorization", `Bearer ${opts.token}`);
			}
			return r.request;
		},
	});

	function setState(newState: ClientState) {
		state.set(newState);
	}

	function send(data: unknown, force = false) {
		if (state.get() === "ready" || force) {
			let msg: string | ArrayBuffer;
			if (typeof data === "string") {
				msg = data;
			} else if (format === "msgpack" && !opts.compress) {
				msg = packData(data);
			} else {
				msg = JSON.stringify(data);
			}

			ws?.send(msg);
			opts.onSend?.(data);
		} else {
			queue.push(data);
		}
	}

	function flushQueue() {
		while (queue.length > 0 && state.get() === "ready") {
			// TODO: can state can change from ready to something else (eg. disconnected)
			// between the while condition and send function state.get()?
			const item = queue.shift();
			if (item) send(item);
		}
	}

	function connect() {
		if (state.get() !== "connecting") return;
		const url = new URL(`/api/v1/sync?version=1&format=${format}`, opts.apiUrl);
		if (opts.compress) url.searchParams.set("compression", opts.compress);
		ws = new WebSocket(url);
		ws.binaryType = "arraybuffer";

		const streamProcessor = opts.compress
			? createDecompressor(opts.format ?? "json", handleMessage, opts.onError)
			: null;

		ws.addEventListener("message", (e) => {
			const isBinary = e.data instanceof ArrayBuffer;

			if (isBinary && streamProcessor) {
				streamProcessor.write(new Uint8Array(e.data));
				return;
			}

			let msg: MessageEnvelope;
			if (isBinary) {
				msg = unpack(new Uint8Array(e.data));
			} else {
				msg = JSON.parse(e.data);
			}
			handleMessage(msg);
		});

		ws.addEventListener("open", (_e) => {
			setState("connected");
			send({ type: "Hello", token: opts.token, ...resume }, true);
		});

		ws.addEventListener("error", (e) => {
			if (state.get() === "stopped") return;
			setState("connecting");
			opts.onError?.(e as unknown as Error);
			ws.close();
		});

		ws.addEventListener("close", () => {
			if (state.get() === "stopped") return;
			setState("connecting");
			setTimeout(connect, 1000);
		});
	}

	function start(token?: string) {
		if (token) opts.token = token;
		setState("connecting");
		if (ws) {
			ws.close();
			connect();
		} else {
			connect();
		}
	}

	function stop() {
		setState("stopped");
		ws?.close();
	}

	function stopAggressive() {
		opts.token = undefined;
		setState("stopped");
		ws?.close();
		resume = null;
	}

	return {
		state: state.observable,
		opts,
		http,
		start,
		stop,
		send,
		stopAggressive,
		onSync: (listener) => {
			syncListeners.add(listener);
			return () => {
				syncListeners.delete(listener);
			};
		},
		isWebtransport: false,
	};
}

export type WebtransportClient = Client & {
	subscribeDocument(options: DocumentOptions): Stream;
	subscribeChannel(options: SubscribeChannelOptions): Stream;
	subscribeRoom(options: SubscribeRoomOptions): Stream;
};

export type DocumentOptions = StreamOptions & {
	channel_id: string;
	branch_id: string;
	state_vector?: string;
};

export type SubscribeChannelOptions = StreamOptions & {
	channel_id: string;
};

export type SubscribeRoomOptions = StreamOptions & {
	room_id: string;
};

export type StreamOptions = {
	onSync: (event: MessageSync, raw: MessageEnvelope, streamId: number) => void;
	onError?: (error: Error, streamId: number) => void;
	onSend?: (data: unknown, streamId: number) => void;
	onMessage?: (raw: MessageEnvelope, streamId: number) => void;
};

export type Stream = {
	close: () => void;
	send: (data: MessageClient) => void;
};

export type StreamInfo = unknown;

export function createWebtransportClient(
	opts: ClientOptions,
): WebtransportClient {
	if (!("WebTransport" in globalThis))
		throw new Error("WebTransport is not supported by client");

	const http = createFetch<paths>({
		baseUrl: opts.apiUrl,
	});

	http.use({
		onRequest(r) {
			if (opts.token) {
				r.request.headers.set("authorization", `Bearer ${opts.token}`);
			}
			return r.request;
		},
	});

	const state = createObservable<ClientState>("stopped");
	const queue: Array<unknown> = [];
	let transport: WebTransport | null = null;
	let bidiStream: WebTransportBidirectionalStream | null = null;
	let writer: WritableStreamDefaultWriter | null = null;
	let resume: null | Resume = null;
	const syncListeners = new Set<
		(msg: MessageSync, raw: MessageEnvelope) => void
	>();
	const format = opts.format ?? "json";

	function handleMessage(msg: MessageEnvelope) {
		opts.onMessage?.(msg);
		switch (msg.op) {
			case "Ping": {
				send({ type: "Pong" }, true);
				break;
			}
			case "Sync": {
				if (resume) resume.seq = msg.seq;
				opts.onSync(msg.data, msg);
				for (const listener of syncListeners) {
					listener(msg.data, msg);
				}
				break;
			}
			case "Ready": {
				opts.onReady(msg);
				resume = { conn: msg.conn, seq: msg.seq };
				state.set("ready");
				flushQueue();
				break;
			}
			case "Resumed": {
				state.set("ready");
				flushQueue();
				break;
			}
			case "Error": {
				opts.onError?.(new Error(msg.error));
				break;
			}
			case "Reconnect": {
				if (!msg.can_resume) resume = null;
				transport?.close();
				break;
			}
		}
	}

	function send(data: unknown, force = false) {
		if ((state.get() === "ready" || force) && writer) {
			const packed =
				format === "msgpack"
					? pack(data)
					: new TextEncoder().encode(JSON.stringify(data));
			const len = new Uint8Array(4);
			new DataView(len.buffer).setUint32(0, packed.length, false);
			writer.write(new Uint8Array([...len, ...packed]));
			opts.onSend?.(data);
		} else {
			queue.push(data);
		}
	}

	function flushQueue() {
		while (queue.length > 0 && state.get() === "ready") {
			const item = queue.shift();
			if (item) send(item);
		}
	}

	async function connect() {
		if (state.get() !== "connecting") return;

		try {
			// TODO: error handling
			// PERF: cache server info
			const info: ServerInfo = await http
				.GET("/api/v1/server/@self")
				.then(({ data }) => data!);

			const cert = info.features.webtransport?.certificate_hashes[0].value;
			if (!cert) throw new Error("WebTransport is not supported by server");

			const url = new URL(info.features.webtransport.sync_url);
			url.searchParams.set("version", "1");
			url.searchParams.set("format", format);
			if (opts.compress) {
				url.searchParams.set("compress", opts.compress);
			}

			transport = new WebTransport(
				url,
				// `https://localhost:4433/api/v1/sync-webtransport?version=1&format=${format}`,
				{
					serverCertificateHashes: [
						{
							algorithm: "sha-256",
							value: base64UrlDecode(cert),
						},
					],
				},
			);

			transport.closed.then((info) => {
				// TODO(later): make client emit an event for this
				console.log("closed", info);
			});

			transport.draining?.then(() => {
				// TODO(later): make client emit an event for this
				console.log("draining");
			});

			await transport.ready;
			state.set("connected");
			// TODO(later): make client emit an event for when protocol is selected

			bidiStream = await transport.createBidirectionalStream();
			writer = bidiStream.writable.getWriter();

			// FIXME: handle compression for webtransport stream

			send({ type: "Hello", token: opts.token, ...resume }, true);

			let buffer = new Uint8Array(0);
			const reader = bidiStream.readable.getReader();
			try {
				while (true) {
					const { value, done } = await reader.read();
					if (done) break;

					const newBuffer = new Uint8Array(buffer.length + value.length);
					newBuffer.set(buffer);
					newBuffer.set(value, buffer.length);
					buffer = newBuffer;

					while (buffer.length >= 4) {
						const len = new DataView(
							buffer.buffer,
							buffer.byteOffset,
							4,
						).getUint32(0, false);
						if (buffer.length >= 4 + len) {
							const payload = buffer.subarray(4, 4 + len);
							const msg =
								format === "msgpack"
									? unpack(payload)
									: JSON.parse(new TextDecoder().decode(payload));
							handleMessage(msg);
							buffer = buffer.subarray(4 + len);
						} else {
							break;
						}
					}
				}
			} finally {
				reader.releaseLock();
				writer.releaseLock();
				writer = null;
			}
		} catch (err) {
			if (state.get() === "stopped") return;
			state.set("connecting");
			opts.onError?.(err as Error);
			setTimeout(connect, 1000);
		}
	}

	function start(token?: string) {
		if (token) opts.token = token;
		state.set("connecting");
		connect();
	}

	function stop() {
		state.set("stopped");
		writer?.releaseLock();
		writer = null;
		transport?.close();
	}

	function stopAggressive() {
		opts.token = undefined;
		state.set("stopped");
		writer?.releaseLock();
		writer = null;
		transport?.close();
		resume = null;
	}

	/** open a new stream */
	let streamIdCounter = 0;
	const subscribe = (options: StreamOptions): Stream => {
		// TODO: wait until transport is ready before opening stream (eg. if Hello hasn't been sent yet)
		if (!transport) throw new Error("transport is closed");

		const streamId = streamIdCounter++;
		opts.onStreamOpen?.(streamId);

		let writer: WritableStreamDefaultWriter | null = null;
		let reader: ReadableStreamDefaultReader | null = null;
		let closed = false;

		const streamPromise = transport.createBidirectionalStream();

		const queue: Array<unknown> = [];
		let isQueueDraining = false;

		const drainQueue = async () => {
			if (isQueueDraining) return;
			if (!writer) return;
			isQueueDraining = true;

			while (queue.length > 0 && !closed) {
				const item = queue.shift();
				const packed =
					format === "msgpack"
						? pack(item)
						: new TextEncoder().encode(JSON.stringify(item));
				const len = new Uint8Array(4);
				new DataView(len.buffer).setUint32(0, packed.length, false);
				writer.write(new Uint8Array([...len, ...packed]));
				// TODO: call opts.onSomething when sending message
				// console.log("AAA send", item);
				options.onSend?.(item, streamId);
				// PERF: call scheduler.yield() here if it exists?
			}

			isQueueDraining = false;
		};

		const send = (data: unknown) => {
			if (closed) throw new Error("transport closed");
			queue.push(data);
			drainQueue();
		};

		const close = () => {
			if (closed) return;
			opts.onStreamClose?.(streamId);
			closed = true;
			reader?.releaseLock();
			writer?.close();
			reader = null;
			writer = null;
		};

		(async () => {
			const stream = await streamPromise;
			if (closed) {
				// NOTE: maybe i want writable.abort() instead?
				stream.writable.close();
				return;
			}

			writer = stream.writable.getWriter();
			reader = stream.readable.getReader();

			drainQueue();

			let buffer = new Uint8Array(0);
			try {
				while (true) {
					const { value, done } = await reader.read();
					if (done) break;

					const newBuffer = new Uint8Array(buffer.length + value.length);
					newBuffer.set(buffer);
					newBuffer.set(value, buffer.length);
					buffer = newBuffer;

					while (buffer.length >= 4) {
						const len = new DataView(
							buffer.buffer,
							buffer.byteOffset,
							4,
						).getUint32(0, false);
						if (buffer.length >= 4 + len) {
							const payload = buffer.subarray(4, 4 + len);
							const msg: MessageEnvelope =
								format === "msgpack"
									? unpack(payload)
									: JSON.parse(new TextDecoder().decode(payload));
							// TODO: call opts.onSomething when receiving message
							// console.log("AAA recv", msg);
							options.onMessage?.(msg, streamId);
							switch (msg.op) {
								case "Ping": {
									send({ type: "Pong" });
									break;
								}
								case "Sync": {
									options.onSync(msg.data, msg, streamId);
									break;
								}
								case "Error": {
									options.onError?.(new Error(msg.error), streamId);
									break;
								}
							}
							buffer = buffer.subarray(4 + len);
						} else {
							break;
						}
					}
				}
			} catch (err) {
				// NOTE: should i also call opts.onError here? should onError take another stream arg?
				options.onError?.(err as Error, streamId);
			} finally {
				if (!closed) close();
			}
		})();

		return {
			send,
			close,
		};
	};

	return {
		opts,
		http,
		state: state.observable,
		start,
		stop,
		stopAggressive,
		send,
		onSync: (listener) => {
			syncListeners.add(listener);
			return () => {
				syncListeners.delete(listener);
			};
		},
		isWebtransport: true,

		subscribeDocument(options: DocumentOptions): Stream {
			const stream = subscribe(options);

			stream.send({
				type: "DocumentSubscribe",
				channel_id: options.channel_id,
				branch_id: options.branch_id,
				state_vector: options.state_vector,
			});

			return stream;
		},

		subscribeChannel(options: SubscribeChannelOptions): Stream {
			const stream = subscribe(options);

			stream.send({
				type: "ChannelSubscribe",
				channel_id: options.channel_id,
			});

			return stream;
		},

		subscribeRoom(options: SubscribeRoomOptions): Stream {
			const stream = subscribe(options);

			stream.send({
				type: "RoomSubscribe",
				room_id: options.room_id,
			});

			return stream;
		},
	};
}

// TEMP: reexport
export { UUID_MAX, UUID_MIN } from "./core/uuid.ts";

function createDecompressor(
	format: "json" | "msgpack",
	onMessage: (msg: MessageEnvelope) => void,
	onError?: (err: Error) => void,
) {
	const stream = new DecompressionStream("deflate");
	const deserializer =
		format === "json" ? new JsonStream() : new MsgpackStream();
	const reader = stream.readable.pipeThrough(deserializer).getReader();

	(async () => {
		try {
			while (true) {
				const { value, done } = await reader.read();
				if (done) break;
				if (value) onMessage(value as MessageEnvelope);
			}
		} catch (err) {
			onError?.(err as Error);
		}
	})();

	return stream.writable.getWriter();
}

// TODO: deduplicate with frontend/src/components/features/editor/editor-utils.ts (extract base64 functions into ts-sdk, make frontend import from sdk)
/** decode unpadded url safe base64 */
export function base64UrlDecode(str: string): Uint8Array {
	str = str.replace(/-/g, "+").replace(/_/g, "/");

	const pad = str.length % 4;
	if (pad) {
		str += "=".repeat(4 - pad);
	}

	const binary = atob(str);
	const bytes = new Uint8Array(binary.length);

	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}

	return bytes;
}
