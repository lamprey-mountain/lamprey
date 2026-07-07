import { Emitter } from "../core/events";
import { RoomManager } from "../resources/room";
import type { MessageSync } from "../types";
import { type Backend, DirectBackend } from "./backend";
import { ClientBackend } from "../core/private";

export type ClientOptions = {
	apiUrl: string;
	token?: string;
	// maybe add cdn/media url
	// TODO: allow passing custom fetch impl
};

export type ClientEvents = {
	sync: MessageSync;
	// sync: api["MessageEnvelope"],
};

// NOTE: maybe rename to Client?
export class Lamprey extends Emitter<ClientEvents> {
	// public http: Http;
	// private backend: Backend;
	[ClientBackend]: Backend;

	public readonly rooms: RoomManager;
	// channels
	// users
	// media
	// voice (maybe?)

	constructor(o: ClientOptions) {
		super();

		if (!o.token) throw new Error("TODO: handle client without token");

		// TODO: allow configurable backends
		const backend = new DirectBackend({ apiUrl: o.apiUrl, token: o.token });

		backend.on("sync", (m) => {
			if (m.op === "Sync") {
				this.handleSync(m.data);
			}
		});

		this[ClientBackend] = backend;
		this.rooms = new RoomManager(this);
		// this.rooms.cache.set("a", "b");
	}

	private handleSync(sync: MessageSync) {
		// if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
		//   this.rooms.upsert(msg.room);
		// } else if (msg.type === "RoomDelete") {
		//   this.rooms.delete(msg.room_id);
		// }
	}
}

// export type Item<T> =
// 	| { status: "loading" } // the item is currently being loaded
// 	| { status: "ready"; data: T } // up to date and currently in sync
// 	| { status: "stale"; data: T } // loaded from cache (eg. http cache, indexeddb. 304 not modified is `ready` iff the client is connected to sync)
// 	| { status: "error"; error: unknown }; // an error occurred while fetching this resource
// maybe add "not found"/doesnt exist

// class Authenticator {}
// create session
// create guest or use auth method
// convert to client (class Lamprey)?

// export type ClientState = "unauthenticated" | "authenticated" | "connected" | "ready";
// export type ClientAuth = { token: string } | null;
