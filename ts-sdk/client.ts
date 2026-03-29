import createFetch from "openapi-fetch";
import type * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import type { MessageEnvelope, MessageReady, MessageSync } from "./types.ts";
import { createObservable, type Observer } from "./observable.ts";
import { pack, unpack, Unpackr, UnpackrStream } from "msgpackr";
export * from "./observable.ts";

export type ClientState = "stopped" | "connecting" | "connected" | "ready";

export type ClientOptions = {
	apiUrl: string;
	token?: string;
	onReady: (event: MessageReady) => void;
	onSync: (event: MessageSync, raw: MessageEnvelope) => void;
	onError?: (error: Error) => void;
	onSend?: (data: any) => void;
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

	state: Observer<ClientState>;

	getWebsocket: () => WebSocket;

	/** Send a message to the sync server, queueing if not connected */
	send: (data: any) => void;
};

type Resume = {
	conn: string;
	seq: number;
};

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let resume: null | Resume = null;
	const state = createObservable<ClientState>("stopped");
	const queue: Array<any> = [];
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

	function packData(data: any): ArrayBuffer {
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

	function send(data: any, force = false) {
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
			send(queue.shift()!);
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

	return {
		state: state.observable,
		opts,
		http,
		start,
		stop,
		getWebsocket: () => ws,
		send,
	};
}

export const UUID_MIN = "00000000-0000-0000-0000-000000000000";
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";

function createDecompressor(
	format: "json" | "msgpack",
	onMessage: (msg: MessageEnvelope) => void,
	onError?: (err: Error) => void,
) {
	const stream = new DecompressionStream("deflate");
	const deserializer = format === "json"
		? new JsonStream()
		: new MsgpackStream();
	const reader = stream.readable.pipeThrough(deserializer).getReader();

	(async () => {
		try {
			while (true) {
				const { value, done } = await reader.read();
				if (done) break;
				if (value) onMessage(value);
			}
		} catch (err) {
			onError?.(err as Error);
		}
	})();

	return stream.writable.getWriter();
}

export class MsgpackStream extends TransformStream<Uint8Array, any> {
	constructor() {
		const unpacker = new Unpackr({
			mapsAsObjects: true,
		});

		let buffer = new Uint8Array(0);

		super({
			transform(chunk, controller) {
				const combined = new Uint8Array(buffer.length + chunk.length);
				combined.set(buffer);
				combined.set(chunk, buffer.length);
				buffer = combined;

				let consumedUntil = 0;

				try {
					unpacker.unpackMultiple(buffer, (value, _start, end) => {
						controller.enqueue(value);
						if (end !== undefined) {
							consumedUntil = end;
						} else console.warn("no end!");
					});

					buffer = buffer.slice(consumedUntil);
				} catch (e) {
					// if it throws, we have a partial message
					if (consumedUntil > 0) {
						buffer = buffer.slice(consumedUntil);
					}
				}
			},
			flush() {
				buffer = new Uint8Array(0);
			},
		});
	}
}

export class JsonStream extends TransformStream<Uint8Array, any> {
	constructor() {
		const decoder = new TextDecoder();
		let buffer = "";

		super({
			transform(chunk, controller) {
				buffer += decoder.decode(chunk, { stream: true });

				while (true) {
					const endIdx = findJsonEnd(buffer);
					if (endIdx === -1) break;

					const raw = buffer.slice(0, endIdx);
					try {
						const obj = JSON.parse(raw);
						controller.enqueue(obj);
					} catch (e) {
						console.error("Failed to parse JSON segment:", e);
					}

					buffer = buffer.slice(endIdx).trimStart();
				}
			},
			flush(controller) {
				const final = decoder.decode();
				if (final.trim()) {
					try {
						controller.enqueue(JSON.parse(final));
					} catch {}
				}
			},
		});
	}
}

/**
 * Finds the end of the first complete JSON object in a string.
 * Returns -1 if the object is incomplete.
 */
function findJsonEnd(str: string): number {
	let braces = 0;
	let inString = false;
	let escaped = false;
	let started = false;

	for (let i = 0; i < str.length; i++) {
		const char = str[i];
		if (escaped) {
			escaped = false;
			continue;
		}
		if (char === "\\") {
			escaped = true;
			continue;
		}
		if (char === '"') {
			inString = !inString;
			continue;
		}
		if (inString) continue;

		if (char === "{") {
			braces++;
			started = true;
		} else if (char === "}") {
			braces--;
			if (started && braces === 0) return i + 1;
		}
	}
	return -1;
}
