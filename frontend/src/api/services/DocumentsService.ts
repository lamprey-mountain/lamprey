import { BaseService } from "../core/Service";
import type { HistoryPagination, User, UserWithRelationship } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";

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
				this.client.http.GET(
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
			this.client.http.GET(
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
			this.store.users.cache.set(user.id, userWithRelationship);
		}

		for (const member of data.room_members) {
			this.store.roomMembers.upsert(member);
		}

		for (const member of data.thread_members) {
			this.store.threadMembers.upsert(member);
		}

		return data;
	}

	async getContent(
		channel_id: string,
		branch_id: string,
	): Promise<ArrayBuffer> {
		return await this.retryWithBackoff<ArrayBuffer>(() =>
			this.client.http.GET(
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
