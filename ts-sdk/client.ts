import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import { MessageEnvelope, MessageReady, MessageSync } from "./types.ts";
import { createObservable, Observer } from "./observable.ts";
export * from "./observable.ts";

export type ClientState = "stopped" | "connecting" | "connected" | "ready";

export type ClientOptions = {
	apiUrl: string;
	token?: string;
	onReady: (event: MessageReady) => void;
	onSync: (event: MessageSync) => void;
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
};

type Resume = {
	conn: string;
	seq: number;
};

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let resume: null | Resume = null;
	const state = createObservable<ClientState>("stopped");

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

	function setupWebsocket() {
		if (state.get() !== "connecting") return;

		ws = new WebSocket(new URL("/api/v1/sync?version=1", opts.apiUrl));
		ws.addEventListener("message", (e) => {
			const msg: MessageEnvelope = JSON.parse(e.data);
			if (msg.op === "Ping") {
				ws.send(JSON.stringify({ type: "Pong" }));
			} else if (msg.op === "Sync") {
				if (resume) resume.seq = msg.seq;
				opts.onSync(msg.data);
			} else if (msg.op === "Error") {
				console.error(msg.error);
			} else if (msg.op === "Ready") {
				opts.onReady(msg);
				resume = { conn: msg.conn, seq: msg.seq };
				setState("ready");
			} else if (msg.op === "Resumed") {
				setState("ready");
			} else if (msg.op === "Reconnect") {
				if (!msg.can_resume) resume = null;
				ws.close();
			}
		});

		ws.addEventListener("open", (_e) => {
			setState("connected");
			ws.send(JSON.stringify({ type: "Hello", token: opts.token, ...resume }));
		});

		ws.addEventListener("error", (e) => {
			if (state.get() === "stopped") return;
			setState("connecting");
			console.error(e);
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
	};
}

export const UUID_MIN = "00000000-0000-0000-0000-000000000000";
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";
