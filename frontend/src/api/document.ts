import type { HistoryPagination } from "sdk";
import type { Api } from "../api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { fetchWithRetry } from "./util.ts";

export class Documents {
	api: Api = null as unknown as Api;

	async history(
		channel_id: string,
		branch_id: string,
		params?: {
			by_author?: boolean;
			by_tag?: boolean;
			by_time?: number;
			by_changes?: number;
			cursor?: string;
			limit?: number;
		},
	): Promise<HistoryPagination> {
		const data = await fetchWithRetry(() =>
			this.api.client.http.GET(
				"/api/v1/document/{channel_id}/branch/{branch_id}/history",
				{
					params: {
						path: { channel_id, branch_id },
						query: params,
					},
				},
			)
		);

		for (const user of data.users) {
			this.api.users.cache.set(user.id, user);
		}

		for (const member of data.room_members) {
			let cache = this.api.room_members.cache.get(member.room_id);
			if (!cache) {
				cache = new ReactiveMap();
				this.api.room_members.cache.set(member.room_id, cache);
			}
			cache.set(member.user_id, member);
		}

		for (const member of data.thread_members) {
			let cache = this.api.thread_members.cache.get(member.thread_id);
			if (!cache) {
				cache = new ReactiveMap();
				this.api.thread_members.cache.set(member.thread_id, cache);
			}
			cache.set(member.user_id, member);
		}

		return data;
	}
}
