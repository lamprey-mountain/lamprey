import type { MessageClient, MessageEnvelope } from "ts-sdk/types";
import type { UntypedRequest } from "../backend";

export type WorkerCommand =
	| { type: "fetch"; request: UntypedRequest; nonce: string } // send an http request
	| { type: "send"; message: MessageClient } // send event via websocket
	| { type: "heartbeat" }
	/** initialize the worker, send auth data */
	| { type: "connect"; api_url: string; token: string };

export type WorkerEvent =
	| { type: "sync"; message: MessageEnvelope } // message from websocket sync
	| { type: "disconnect" } // disconnected from websocket sync
	// FIXME: Response is not transferrable
	| { type: "fetched"; body: Response; nonce: string } // response to a fetch command
	| { type: "error"; error: unknown };
// type: "ready"
// | {
// 		type: "state";
// 		state: ClientState;
//   }

export const HEATBEAT_INTERVAL_REQUIRED = 1000 * 10;
export const HEATBEAT_INTERVAL_CLIENT = 1000 * 5;
