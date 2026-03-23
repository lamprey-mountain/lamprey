import { Room } from "sdk";
import { BaseService } from "../core/Service";
import { batch, createResource, createSignal, type Resource } from "solid-js";
import type { Pagination } from "sdk";
import { ListState, PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

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
			})
		);
		this.upsert(data);
		return data;
	}

	async create(body: { name: string; public?: boolean | null }): Promise<Room> {
		const data = await this.retryWithBackoff<Room>(() =>
			this.client.http.POST("/api/v1/room", {
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	async update(room_id: string, body: any): Promise<Room> {
		const data = await this.retryWithBackoff<Room>(() =>
			this.client.http.PATCH(
				"/api/v1/room/{room_id}",
				{
					params: { path: { room_id } },
					body,
				},
			)
		);
		this.upsert(data);
		return data;
	}

	async fetchList(cursor?: string): Promise<Pagination<Room>> {
		return this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/user/{user_id}/room", {
				params: {
					path: { user_id: "@self" },
					query: {
						dir: "f",
						limit: 100,
						from: cursor,
					},
				},
			})
		);
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
			})
		);
	}

	private async fetchPage(
		list: PaginatedList,
		fetch: (cursor?: string) => Promise<Pagination<Room>>,
	): Promise<ListState> {
		if (list.state.isLoading || !list.state.has_more) return list.state;
		list.setLoading(true);

		try {
			const data = await fetch();
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

	useList() {
		if (
			this.roomList.state.ids.length === 0 && !this.roomList.state.isLoading
		) {
			this.fetchPage(this.roomList, this.fetchList);
		}
		return this.roomList.state;
	}

	useListAll() {
		if (
			this.roomListAll.state.ids.length === 0 &&
			!this.roomListAll.state.isLoading
		) {
			this.fetchPage(this.roomListAll, this.fetchListAll);
		}
		return this.roomListAll.state;
	}

	async markRead(room_id: string) {
		let has_more = true;
		let from: string | undefined = undefined;
		while (has_more) {
			let data;
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
					})
				);
			} catch (error) {
				log.error("Failed to fetch threads for room", error);
				break;
			}

			for (const thread of data.items) {
				if (thread.last_version_id) {
					await this.client.http.PUT(
						"/api/v1/channel/{channel_id}/ack",
						{
							params: { path: { channel_id: thread.id } },
							body: { version_id: thread.last_version_id },
						},
					);
				}
			}
			has_more = data.has_more;
			from = data.items.at(-1)?.id;
		}
	}
}
