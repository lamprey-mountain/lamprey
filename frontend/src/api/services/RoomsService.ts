import type { Channel, Pagination, Room } from "sdk";
import { logger } from "@/utils/logger";
import { type ListState, PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/rooms");

export class RoomsService extends BaseService<Room> {
	protected cacheName = "room";

	public roomList = new PaginatedList();
	private roomListAll = new PaginatedList();

	getKey(item: Room): string {
		return item.id;
	}

	async fetch(id: string): Promise<Room> {
		const data = await this.retryWithBackoff<Room>(() =>
			this.client.http.GET("/api/v1/room/{room_id}", {
				params: { path: { room_id: id } },
			}),
		);
		this.upsert(data);
		return data;
	}

	async create(body: { name: string; public?: boolean | null }): Promise<Room> {
		const data = await this.retryWithBackoff<Room>(() =>
			this.client.http.POST("/api/v1/room", {
				body,
			}),
		);
		this.upsert(data);
		return data;
	}

	async search(query: string): Promise<{ rooms: Room[]; results: string[] }> {
		const data = await this.retryWithBackoff<{
			rooms: Room[];
			results: string[];
		}>(() =>
			this.client.http.POST("/api/v1/room/search", {
				body: { query },
			}),
		);
		if (data.rooms) {
			for (const room of data.rooms) {
				this.upsert(room);
			}
		}
		return data;
	}

	async update(room_id: string, body: Record<string, unknown>): Promise<Room> {
		const data = await this.retryWithBackoff<Room>(() =>
			this.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body,
			}),
		);
		this.upsert(data);
		return data;
	}

	async fetchList(_cursor?: string): Promise<Pagination<Room>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/user/{user_id}/room", {
				params: {
					path: { user_id: "@self" },
				},
			}),
		);

		for (const room of data.rooms) {
			this.upsert(room);
		}

		return {
			items: data.rooms,
			total: data.rooms.length,
			has_more: false,
		};
	}

	public async fetchListAll(cursor?: string): Promise<Pagination<Room>> {
		return this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room", {
				params: {
					query: {
						dir: "f",
						limit: 100,
						from: cursor,
					},
				},
			}),
		);
	}

	private async fetchPage(
		list: PaginatedList,
		fetch: (cursor?: string) => Promise<Pagination<Room>>,
	): Promise<ListState> {
		if (list.state.isLoading || !list.state.has_more) return list.state;
		list.setLoading(true);

		try {
			const data = await fetch(list.state.cursor ?? undefined);
			this.upsertBulk(data.items);

			const newIds = data.items.map((room) => this.getKey(room));
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.id);

			return list.state;
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	/** load the next page of all rooms on this server */
	async loadListAll(): Promise<ListState> {
		const list = this.roomListAll;
		if (list.state.isLoading || !list.state.has_more) return list.state;

		list.setLoading(true);
		try {
			const data = await this.fetchListAll(list.state.cursor ?? undefined);
			this.upsertBulk(data.items);
			list.appendPage(
				data.items.map((room) => room.id),
				data.has_more,
				data.items.at(-1)?.id,
			);
			return list.state;
		} catch (e) {
			list.setError(e);
			throw e;
		}
	}

	useList() {
		if (
			this.roomList.state.ids.length === 0 &&
			!this.roomList.state.isLoading
		) {
			this.fetchPage(this.roomList, (cursor) => this.fetchList(cursor));
		}
		return this.roomList.state;
	}

	useListAll() {
		if (
			this.roomListAll.state.ids.length === 0 &&
			!this.roomListAll.state.isLoading
		) {
			this.fetchPage(this.roomListAll, (cursor) => this.fetchListAll(cursor));
		}
		return this.roomListAll.state;
	}

	/** mark all channels in a room as read, using a single bulk ack */
	async markRead(room_id: string) {
		const acks: Array<{ channel_id: string; message_id: string }> = [];
		let has_more = true;
		let from: string | undefined;

		while (has_more) {
			let data: Pagination<Channel> | undefined;
			try {
				data = await this.retryWithBackoff(() =>
					this.client.http.GET("/api/v1/room/{room_id}/channel", {
						params: {
							path: { room_id },
							query: {
								dir: "f",
								limit: 100,
								from,
							},
						},
					}),
				);
			} catch (error) {
				log.error("Failed to fetch threads for room", error);
				break;
			}

			if (!data?.items) break;

			for (const channel of data.items) {
				if (channel.last_message_id) {
					acks.push({
						channel_id: channel.id,
						message_id: channel.last_message_id,
					});
				}
			}
			has_more = data.has_more;
			from = data.items.at(-1)?.id;
		}

		if (acks.length === 0) return;
		await this.store.channels.ackBulk(acks);
	}

	protected afterDelete(id: string): void {
		this.roomList.removeId(id);
		this.roomListAll.removeId(id);
	}

	clear() {
		super.clear();
		this.roomList.clear();
		this.roomListAll.clear();
	}
}
