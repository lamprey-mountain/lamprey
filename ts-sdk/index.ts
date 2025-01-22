import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import { UUID } from "uuidv7";
import type { paths } from "./schema.d.ts";
import { MessageServer } from "./types.ts";

export * as types from "./types.ts";

export type ClientOptions = {
	baseUrl: string;
	token: string;
	onMessage: (event: MessageServer) => void;
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
	let wsStopped = false;

	function setupWebsocket() {
		if (wsStopped) return;

		ws = new WebSocket(new URL("/api/v1/sync", opts.baseUrl));
		ws.addEventListener("message", (e) => {
			const msg = JSON.parse(e.data);
			if (msg.type === "Ping") {
				ws.send(JSON.stringify({ type: "Pong" }));
			} else {
				opts.onMessage(msg);
			}
		});

		ws.addEventListener("open", (_e) => {
			console.log("opened");
			ws.send(JSON.stringify({ type: "Hello", token: opts.token }));
		});

		ws.addEventListener("close", () => {
			setTimeout(setupWebsocket, 1000);
		});
		
		ws.addEventListener("error", (e) => {
			console.error(e);
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
			wsStopped = false;
			setupWebsocket();
		},
		stop: () => {
			wsStopped = true;
			ws.close();
		},
	};
}

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes.slice(0, 6).reduce(
		(acc: number, e: number) => acc * 256 + e,
		0,
	);
	return new Date(timestamp);
}
