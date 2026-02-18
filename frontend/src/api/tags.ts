import type { PaginationResponse, Tag } from "sdk";
import type { Api } from "../api";

export class Tags {
	api: Api = null as unknown as Api;

	/**
	 * List tags in a channel
	 */
	async list(
		channel_id: string,
		archived?: boolean,
	): Promise<PaginationResponse<Tag>> {
		const params = new URLSearchParams();
		if (archived !== undefined) {
			params.set("archived", archived.toString());
		}

		const response = await this.api.client.http.GET(
			"/api/v1/channel/{channel_id}/tag",
			{
				params: {
					path: { channel_id },
					query: Object.fromEntries(params) as any,
				},
			},
		);

		return response.data as PaginationResponse<Tag>;
	}

	/**
	 * Search tags in a channel
	 */
	async search(
		channel_id: string,
		query: string,
		archived?: boolean,
	): Promise<PaginationResponse<Tag>> {
		const params = new URLSearchParams();
		params.set("query", query);
		if (archived !== undefined) {
			params.set("archived", archived.toString());
		}

		const response = await this.api.client.http.GET(
			"/api/v1/channel/{channel_id}/tag/search",
			{
				params: {
					path: { channel_id },
					query: Object.fromEntries(params) as any,
				},
			},
		);

		return response.data as PaginationResponse<Tag>;
	}

	/**
	 * Create a new tag
	 */
	async create(
		channel_id: string,
		data: {
			name: string;
			description?: string;
			color?: { r: number; g: number; b: number; a: number };
			restricted?: boolean;
		},
	): Promise<Tag> {
		const response = await this.api.client.http.POST(
			"/api/v1/channel/{channel_id}/tag",
			{
				params: {
					path: { channel_id },
				},
				body: data,
			},
		);

		return response.data as Tag;
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
			color?: { r: number; g: number; b: number; a: number } | null;
			archived?: boolean;
			restricted?: boolean;
		},
	): Promise<Tag> {
		const response = await this.api.client.http.PATCH(
			"/api/v1/channel/{channel_id}/tag/{tag_id}",
			{
				params: {
					path: { channel_id, tag_id },
				},
				body: data,
			},
		);

		return response.data as Tag;
	}

	/**
	 * Delete a tag
	 */
	async delete(
		channel_id: string,
		tag_id: string,
		force?: boolean,
	): Promise<void> {
		await this.api.client.http.DELETE(
			"/api/v1/channel/{channel_id}/tag/{tag_id}",
			{
				params: {
					path: { channel_id, tag_id },
					query: force ? { force: true } : {},
				},
			},
		);
	}
}
