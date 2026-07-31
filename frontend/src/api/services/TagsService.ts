import type { PaginationResponse, Tag } from "sdk";
import {
	type Accessor,
	createEffect,
	createResource,
	type Resource,
} from "solid-js";
import { BaseService } from "../core/Service";

export class TagsService extends BaseService<Tag> {
	protected cacheName = "tag";

	getKey(item: Tag): string {
		return item.id;
	}

	async fetch(_id: string): Promise<Tag> {
		throw new Error("Use fetchByChannel(channel_id, tag_id) instead");
	}

	async fetchByChannel(channel_id: string, tag_id: string): Promise<Tag> {
		const tag = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
				},
			}),
		);

		this.upsert(tag);
		return tag;
	}

	useTag(
		channel_id: Accessor<string>,
		tag_id: Accessor<string>,
	): Resource<Tag | undefined> {
		const [resource, { mutate }] = createResource(
			() => {
				const cid = channel_id();
				const tid = tag_id();
				return cid && tid ? { cid, tid } : undefined;
			},
			async ({ cid, tid }) => {
				const cached = this.cache.get(tid);
				if (cached) return cached;
				return this.fetchByChannel(cid, tid);
			},
		);

		createEffect(() => {
			const tid = tag_id();
			if (!tid) return;
			if (this.cache.has(tid)) {
				mutate(this.cache.get(tid));
			}
		});

		return resource;
	}

	/**
	 * List tags in a channel
	 */
	async list(
		channel_id: string,
		archived?: boolean,
	): Promise<PaginationResponse<Tag>> {
		const params: Record<string, boolean> = {};
		if (archived !== undefined) {
			params.archived = archived;
		}

		const data = await this.retryWithBackoff<PaginationResponse<Tag>>(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/tag", {
				params: {
					path: { channel_id },
					query: params,
				},
			}),
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
		const data = await this.retryWithBackoff<PaginationResponse<Tag>>(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/tag/search", {
				params: {
					path: { channel_id },
					query: { query, ...(archived !== undefined ? { archived } : {}) },
				},
			}),
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
			}),
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
			}),
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
		const params: Record<string, boolean> = {};
		if (force) {
			params.force = force;
		}
		await this.retryWithBackoff(() =>
			this.client.http.DELETE("/api/v1/channel/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
					query: params,
				},
			}),
		);
		this.cache.delete(tag_id);
	}
}
