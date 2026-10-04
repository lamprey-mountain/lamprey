import { Http } from "./client";
import { Emitter } from "./core/events";
import type { MessageClient, MessageEnvelope } from "./types";

export type Client2Options = {
	apiUrl: string;
	// token?: string;
	// format?: "json" | "msgpack";
	// compress?: "deflate";
};

export type Client2Events = {
	// onReady: (event: MessageReady) => void;
	// onSync: (event: MessageSync, raw: MessageEnvelope) => void;
	// onError?: (error: Error) => void;
	// onSend?: (data: unknown) => void;
	// onMessage?: (raw: MessageEnvelope) => void;
	// onStreamOpen?: (stream: StreamInfo) => void;
	// onStreamClose?: (stream: StreamInfo) => void;
};

export class Client2 extends Emitter<Client2Events> {
	// public readonly http: Http;
	public readonly apiUrl: string;

	constructor(opts: Client2Options) {
		super();
		this.apiUrl = opts.apiUrl;
	}

	public start(token?: string) {}
	public stop() {}
	public kill() {}
	public subscribe(subscribe: Client2Subscribe) {}
}

export type Client2Subscribe = {
	// room | channel | document
};

export class Stream extends Emitter<StreamEvents> {
	constructor() {
		super();
		// TODO
	}

	public close() {}
	public send(data: MessageClient) {}
}

export type StreamEvents = {
	open: void;
	close: void;
	error: Error;
	recv: MessageEnvelope;
	send: MessageClient;
};
