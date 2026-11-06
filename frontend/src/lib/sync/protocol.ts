import type {
	ClientState,
	MessageClient,
	MessageEnvelope,
	MessageReady,
	MessageSync,
} from "sdk";

export type StreamSubscribeOptions =
	| {
			kind: "document";
			channel_id: string;
			branch_id: string;
			state_vector?: string;
	  }
	| { kind: "channel"; channel_id: string }
	| { kind: "room"; room_id: string };

export type SyncConnectionOptions = {
	apiUrl: string;
	token?: string;
	format?: "json" | "msgpack";
	compress?: "deflate";
	preferWebtransport?: boolean;
};

export type WorkerRequest =
	| { type: "connect"; options: SyncConnectionOptions }
	| { type: "send"; message: MessageClient }
	| { type: "stream-open"; streamId: number; options: StreamSubscribeOptions }
	| { type: "stream-send"; streamId: number; message: MessageClient }
	| { type: "stream-close"; streamId: number }
	| { type: "stop" }
	| { type: "stop-aggressive" }
	| { type: "heartbeat" };

export type WorkerResponse =
	| { type: "state"; state: ClientState }
	| { type: "capabilities"; webtransport: boolean }
	| { type: "ready"; event: MessageReady }
	| { type: "sync"; event: MessageSync; raw: MessageEnvelope }
	| {
			type: "stream-sync";
			streamId: number;
			event: MessageSync;
			raw: MessageEnvelope;
	  }
	| { type: "stream-error"; streamId: number; error: string }
	| { type: "stream-close"; streamId: number }
	| { type: "error"; error: string };

export const HEARTBEAT_INTERVAL = 30_000;
export const HEARTBEAT_TIMEOUT = 120_000;
