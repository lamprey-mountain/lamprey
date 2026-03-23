import { BaseService } from "../core/Service";
import type { HistoryPagination, User, UserWithRelationship } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import type { Api } from "@/api";

export type RevisionContent = {
	data?: {
		root: {
			blocks: Array<{
				Markdown?: {
					content: string;
				};
			}>;
		};
	};
	root?: {
		blocks: Array<{
			Markdown?: {
				content: string;
			};
		}>;
	};
};

export class DocumentsService extends BaseService<RevisionContent> {
	api: Api = null as unknown as Api;
	protected cacheName = "document";

	revisionCache = new Map<string, RevisionContent>();

	getKey(item: RevisionContent): string {
		return "";
	}

	async fetch(id: string): Promise<RevisionContent> {
		const [channelId, revisionId] = id.split("@");
		const result = await this.getRevisionContent(channelId, revisionId);
		if (!result) {
			throw new Error(`Revision ${id} not found`);
		}
		return result;
	}

	async getRevisionContent(
		channel_id: string,
		revision_id: string,
	): Promise<RevisionContent | null> {
		const cacheKey = `${channel_id}@${revision_id}`;

		if (this.revisionCache.has(cacheKey)) {
			return this.revisionCache.get(cacheKey)!;
		}

		try {
			const data = await this.retryWithBackoff<RevisionContent>(() =>
				this.api.client.http.GET(
					"/api/v1/document/{channel_id}/revision/{revision_id}/content",
					{
						params: {
							path: {
								channel_id,
								revision_id: revision_id as any,
							},
						},
					},
				)
			);
			this.revisionCache.set(cacheKey, data);
			return data;
		} catch {
			return null;
		}
	}

	clearChannelCache(channel_id: string): void {
		const prefix = `${channel_id}@`;
		for (const key of this.revisionCache.keys()) {
			if (key.startsWith(prefix)) {
				this.revisionCache.delete(key);
			}
		}
	}

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
		const data = await this.retryWithBackoff<HistoryPagination>(() =>
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
			const userWithRelationship: UserWithRelationship = {
				...user,
				relationship: {
					note: null,
					relation: null,
					petname: null,
					until: null,
				},
			};
			this.api.users.cache.set(user.id, userWithRelationship);
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

	async getContent(
		channel_id: string,
		branch_id: string,
	): Promise<ArrayBuffer> {
		return await this.retryWithBackoff<ArrayBuffer>(() =>
			this.api.client.http.GET(
				"/api/v1/document/{channel_id}/branch/{branch_id}/crdt",
				{
					params: {
						path: { channel_id, branch_id },
					},
					parseAs: "arrayBuffer",
				},
			)
		);
	}
}
