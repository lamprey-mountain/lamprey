import { EmojiCustom, Pagination } from "sdk";
import { BaseService } from "../core/Service";
import { createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

const log = logger.for("api/emoji");

export class EmojiService extends BaseService<EmojiCustom> {
	protected cacheName = "emoji";

	private _roomLists = new Map<string, PaginatedList>();

	getKey(item: EmojiCustom): string {
		return item.id;
	}

	async fetch(id: string): Promise<EmojiCustom> {
		throw new Error("Use fetchByRoom(room_id, emoji_id) instead");
	}

	async fetchByRoom(room_id: string, emoji_id: string): Promise<EmojiCustom> {
		const data = await this.retryWithBackoff<EmojiCustom>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/emoji/{emoji_id}", {
				params: { path: { room_id, emoji_id } },
			})
		);
		this.upsert(data);
		return data;
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<EmojiCustom>>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/emoji", {
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

			const newIds = data.items.map((emoji) => emoji.id);
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.id);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useRoomList(
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

	async search(query: string): Promise<Pagination<EmojiCustom>> {
		return await this.retryWithBackoff<Pagination<EmojiCustom>>(() =>
			this.client.http.GET("/api/v1/emoji/search", {
				params: {
					query: { query, limit: 100 },
				},
			})
		);
	}

	async listAllCustom(roomIds: string[]): Promise<EmojiCustom[]> {
		const results = await Promise.all(
			roomIds.map(async (room_id) => {
				try {
					const data = await this.retryWithBackoff<Pagination<EmojiCustom>>(
						() =>
							this.client.http.GET("/api/v1/room/{room_id}/emoji", {
								params: {
									path: { room_id },
								},
							}),
					);
					this.upsertBulk(data.items);
					return data.items;
				} catch (e) {
					log.error(String(e));
					return [];
				}
			}),
		);
		return results.flat();
	}
}
