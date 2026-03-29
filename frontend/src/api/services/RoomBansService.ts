import type { Pagination, RoomBan } from "sdk";
import { type Accessor, createResource, type Resource } from "solid-js";
import { logger } from "../../logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/room_bans");

export class RoomBansService extends BaseService<RoomBan> {
	protected cacheName = "room_ban";

	private _roomLists = new Map<string, PaginatedList>();

	getKey(item: RoomBan): string {
		return `${item.room_id}:${item.user_id}`;
	}

	private compositeId(room_id: string, user_id: string): string {
		return `${room_id}:${user_id}`;
	}

	protected getDbKey(id: string): IDBValidKey {
		const [room_id, user_id] = id.split(":");
		return [room_id, user_id];
	}

	async fetch(id: string): Promise<RoomBan> {
		const [room_id, user_id] = id.split(":");
		if (!room_id || !user_id) throw new Error("Invalid composite ID");

		try {
			const data = await this.retryWithBackoff<RoomBan>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/ban/{user_id}", {
					params: { path: { room_id, user_id } },
				}),
			);
			return data;
		} catch (error: any) {
			if (error?.error === "not found") {
				// Return placeholder for non-existent ban
				return {
					room_id,
					user_id,
					banned_at: new Date().toISOString(),
					expires_at: null,
					reason: null,
				} as unknown as RoomBan;
			}
			throw error;
		}
	}

	async fetchByRoom(room_id: string, user_id: string): Promise<RoomBan> {
		return await this.fetch(this.compositeId(room_id, user_id));
	}

	useBan(
		room_id: Accessor<string>,
		user_id: Accessor<string>,
	): Resource<RoomBan | undefined> {
		const id = () => {
			const r = room_id();
			const u = user_id();
			return r && u ? this.compositeId(r, u) : undefined;
		};
		return this.use(id);
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<RoomBan>>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/ban", {
					params: {
						path: { room_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				}),
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((ban) => this.getKey(ban));
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.user_id);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useList(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(room_id, async (rid) => {
			if (!rid) return undefined;

			let list = this._roomLists.get(rid);
			if (!list) {
				list = new PaginatedList();
				this._roomLists.set(rid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchRoomPage(rid, list);
			}

			return list;
		});

		return resource;
	}
}
