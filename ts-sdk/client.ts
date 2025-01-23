import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import { MessageServer } from "./types.ts";

export type ClientState = "stopped" | "connected" | "ready" | "reconnecting";

export type ClientOptions = {
	baseUrl: string;
	token: string;
	onMessage: (event: MessageServer) => void;
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

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let state: ClientState = "stopped";

	function setState(newState: ClientState) {
		state = newState;
		opts.onState(newState);
	}

	function setupWebsocket() {
		if (state !== "reconnecting") return;

		ws = new WebSocket(new URL("/api/v1/sync", opts.baseUrl));
		ws.addEventListener("message", (e) => {
			const msg = JSON.parse(e.data);
			if (msg.type === "Ping") {
				ws.send(JSON.stringify({ type: "Pong" }));
			} else {
				if (msg.type === "Ready") {
					setState("ready");
				}
				opts.onMessage(msg);
			}
		});

		ws.addEventListener("open", (_e) => {
			setState("connected");
			ws.send(JSON.stringify({ type: "Hello", token: opts.token }));
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
