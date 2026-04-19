import type { DocumentTag, DocumentTagCreate, DocumentTagPatch } from "sdk";
import { createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

export class DocumentTagService extends BaseService<DocumentTag> {
	protected cacheName = "document_tag";

	private tagLists = new Map<string, PaginatedList>();

	getKey(item: DocumentTag): string {
		return item.id;
	}

	async fetch(_id: string): Promise<DocumentTag> {
		throw new Error("Use list() or fetchById(channel_id, tag_id) instead");
	}

	private async fetchTagPage(
		channel_id: string,
		list: PaginatedList,
	): Promise<void> {
		if (list.state.isLoading) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<DocumentTag[]>(() =>
				this.client.http.GET("/api/v1/document/{channel_id}/tag", {
					params: {
						path: { channel_id },
					},
				}),
			);

			this.upsertBulk(data);

			const newIds = data.map((t) => t.id);
			list.appendPage(newIds, false, undefined);
		} catch (e) {
			list.setError(e);
			throw e;
		}
	}

	useList(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(channel_id, async (id) => {
			if (!id) return undefined;

			let list = this.tagLists.get(id);
			if (!list) {
				list = new PaginatedList();
				this.tagLists.set(id, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchTagPage(id, list);
			}

			return list;
		});

		return resource;
	}

	async list(channel_id: string): Promise<DocumentTag[]> {
		const data = await this.retryWithBackoff<DocumentTag[]>(() =>
			this.client.http.GET("/api/v1/document/{channel_id}/tag", {
				params: {
					path: { channel_id },
				},
			}),
		);

		this.upsertBulk(data);
		return data;
	}

	async fetchById(channel_id: string, tag_id: string): Promise<DocumentTag> {
		const data = await this.retryWithBackoff<DocumentTag>(() =>
			this.client.http.GET("/api/v1/document/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
				},
			}),
		);

		this.upsert(data);
		return data;
	}

	async create(channel_id: string, body: DocumentTagCreate): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/document/{channel_id}/tag", {
				params: {
					path: { channel_id },
				},
				body,
			}),
		);
	}

	async update(
		channel_id: string,
		tag_id: string,
		patch: DocumentTagPatch,
	): Promise<DocumentTag> {
		const data = await this.retryWithBackoff<DocumentTag>(() =>
			this.client.http.PATCH("/api/v1/document/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
				},
				body: patch,
			}),
		);

		this.upsert(data);
		return data;
	}

	async remove(channel_id: string, tag_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE("/api/v1/document/{channel_id}/tag/{tag_id}", {
				params: {
					path: { channel_id, tag_id },
				},
			}),
		);

		this.cache.delete(tag_id);
	}

	clear() {
		super.clear();
		for (const list of this.tagLists.values()) {
			list.clear();
		}
		this.tagLists.clear();
	}
}
