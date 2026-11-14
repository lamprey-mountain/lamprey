import type { EmojiCustom, Pagination, Room } from "sdk";
import { batch, createResource, type Resource } from "solid-js";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/emoji");

export class EmojiService extends BaseService<EmojiCustom> {
	protected cacheName = "emoji";

	private _roomLists = new Map<string, PaginatedList>();
	private _roomFetches = new Map<string, Promise<void>>();
	/** room id -> `emoji_count` of the room when its emoji were last fetched */
	private _fetchedRooms = new Map<string, number>();
	private _hydrated = false;
	private _hydrating?: Promise<void>;

	getKey(item: EmojiCustom): string {
		return item.id;
	}

	async fetch(_id: string): Promise<EmojiCustom> {
		throw new Error("Use fetchByRoom(room_id, emoji_id) instead");
	}

	async fetchByRoom(room_id: string, emoji_id: string): Promise<EmojiCustom> {
		const data = await this.retryWithBackoff<EmojiCustom>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/emoji/{emoji_id}", {
				params: { path: { room_id, emoji_id } },
			}),
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
							limit: 1024,
							from: cursor,
						},
					},
				}),
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
			}),
		);
	}

	/** drop cached emoji of a room that the server no longer returns */
	private pruneRoom(room_id: string, keep: Set<string>) {
		for (const emoji of [...this.cache.values()]) {
			if (emoji.owner?.owner !== "Room") continue;
			if (emoji.owner.room_id !== room_id) continue;
			if (!keep.has(emoji.id)) this.delete(emoji.id);
		}
	}

	private fetchAllRoomEmoji(room: Room): Promise<void> {
		const existing = this._roomFetches.get(room.id);
		if (existing) return existing;

		const promise = (async () => {
			try {
				const data = await this.retryWithBackoff<Pagination<EmojiCustom>>(
					() =>
						this.client.http.GET("/api/v1/room/{room_id}/emoji", {
							params: {
								path: { room_id: room.id },
								query: { limit: 1024 },
							},
						}),
				);
				this.upsertBulk(data.items);
				this.pruneRoom(
					room.id,
					new Set(data.items.map((emoji) => emoji.id)),
				);
				this._fetchedRooms.set(room.id, room.emoji_count);
			} catch (e) {
				log.error(String(e));
			}
		})().finally(() => {
			this._roomFetches.delete(room.id);
		});

		this._roomFetches.set(room.id, promise);
		return promise;
	}

	/**
	 * Load custom emoji persisted in IndexedDB into the in-memory cache.
	 * Only reads once per service lifetime.
	 */
	async loadCached(): Promise<void> {
		if (this._hydrated) return;
		if (this._hydrating) return this._hydrating;

		const db = this.db;
		if (!db) return;

		this._hydrating = db
			.getAll("emoji")
			.then((items) => {
				batch(() => {
					for (const item of items) {
						this.cache.set(item.id, item);
					}
				});
				this._hydrated = true;
			})
			.catch((e) => {
				log.warn("failed to load cached emoji", e);
			})
			.finally(() => {
				this._hydrating = undefined;
			});

		return this._hydrating;
	}

	async listAllCustom(rooms: Room[]): Promise<EmojiCustom[]> {
		await this.loadCached();

		const cachedCounts = new Map<string, number>();
		for (const emoji of this.cache.values()) {
			if (emoji.owner?.owner !== "Room") continue;
			const room_id = emoji.owner.room_id;
			cachedCounts.set(room_id, (cachedCounts.get(room_id) ?? 0) + 1);
		}

		const roomsToFetch = rooms.filter((room) => {
			// already in sync at the current emoji count
			if (this._fetchedRooms.get(room.id) === room.emoji_count) return false;

			// rooms without custom emoji don't need a request
			if (room.emoji_count === 0) {
				this.pruneRoom(room.id, new Set());
				this._fetchedRooms.set(room.id, 0);
				return false;
			}

			// the persisted cache already has every emoji in the room
			const cached = cachedCounts.get(room.id) ?? 0;
			const neverFetched = this._fetchedRooms.get(room.id) === undefined;
			if (neverFetched && cached === room.emoji_count) {
				this._fetchedRooms.set(room.id, room.emoji_count);
				return false;
			}

			return true;
		});

		if (roomsToFetch.length === 0) {
			return [...this.cache.values()];
		}

		await Promise.all(roomsToFetch.map((room) => this.fetchAllRoomEmoji(room)));
		return [...this.cache.values()];
	}

	clear() {
		super.clear();
		for (const list of this._roomLists.values()) {
			list.clear();
		}
		this._roomLists.clear();
		this._roomFetches.clear();
		this._fetchedRooms.clear();
		this._hydrated = false;
		this._hydrating = undefined;
	}
}
