import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import { MessageEnvelope, MessageReady, MessageSync } from "./types.ts";
import { createObservable, Observer } from "./observable.ts";
import { pack, unpack, unpackMultiple } from "msgpackr";
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
	const useMsgpack = format === "msgpack";

	const dec = new DecompressionStream("deflate");
	const decReader = dec.readable.getReader();
	const decWriter = dec.writable.getWriter();

	let buffer = new Uint8Array(0);

	async function processDecompressedStream() {
		try {
			const { value, done } = await decReader.read();
			if (done) return;

			const newBuffer = new Uint8Array(buffer.length + value.length);
			newBuffer.set(buffer);
			newBuffer.set(value, buffer.length);
			buffer = newBuffer;

			try {
				const msg = unpackMultiple(buffer);
				for (const m of msg) handleMessage(m);
				buffer = new Uint8Array(0);
			} catch (e) {
				console.log("Waiting for more data...");
			}
		} catch (e) {
			console.error("Decompression failed", e);
		}
	}

	const handleMessage = (msg: MessageEnvelope) => {
		opts.onMessage?.(msg);
		if (msg.op === "Ping") {
			const pong = { type: "Pong" };
			send(pong);
		} else if (msg.op === "Sync") {
			if (resume) resume.seq = msg.seq;
			opts.onSync(msg.data, msg);
		} else if (msg.op === "Error") {
			opts.onError?.(new Error(msg.error));
		} else if (msg.op === "Ready") {
			opts.onReady(msg);
			resume = { conn: msg.conn, seq: msg.seq };
			setState("ready");
			flushQueue();
		} else if (msg.op === "Resumed") {
			setState("ready");
			flushQueue();
		} else if (msg.op === "Reconnect") {
			if (!msg.can_resume) resume = null;
			ws.close();
		}
	};

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

	function flushQueue() {
		while (queue.length > 0 && state.get() === "ready") {
			const item = queue.shift()!;
			send2(item);
		}
	}

	function send2(data: any) {
		let msg: string | ArrayBuffer;
		if (typeof data === "string") {
			msg = data;
		} else if (useMsgpack && !opts.compress) {
			msg = packData(data);
		} else {
			msg = JSON.stringify(data);
		}

		ws.send(msg);
		opts.onSend?.(data);
	}

	function send(data: any, force = false) {
		// maybe i should only send it when its sent to the websocket
		// right now it might *say* its sent, but actually be in the queue
		if (state.get() === "ready" || force) {
			send2(data);
		} else {
			queue.push(data);
		}
	}

	function setupWebsocket() {
		if (state.get() !== "connecting") return;
		const url = new URL(`/api/v1/sync?version=1&format=${format}`, opts.apiUrl);
		if (opts.compress) url.searchParams.set("compression", opts.compress);
		ws = new WebSocket(url);
		ws.binaryType = "arraybuffer";

		ws.addEventListener("message", (e) => {
			if (opts.compress) {
				decWriter.write(new Uint8Array(e.data));
				processDecompressedStream();
				return;
			}

			let msg: MessageEnvelope;
			if (e.data instanceof ArrayBuffer) {
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
			setTimeout(setupWebsocket, 1000);
		});
	}

	function start(token?: string) {
		if (token) opts.token = token;
		setState("connecting");
		if (ws) {
			ws.close();
			setupWebsocket();
		} else {
			setupWebsocket();
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
