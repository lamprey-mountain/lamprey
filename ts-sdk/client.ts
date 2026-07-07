import { pack, unpack } from "msgpackr";
import type * as oapi from "openapi-fetch";
import createFetch from "openapi-fetch";
import { JsonStream } from "./lib/json-stream.ts";
import { MsgpackStream } from "./lib/msgpack-stream.ts";
import { createObservable, type Observer } from "./observable.ts";
import type { paths } from "./schema.d.ts";
import type { MessageEnvelope, MessageReady, MessageSync } from "./types.ts";

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

	getWebsocket: () => WebSocket;

	/** Send a message to the sync server, queueing if not connected */
	send: (data: unknown) => void;

	/** Subscribe to sync events */
	onSync: (
		listener: (msg: MessageSync, raw: MessageEnvelope) => void,
	) => () => void;
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
		getWebsocket: () => ws,
		send,
		stopAggressive,
		onSync: (listener) => {
			syncListeners.add(listener);
			return () => {
				syncListeners.delete(listener);
			};
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
