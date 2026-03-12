import { Room } from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";

export class RoomsService extends BaseService<Room> {
	getKey(item: Room): string {
		return item.id;
	}

	async fetch(id: string): Promise<Room> {
		return await fetchWithRetry(() =>
			this.client.http.GET("/api/v1/room/{room_id}", {
				params: { path: { room_id: id } },
			})
		);
	}

	async create(body: { name: string; public?: boolean | null }): Promise<Room> {
		const { data, error } = await this.client.http.POST("/api/v1/room", {
			body,
		});
		if (error) throw error;
		this.upsert(data);
		return data;
	}

	async update(room_id: string, body: any): Promise<Room> {
		const { data, error } = await this.client.http.PATCH(
			"/api/v1/room/{room_id}",
			{
				params: { path: { room_id } },
				body,
			},
		);
		if (error) throw error;
		this.upsert(data);
		return data;
	}
}
