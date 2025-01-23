import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import { MessageEnvelope, MessageReady, MessageSync } from "./types.ts";

export type ClientState = "stopped" | "connected" | "ready" | "reconnecting";

export type ClientOptions = {
	baseUrl: string;
	token: string;
	onReady: (event: MessageReady) => void;
	onSync: (event: MessageSync) => void;
	onState: (state: ClientState) => void;
};

export type Client = {
	opts: ClientOptions;

	/** Typed fetch */
	http: oapi.Client<paths>;

	/** Start receiving events */
	start: () => void;

	/** Stop receiving events */
	stop: () => void;
};

type Resume = {
	conn: string;
	seq: number;
};

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let state: ClientState = "stopped";
	let resume: null | Resume = null;

	function setState(newState: ClientState) {
		state = newState;
		opts.onState(newState);
	}

	function setupWebsocket() {
		if (state !== "reconnecting") return;

		ws = new WebSocket(new URL("/api/v1/sync", opts.baseUrl));
		ws.addEventListener("message", (e) => {
			const msg: MessageEnvelope = JSON.parse(e.data);
			if (msg.op === "Ping") {
				ws.send(JSON.stringify({ type: "Pong" }));
			} else if (msg.op === "Sync") {
				if (resume) resume.seq = msg.seq;
				opts.onSync(msg.data);
			} else if (msg.op === "Error") {
				console.error(msg.error);
				setState("reconnecting");
				ws.close();
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
			setState("reconnecting");
			console.error(e);
			ws.close();
		});

		ws.addEventListener("close", () => {
			setTimeout(setupWebsocket, 1000);
		});
	}

	const http = createFetch<paths>({
		baseUrl: opts.baseUrl,
	});

	http.use({
		onRequest(r) {
			r.request.headers.set("authorization", `${opts.token}`);
			return r.request;
		},
	});

	return {
		opts,
		http,
		start: () => {
			setState("reconnecting");
			if (ws) {
				ws.close();
			} else {
				setupWebsocket();
			}
		},
		stop: () => {
			setState("stopped");
			ws?.close();
		},
	};
}
