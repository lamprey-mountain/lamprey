import { PaginationResponse, Tag } from "sdk";
import { BaseService } from "../core/Service";

export class TagsService extends BaseService<Tag> {
	protected cacheName = "tag";

	getKey(item: Tag): string {
		return item.id;
	}

	async fetch(id: string): Promise<Tag> {
		throw new Error("Use fetchByChannel(channel_id, tag_id) instead");
	}

	async fetchByChannel(channel_id: string, tag_id: string): Promise<Tag> {
		throw new Error("Fetch via list() and cache lookup");
	}

	/**
	 * List tags in a channel
	 */
	async list(
		channel_id: string,
		archived?: boolean,
	): Promise<PaginationResponse<Tag>> {
		const params: any = {};
		if (archived !== undefined) {
			params.archived = archived;
		}

		const data = await this.retryWithBackoff<PaginationResponse<Tag>>(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/tag", {
				params: {
					path: { channel_id },
					query: params,
				},
			})
		);

		this.upsertBulk(data.items);
		return data;
	}

	/**
	 * Search tags in a channel
	 */
	async search(
		channel_id: string,
		query: string,
		archived?: boolean,
	): Promise<PaginationResponse<Tag>> {
		const params: any = { query };
		if (archived !== undefined) {
			params.archived = archived;
		}

		const data = await this.retryWithBackoff<PaginationResponse<Tag>>(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/tag/search", {
				params: {
					path: { channel_id },
					query: params,
				},
			})
		);

		this.upsertBulk(data.items);
		return data;
	}

	/**
	 * Create a new tag
	 */
	async create(
		channel_id: string,
		data: {
			name: string;
			description?: string;
			color?: string;
			restricted?: boolean;
		},
	): Promise<Tag> {
		const result = await this.retryWithBackoff<Tag>(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/tag", {
				params: {
					path: { channel_id },
				},
				body: data,
			})
		);
		this.upsert(result);
		return result;
	}

	/**
	 * Update a tag
	 */
	async update(
		channel_id: string,
		tag_id: string,
		data: {
			name?: string;
			description?: string | null;
			color?: string | null;
			archived?: boolean;
			restricted?: boolean;
		},
	): Promise<Tag> {
		const result = await this.retryWithBackoff<Tag>(() =>
			this.client.http.PATCH("/api/v1/channel/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
				},
				body: data,
			})
		);
		this.upsert(result);
		return result;
	}

	/**
	 * Delete a tag
	 */
	async remove(
		channel_id: string,
		tag_id: string,
		force?: boolean,
	): Promise<void> {
		const params: any = {};
		if (force) {
			params.force = force;
		}
		await this.retryWithBackoff(() =>
			this.client.http.DELETE("/api/v1/channel/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
					query: params,
				},
			})
		);
		this.cache.delete(tag_id);
	}
}
