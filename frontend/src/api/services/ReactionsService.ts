import type { Pagination } from "sdk";
import { BaseService } from "../core/Service";

export type ReactionUser = { user_id: string };

export class ReactionsService extends BaseService<never> {
	protected cacheName = "reactions";

	getKey(item: never): string {
		throw new Error("ReactionsService does not cache items");
	}

	async fetch(id: string): Promise<never> {
		throw new Error("ReactionsService does not fetch single items");
	}

	async list(
		channel_id: string,
		message_id: string,
		key: string,
		query: { limit?: number; after?: string },
	): Promise<Pagination<ReactionUser>> {
		return await this.retryWithBackoff<Pagination<ReactionUser>>(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
				{
					params: {
						path: {
							reaction_key: key,
							message_id,
							channel_id,
						},
						query,
					},
				},
			),
		);
	}

	async add(
		channel_id: string,
		message_id: string,
		key: string,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
				{
					params: {
						path: {
							user_id: "@self",
							reaction_key: key,
							message_id,
							channel_id,
						},
					},
				},
			),
		);
	}

	async remove(
		channel_id: string,
		message_id: string,
		key: string,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
				{
					params: {
						path: {
							reaction_key: key,
							message_id,
							channel_id,
							user_id: "@self",
						},
					},
				},
			),
		);
	}

	async removeForUser(
		channel_id: string,
		message_id: string,
		user_id: string,
		key: string,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
				{
					params: {
						path: {
							reaction_key: key,
							message_id,
							channel_id,
							user_id,
						},
					},
				},
			),
		);
	}

	async removeForKey(
		channel_id: string,
		message_id: string,
		key: string,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
				{
					params: {
						path: {
							reaction_key: key,
							message_id,
							channel_id,
						},
					},
				},
			),
		);
	}

	async removeAll(channel_id: string, message_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE(
				"/api/v1/channel/{channel_id}/message/{message_id}/reaction",
				{
					params: {
						path: {
							message_id,
							channel_id,
						},
					},
				},
			),
		);
	}
}
