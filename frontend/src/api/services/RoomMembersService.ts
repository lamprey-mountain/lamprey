import { RoomMember } from "sdk";
import { BaseService } from "../core/Service";
import { Accessor, createEffect, createResource, Resource } from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";

export class RoomMembersService extends BaseService<RoomMember> {
	protected cacheName = "room_member";

	// For now, let's use the key convention "room_id:user_id" for the main cache
	// so `use()` works with a composite ID.

	getKey(item: RoomMember | string, user_id?: string): string {
		if (typeof item === "string") {
			return `${item}:${user_id}`;
		}
		return `${item.room_id}:${item.user_id}`;
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
	}

	useMember(
		room_id: Accessor<string>,
		user_id: Accessor<string>,
	): Resource<RoomMember | undefined> {
		const id = () => {
			const r = room_id();
			const u = user_id();
			return r && u ? this.getKey(r, u) : undefined;
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
