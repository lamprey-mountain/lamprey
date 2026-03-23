import { Channel, ChannelPatch } from "sdk";
import { BaseService } from "../core/Service";

export class ChannelsService extends BaseService<Channel> {
	protected cacheName = "channel";

	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(id: string): Promise<Channel> {
		return await this.retryWithBackoff<Channel>(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: id } },
			})
		);
	}

	async create(
		room_id: string,
		body: { name: string; parent_id?: string },
	): Promise<Channel> {
		const data = await this.retryWithBackoff<Channel>(() =>
			this.client.http.POST("/api/v1/room/{room_id}/channel", {
				params: { path: { room_id } },
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	async update(channel_id: string, body: ChannelPatch): Promise<Channel> {
		const data = await this.retryWithBackoff<Channel>(() =>
			this.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id } },
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	async typing(channel_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/typing", {
				params: { path: { channel_id } },
			})
		);
	}

	async ack(
		channel_id: string,
		message_id: string | undefined,
		version_id: string,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/channel/{channel_id}/ack", {
				params: { path: { channel_id } },
				body: { message_id, version_id },
			})
		);

		// Update cache
		const t = this.cache.get(channel_id);
		if (t) {
			const is_unread = version_id < (t.last_version_id ?? "");
			this.cache.set(channel_id, {
				...t,
				last_read_id: version_id,
				mention_count: 0,
				is_unread,
			} as Channel);
		}
	}

	async ackBulk(
		acks: Array<{
			channel_id: string;
			message_id?: string;
			version_id: string;
			mention_count?: number;
		}>,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/ack", {
				body: { acks },
			})
		);

		// Update cache in batch
		for (const ack of acks) {
			const t = this.cache.get(ack.channel_id);
			if (t) {
				const is_unread = ack.version_id < (t.last_version_id ?? "");
				const mention_count = ack.mention_count ?? 0;
				this.cache.set(ack.channel_id, {
					...t,
					last_read_id: ack.version_id,
					mention_count,
					is_unread,
				} as Channel);
			}
		}
	}

	async archive(channel_id: string): Promise<Channel> {
		return await this.update(channel_id, { archived: true });
	}

	async unarchive(channel_id: string): Promise<Channel> {
		return await this.update(channel_id, { archived: false });
	}

	async lock(channel_id: string): Promise<Channel> {
		return await this.update(channel_id, { locked: { allow_roles: [] } });
	}

	async unlock(channel_id: string): Promise<Channel> {
		return await this.update(channel_id, { locked: null });
	}

	async createThreadFromMessage(
		channel_id: string,
		message_id: string,
		body: { name: string; type?: import("sdk").ChannelType },
	): Promise<Channel> {
		const data = await this.retryWithBackoff<Channel>(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/message/{message_id}/thread",
				{
					params: { path: { channel_id, message_id } },
					body: body as any,
				},
			)
		);
		this.upsert(data);
		return data;
	}

	async createTag(
		channel_id: string,
		body: import("sdk").TagCreate,
	): Promise<import("sdk").Tag> {
		return await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/tag",
				{
					params: { path: { channel_id } },
					body: body,
				},
			)
		);
	}

	async updateTag(
		channel_id: string,
		tag_id: string,
		body: import("sdk").TagPatch,
	): Promise<import("sdk").Tag> {
		return await this.retryWithBackoff(() =>
			this.client.http.PATCH(
				"/api/v1/channel/{channel_id}/tag/{tag_id}",
				{
					params: { path: { channel_id, tag_id } },
					body: body,
				},
			)
		);
	}

	async deleteTag(
		channel_id: string,
		tag_id: string,
		force: boolean = false,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE(
				"/api/v1/channel/{channel_id}/tag/{tag_id}",
				{
					params: { path: { channel_id, tag_id } },
					query: { force },
				},
			)
		);
	}

	// Helper normalization method
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
