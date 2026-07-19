import { ClientBackend } from "../core/private";
import { defineResource } from "../core/resource";
import type { api, Lamprey } from "../index";
import { Manager } from "./mod";

export type ApiRoom = api["Room"];

export type Room = ApiRoom & RoomExt;

type RoomExt = {
	// channels
	// threads
	// roles
	// members
	// auditLog
	// emojis
	// webhooks
	// automod
	// bans
	// invites

	delete(): Promise<void>;
	// async fetch() {}
	// async update() {}
	// async delete() {}
	// async leave() {}
	// toJSON(): ApiRoom {}
};

export const roomResource = defineResource<ApiRoom, RoomExt>((client) => ({
	async delete() {
		const response = await client[ClientBackend].fetch({
			method: "delete",
			path: "/api/v1/room/{room_id}",
			params: { room_id: this.id },
		});
		if (!response.ok) {
			console.error("failed request", response);
			throw "TODO: better error handling";
		}
	},
}));

export class RoomManager extends Manager<Room, string> {
	constructor(public client: Lamprey) {
		super();
	}

	// TODO: use collection type?
	public readonly cache = new Map();

	resolve(it: Room): Room;
	resolve(it: string): Room | undefined;
	resolve(it: Room | string): Room | undefined {
		return typeof it === "string" ? this.cache.get(it) : it;
	}

	resolveId(it: string | Room): string {
		return typeof it === "string" ? it : it.id;
	}

	async fetch(id: string): Promise<Room> {
		// TODO: check cache first
		const res = await this.client[ClientBackend].fetch({
			method: "get",
			path: "/api/v1/room/{room_id}",
			params: { room_id: id },
		});
		if (!res.ok) {
			console.error("failed request", res);
			throw "TODO: better error handling";
		}

		const r = roomResource.wrap(this.client, await res.json());
		// TODO: save r in cache
		return r;
	}

	// async create(body: api["RoomCreate"]): Promise<Room> {
	async create(body: unknown): Promise<Room> {
		throw "todo";
	}

	// TODO: other methods?
	// async update(room_id: string, body: Record<string, unknown>): Promise<Room> {}
	// async fetchList(cursor?: string): Promise<Pagination<Room>> {}
	// public async fetchListAll(cursor?: string): Promise<Pagination<Room>> {}
	// async search() // both for admin all rooms, users's own rooms, and all public rooms?
}
