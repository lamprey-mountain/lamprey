import { Channel, ChannelPatch } from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";

export class ChannelsService extends BaseService<Channel> {
	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(id: string): Promise<Channel> {
		return await fetchWithRetry(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: id } },
			})
		);
	}

	async create(
		room_id: string,
		body: { name: string; parent_id?: string },
	): Promise<Channel> {
		const data = await fetchWithRetry(() =>
			this.client.http.POST("/api/v1/room/{room_id}/channel", {
				params: { path: { room_id } },
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	async update(channel_id: string, body: ChannelPatch): Promise<Channel> {
		const data = await fetchWithRetry(() =>
			this.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id } },
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	// Helper normalization method from original
	normalize(channel: Channel): Channel {
		if (!channel.permission_overwrites) channel.permission_overwrites = [];
		if (!channel.recipients) channel.recipients = [];
		if (
			!channel.tags &&
			(channel.type === "ThreadPublic" || channel.type === "ThreadPrivate" ||
				channel.type === "ThreadForum2")
		) {
			channel.tags = [];
		}
		return channel;
	}

	override upsert(item: Channel) {
		super.upsert(this.normalize(item));
	}
}
