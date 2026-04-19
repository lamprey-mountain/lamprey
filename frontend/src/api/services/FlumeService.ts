import type {
	FlumeCreate,
	FlumeDelta,
	FlumeState,
	LampreyComponent,
	Message,
} from "sdk";
import { batch } from "solid-js";
import { applyDelta, createComponentFromCreate } from "@/utils/components";
import { BaseService } from "../core/Service";

export type Flume = {
	channel_id: string;
	message_id: string;
	state: FlumeState;
	components: LampreyComponent[];
};

export class FlumeService extends BaseService<Flume> {
	protected cacheName = "flume";

	getKey(item: Flume): string {
		return item.message_id;
	}

	async fetch(_id: string): Promise<Flume> {
		throw new Error("FlumeService does not support fetching by ID");
	}

	/**
	 * Create a new flume by sending a POST to the flume endpoint.
	 * Called when the user wants to create a live-updating message.
	 */
	async create(channel_id: string, flume: FlumeCreate): Promise<Message> {
		const { data, error } = await this.client.http.POST(
			"/api/v1/channel/{channel_id}/flume",
			{
				params: { path: { channel_id } },
				body: flume,
			},
		);

		if (error) {
			throw error;
		}

		const message = data;
		this.upsert({
			channel_id,
			message_id: message.id,
			state: "Live",
			components: flume.components.map(createComponentFromCreate),
		});

		return message;
	}

	/**
	 * Handle a new flume message from MessageCreate sync event.
	 * Called from MessagesService.handleMessageCreate when flume.state === "Live".
	 */
	handleCreate(channel_id: string, message: Message): void {
		const flume = message.flume;
		if (!flume || flume.state !== "Live") return;

		this.upsert({
			channel_id,
			message_id: message.id,
			state: flume.state,
			components: message.latest_version.components ?? [],
		});
	}

	/**
	 * Handle a committed/autocommitted flume from MessageUpdate sync event.
	 * Called from Store when message.flume?.state !== "Live".
	 */
	handleCommit(message: Message): void {
		const flume = message.flume;
		if (!flume) return;

		this.upsert({
			channel_id: message.channel_id,
			message_id: message.id,
			state: flume.state,
			components: message.components ?? [],
		});
	}

	/**
	 * Handle a FlumeDelta sync event from the server.
	 * Called from Store when a FlumeDelta message sync is received.
	 */
	handleApply(
		_channel_id: string,
		message_id: string,
		delta: FlumeDelta,
	): void {
		const flume = this.cache.get(message_id);
		if (!flume) return;

		const newComponents = applyDelta(flume.components, delta);

		batch(() => {
			this.cache.set(message_id, {
				...flume,
				components: newComponents,
			});
		});
	}

	/**
	 * Handle a message delete - remove the flume from cache.
	 * Called from Store on MessageDelete.
	 */
	handleDelete(message_id: string): void {
		this.delete(message_id);
	}

	/**
	 * Commit a flume via API. Creates a final message version.
	 */
	async commit(channel_id: string, message_id: string): Promise<Message> {
		const { data, error } = await this.client.http.PUT(
			"/api/v1/channel/{channel_id}/flume/{message_id}/commit",
			{ params: { path: { channel_id, message_id } } },
		);

		if (error) {
			throw error;
		}

		return data;
	}

	/**
	 * Ping a flume to keep it alive (resets autocommit timer).
	 */
	async ping(channel_id: string, message_id: string): Promise<void> {
		const { error } = await this.client.http.POST(
			"/api/v1/channel/{channel_id}/flume/{message_id}/ping",
			{ params: { path: { channel_id, message_id } } },
		);

		if (error) {
			throw error;
		}
	}

	/**
	 * Apply a delta to a live flume via API.
	 */
	async applyDelta(
		channel_id: string,
		message_id: string,
		delta: FlumeDelta,
	): Promise<void> {
		const { error } = await this.client.http.PATCH(
			"/api/v1/channel/{channel_id}/flume/{message_id}/delta",
			{
				params: { path: { channel_id, message_id } },
				body: delta,
			},
		);

		if (error) {
			throw error;
		}
	}
}
