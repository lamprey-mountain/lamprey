import { RoomMember } from "sdk";
import { BaseService } from "../core/Service";
import { Accessor, createEffect, createResource, Resource } from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";

export class RoomMembersService extends BaseService<RoomMember> {
	protected cacheName = "room_member";

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
		} catch (error: any) {
			if (error?.error === "not found") {
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
}
