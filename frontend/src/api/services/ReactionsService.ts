import type { MessageSync, Pagination, ReactionKey } from "sdk";
import { BaseService } from "../core/Service";

export type ReactionUser = { user_id: string };

export const areReactionKeysEqual = (
	a: ReactionKey,
	b: ReactionKey,
): boolean => {
	if (a.type !== b.type) return false;
	if (a.type === "Text" && b.type === "Text") return a.content === b.content;
	if (a.type === "Custom" && b.type === "Custom") return a.id === b.id;
	return false;
};

export class ReactionsService extends BaseService<never> {
	protected cacheName = "reactions";

	getKey(_item: never): string {
		throw new Error("ReactionsService does not cache items");
	}

	async fetch(_id: string): Promise<never> {
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

	handleSync(
		sync: MessageSync & {
			type:
				| "ReactionCreate"
				| "ReactionDelete"
				| "ReactionDeleteKey"
				| "ReactionDeleteAll";
		},
	) {
		const me = this.store.users.get("@self")!;
		const msg = this.store.messages.cache.get(sync.message_id);
		if (!msg) return;
		msg.reactions ??= [];

		if (sync.type === "ReactionCreate") {
			const c = msg.reactions.find((i) =>
				areReactionKeysEqual(i.key, sync.key),
			);
			if (c) {
				c.count += 1;
				c.self ||= sync.user_id === me.id;
			} else {
				msg.reactions.push({
					count: 1,
					key: sync.key,
					self: sync.user_id === me.id,
				});
			}
		} else if (sync.type === "ReactionDelete") {
			const idx = msg.reactions.findIndex((i) =>
				areReactionKeysEqual(i.key, sync.key),
			);
			if (idx === -1) return;
			const r = msg.reactions[idx];
			r.count = Math.max(0, r.count - 1);
			if (sync.user_id === me.id) r.self = false;
			if (r.count === 0) msg.reactions.splice(idx, 1);
		} else if (sync.type === "ReactionDeleteKey") {
			const idx = msg.reactions.findIndex((i) =>
				areReactionKeysEqual(i.key, sync.key),
			);
			if (idx !== -1) msg.reactions.splice(idx, 1);
		} else if (sync.type === "ReactionDeleteAll") {
			msg.reactions = [];
		}

		this.store.messages.handleMessageUpdate(msg);
	}
}
