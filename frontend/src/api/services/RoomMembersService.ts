import { ReactiveMap } from "@solid-primitives/map";
import { ReactiveSet } from "@solid-primitives/set";
import type { RoomMember, User } from "sdk";
import { type Accessor, batch, createResource, type Resource } from "solid-js";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

export class RoomMembersService extends BaseService<RoomMember> {
	protected cacheName = "room_member";

	private _roomLists = new Map<string, PaginatedList>();
	private membersByRoom = new ReactiveMap<string, ReactiveSet<string>>();

	getKey(item: RoomMember): string {
		return `${item.room_id}:${item.user_id}`;
	}

	protected override afterUpsert(member: RoomMember) {
		const r = this.membersByRoom.get(member.room_id) ?? new ReactiveSet();
		r.add(member.user_id);
		this.membersByRoom.set(member.room_id, r);
	}

	protected override afterUpsertBulk(members: RoomMember[]) {
		const byRoom = new Map<string, Set<string>>();
		for (const m of members) {
			let s = byRoom.get(m.room_id);
			if (!s) {
				s = new Set();
				byRoom.set(m.room_id, s);
			}
			s.add(m.user_id);
		}

		batch(() => {
			for (const [room_id, user_ids] of byRoom) {
				const r = this.membersByRoom.get(room_id) ?? new ReactiveSet();
				for (const uid of user_ids) {
					r.add(uid);
				}
				this.membersByRoom.set(room_id, r);
			}
		});
	}

	protected override afterDelete(id: string, member?: RoomMember) {
		if (member) {
			this.membersByRoom.get(member.room_id)?.delete(member.user_id);
		} else {
			// fallback if member is not passed
			const [room_id, user_id] = id.split(":");
			if (room_id && user_id) {
				this.membersByRoom.get(room_id)?.delete(user_id);
			}
		}
	}

	listByRoom(room_id: string): RoomMember[] {
		const userIds = this.membersByRoom.get(room_id);
		if (!userIds) return [];
		return [...userIds]
			.map((uid) => this.cache.get(`${room_id}:${uid}`))
			.filter((m): m is RoomMember => m != null);
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

		const data = await this.retryWithBackoff<RoomMember>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/member/{user_id}", {
				params: { path: { room_id, user_id } },
			}),
		);
		return data;
	}

	override delete(id: string) {
		this.cache.delete(id);

		if (this.db && this.cacheName) {
			this.db.delete(this.cacheName, this.getDbKey(id)).catch((e) => {
				console.warn(`Failed to delete from ${this.cacheName}`, {
					key: id,
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
		const result = await this.retryWithBackoff<{
			room_members: RoomMember[];
			users: User[];
		}>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/member/search", {
				params: {
					path: { room_id },
					query: { query },
				},
			}),
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
			const data = await this.retryWithBackoff<{
				items: RoomMember[];
				has_more: boolean;
			}>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/member", {
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
