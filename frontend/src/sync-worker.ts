/// <reference no-default-lib="true"/>
/// <reference lib="webworker" />

declare const self: SharedWorkerGlobalScope;

import {
	type Client,
	type ClientState,
	createClient,
	type MessageEnvelope,
	type MessageReady,
	type MessageSync,
} from "sdk";

// from tab to worker
type PortMessage =
	| {
			type: "connect";
			apiUrl: string;
	  }
	| {
			type: "websocket";
			data: unknown;
	  };

// from worker to tab
type PortResponse =
	| {
			type: "state";
			state: ClientState;
	  }
	| {
			type: "sync";
			event: MessageEnvelope;
	  }
	| {
			type: "websocket";
			data: unknown;
	  };

// information about a tab
type PortData = {
	port: MessagePort;
};

const ports = new Set<PortData>();
const client: Client | null = null;

// cached ready payload for when a client connects
const ready: MessageReady | null = null;

// maximum number of events to cache
const MAX_RESUME_EVENTS = 100;

function getOrCreateClient(): Client {
	throw "todo!";
}

// TODO: when receiving event, broadcast to all connected ports
// TODO: when receiving event, update cached ready

self.addEventListener("connect", (e) => {
	const port = e.ports[0] as MessagePort;
	const portData: PortData = {
		port,
	};

	ports.add(portData);

	port.addEventListener("message", (e: MessageEvent<PortMessage>) => {
		const msg = e.data;

		if (msg.type === "connect") {
			// TODO: get or create client based on url
		} else if (msg.type === "websocket") {
			// TODO: forward message to connected ports
			// TODO: if this is a resume, send last events
			// TODO: if this is a hello, send ready
		}
	});

	port.start();
});
