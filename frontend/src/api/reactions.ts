import type { Api } from "../api.tsx";
import type { Pagination } from "sdk";

export class Reactions {
	api: Api = null as unknown as Api;

	async list(
		channel_id: string,
		message_id: string,
		key: string,
		query: { limit?: number; after?: string },
	) {
		const { data } = await this.api.client.http.GET(
			"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id,
						channel_id,
					},
					query,
				},
			},
		);
		return data as Pagination<{ user_id: string }>;
	}

	async add(channel_id: string, message_id: string, key: string) {
		await this.api.client.http.PUT(
			"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id,
						channel_id,
					},
				},
			},
		);
	}

	async delete(channel_id: string, message_id: string, key: string) {
		await this.api.client.http.DELETE(
			"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id,
						channel_id,
					},
				},
			},
		);
	}
}
