import { ClientRequestMethod } from "openapi-fetch";
import { paths } from "ts-sdk/schema";
import { Emitter } from "../../core/events";
import type { MessageClient, Session } from "../../types";
import { Backend, BackendEvents, type UntypedRequest } from "../backend";
import {
	HEATBEAT_INTERVAL_CLIENT,
	type WorkerCommand,
	type WorkerEvent,
} from "./shared";

export type SharedBackendOptions = {
	sessionId: string;
	token: string;
	apiUrl: string;
};

export class SharedBackend extends Backend {
	private worker: SharedWorker;

	private pendingRequests = new Map<
		string,
		{ resolve: (value: Response) => void; reject: (reason: any) => void }
	>();

	constructor(o: SharedBackendOptions) {
		super();

		// FIXME: handle compilation/bundling
		this.worker = new SharedWorker("./server.ts", {
			type: "module",
			name: `lamprey-client-${o.sessionId}`,
		});

		this.worker.addEventListener("error", (e) => {
			console.error("worker error", e.error);
		});

		this.worker.port.addEventListener(
			"message",
			(e: MessageEvent<WorkerEvent>) => {
				const { data } = e;
				if (data.type === "fetched") {
					const promise = this.pendingRequests.get(data.nonce);
					if (promise) {
						if (data.body.ok) {
							promise.resolve(data.body);
						} else {
							promise.reject(data.body);
						}
						this.pendingRequests.delete(data.nonce);
					}
				}
			},
		);

		this.worker.port.start();

		// TODO: only send this once the worker is ready
		this.post({
			type: "connect",
			api_url: o.apiUrl,
			token: o.token,
		});

		const timer = setInterval(() => {
			this.post({ type: "heartbeat" });
		}, HEATBEAT_INTERVAL_CLIENT);

		// TODO: stop timer on disconnect
		// clearInterval(timer);
	}

	private post(msg: WorkerCommand) {
		this.worker.port.postMessage(msg);
	}

	fetch(req: UntypedRequest): Promise<Response> {
		return new Promise((resolve, reject) => {
			const nonce = Math.random().toString(36);
			this.pendingRequests.set(nonce, { resolve, reject });
			this.post({
				type: "fetch",
				nonce,
				request: req,
			});
		});
	}

	send(msg: MessageClient): void {
		this.post({ type: "send", message: msg });
	}
}
