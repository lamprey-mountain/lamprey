import { RoomMember, User } from "sdk";
import { BaseService } from "../core/Service";
import { Accessor, createEffect, createResource, Resource } from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

export class RoomMembersService extends BaseService<RoomMember> {
	protected cacheName = "room_member";

	private _roomLists = new Map<string, PaginatedList>();

	getKey(item: RoomMember): string {
		return `${item.room_id}:${item.user_id}`;
	}

	private compositeId(room_id: string, user_id: string): string {
		return `${room_id}:${user_id}`;
	}

	protected getDbKey(id: string): IDBValidKey {
		const [room_id, user_id] = id.split(":");
		return [room_id, user_id];
	}

	async fetch(id: string): Promise<RoomMember> {
		// id is "room_id:user_id"
		const [room_id, user_id] = id.split(":");
		if (!room_id || !user_id) throw new Error("Invalid composite ID");

		try {
			const data = await this.retryWithBackoff<RoomMember>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/member/{user_id}", {
					params: { path: { room_id, user_id } },
				})
			);
			return data;
		} catch (error: unknown) {
			const err = error as { error?: string };
			if (err?.error === "not found") {
				// Placeholder
				return {
					membership: "Leave" as const,
					room_id,
					user_id,
					mute: false,
					deaf: false,
					roles: [] as string[],
					joined_at: new Date().toISOString(),
					quarantined: false,
				};
			}
			throw error;
		}
	}

	override upsert(item: RoomMember) {
		this.cache.set(this.getKey(item), item);

		if (this.db && this.cacheName) {
			this.db.put(this.cacheName, item).catch((e) => {
				console.warn(`Failed to write to ${this.cacheName}`, {
					key: [item.room_id, item.user_id],
					error: e,
				});
			});
		}
	}

	// TODO: rename to useRoomMember
	useMember(
		room_id: Accessor<string>,
		user_id: Accessor<string>,
	): Resource<RoomMember | undefined> {
		const id = () => {
			const r = room_id();
			const u = user_id();
			return r && u ? this.compositeId(r, u) : undefined;
		};
		return this.use(id);
	}

	subscribeList(room_id: string, ranges: [number, number][]) {
		this.client.send({
			type: "MemberListSubscribe",
			room_id,
			ranges,
		});
	}

	async search(
		room_id: string,
		query: string,
	): Promise<{ room_members: RoomMember[]; users: User[] }> {
		const result = await this.retryWithBackoff<
			{ room_members: RoomMember[]; users: User[] }
		>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/member/search", {
				params: {
					path: { room_id },
					query: { query },
				},
			})
		);
		return result;
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<
				{ items: RoomMember[]; has_more: boolean }
			>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/member", {
					params: {
						path: { room_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((member) => this.getKey(member));
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.user_id);
		} catch (e) {
			logger.for("api/room_members").error(String(e));
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
