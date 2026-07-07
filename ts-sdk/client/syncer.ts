import { pack, unpack } from "msgpackr";
import { Emitter } from "../core/events.ts";
import { JsonStream } from "../lib/json-stream.ts";
import { MsgpackStream } from "../lib/msgpack-stream.ts";
import type {
	MessageClient,
	MessageEnvelope,
	MessageReady,
	MessageSync,
} from "../types.ts";

export type SyncerState = "stopped" | "connecting" | "connected" | "ready";

export type SyncerEvents = {
	sync: MessageEnvelope;
	error: Error;
	ready: MessageReady;
	state: SyncerState;
};

type Resume = {
	conn: string;
	seq: number;
};

type SyncerOptions = {
	apiUrl: string;
	token: string;
	format?: "json" | "msgpack";
	compress?: "deflate";
};

export class Syncer extends Emitter<SyncerEvents> {
	private ws?: WebSocket;
	private resume?: Resume;
	private st = "stopped" as SyncerState;
	private sendQueue: Array<MessageClient> = [];

	private apiUrl: string;
	private token: string;
	private format: "json" | "msgpack";
	private compress?: "deflate";

	constructor(opts: SyncerOptions) {
		super();
		this.format = opts.format ?? "json";
		this.compress = opts.compress;
		this.apiUrl = opts.apiUrl;
		this.token = opts.token;
	}

	private setState(state: SyncerState) {
		this.st = state;
		this.emit("state", state);
	}

	private handleMessage(msg: MessageEnvelope) {
		switch (msg.op) {
			case "Ping": {
				this.sendInner({ type: "Pong" }, true);
				break;
			}
			case "Sync": {
				if (this.resume) this.resume.seq = msg.seq;
				this.emit("sync", msg);
				break;
			}
			case "Ready": {
				this.emit("ready", msg as MessageReady);
				this.resume = { conn: msg.conn, seq: msg.seq };
				this.setState("ready");
				this.flushQueue();
				break;
			}
			case "Resumed": {
				this.setState("ready");
				this.flushQueue();
				break;
			}
			case "Error": {
				this.emit("error", new Error(msg.error));
				break;
			}
			case "Reconnect": {
				if (!msg.can_resume) this.resume = undefined;
				this.ws?.close();
				break;
			}
		}
	}

	private packData(data: unknown): ArrayBuffer {
		const packed = pack(data);
		return packed.buffer.slice(
			packed.byteOffset,
			packed.byteOffset + packed.byteLength,
		) as ArrayBuffer;
	}

	private sendInner(m: MessageClient, force = false) {
		if (this.st === "ready" || force) {
			let msg: string | ArrayBuffer;
			if (typeof m === "string") {
				msg = m;
			} else if (this.format === "msgpack" && !this.compress) {
				msg = this.packData(m);
			} else {
				msg = JSON.stringify(m);
			}

			this.ws?.send(msg);
		} else {
			this.sendQueue.push(m);
		}
	}

	private flushQueue() {
		while (this.sendQueue.length > 0 && this.st === "ready") {
			const item = this.sendQueue.shift();
			if (item) this.sendInner(item);
		}
	}

	private connect() {
		if (this.st !== "connecting") return;
		this.ws?.close();

		const url = new URL(
			`/api/v1/sync?version=1&format=${this.format}`,
			this.apiUrl,
		);
		if (this.compress) url.searchParams.set("compression", this.compress);
		this.ws = new WebSocket(url);
		this.ws.binaryType = "arraybuffer";

		const streamProcessor = this.compress
			? createDecompressor(
					this.format,
					(msg) => this.handleMessage(msg),
					(err) => this.emit("error", err),
				)
			: null;

		this.ws.addEventListener("message", (e) => {
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
			this.handleMessage(msg);
		});

		this.ws.addEventListener("open", (_e) => {
			this.setState("connected");
			this.sendInner(
				// HACK: openapi thinks resume is required
				{
					type: "Hello",
					token: this.token,
					...this.resume,
				} as unknown as MessageClient,
				true,
			);
		});

		this.ws.addEventListener("error", (e) => {
			if (this.st === "stopped") return;
			this.setState("connecting");
			this.emit("error", e as unknown as Error);
			this.ws?.close();
		});

		this.ws.addEventListener("close", () => {
			if (this.st === "stopped") return;
			this.setState("connecting");
			// TODO: exponential backoff
			setTimeout(() => this.connect(), 1000);
		});
	}

	/** open the syncer connection and begin syncing */
	public start() {
		this.setState("connecting");
		this.connect();
	}

	/** close the syncer connection */
	public stop() {
		this.setState("stopped");
		this.ws?.close();
	}

	/** switch the current token, reconnect, and reauthenticate */
	public authenticate(token: string) {
		this.token = token;
		this.start();
	}

	/** Send a message to the sync server, queueing if not connected */
	public send(message: MessageClient) {
		this.sendInner(message);
	}

	/** get the current state of this syncer */
	public get state(): SyncerState {
		return this.st;
	}

	// TODO: getters for compression, format, maybe token?
}

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
