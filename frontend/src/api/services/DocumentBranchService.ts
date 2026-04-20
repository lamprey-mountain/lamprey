import type {
	DocumentBranch,
	DocumentBranchCreate,
	DocumentBranchPatch,
	Pagination,
} from "sdk";
import { createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

export class DocumentBranchService extends BaseService<DocumentBranch> {
	protected cacheName = "document_branch";

	private branchLists = new Map<string, PaginatedList>();

	getKey(item: DocumentBranch): string {
		return item.id;
	}

	async fetch(_id: string): Promise<DocumentBranch> {
		throw new Error("Use list() or fetchById(channel_id, branch_id) instead");
	}

	private async fetchBranchPage(
		channel_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<DocumentBranch>>(() =>
				this.client.http.GET("/api/v1/document/{channel_id}/branch", {
					params: {
						path: { channel_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				}),
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((b) => b.id);
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.id);
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

			let list = this.branchLists.get(id);
			if (!list) {
				list = new PaginatedList();
				this.branchLists.set(id, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchBranchPage(id, list);
			}

			return list;
		});

		return resource;
	}

	async list(
		channel_id: string,
		states?: DocumentBranch["state"][],
	): Promise<Pagination<DocumentBranch>> {
		const query: Record<string, string> = {};
		if (states && states.length > 0) {
			query.state = states.join(",");
		}

		const data = await this.retryWithBackoff<Pagination<DocumentBranch>>(() =>
			this.client.http.GET("/api/v1/document/{channel_id}/branch", {
				params: {
					path: { channel_id },
					query,
				},
			}),
		);

		this.upsertBulk(data.items);
		return data;
	}

	async fetchById(
		channel_id: string,
		branch_id: string,
	): Promise<DocumentBranch> {
		const data = await this.retryWithBackoff<DocumentBranch>(() =>
			this.client.http.GET("/api/v1/document/{channel_id}/branch/{branch_id}", {
				params: {
					path: { channel_id, branch_id },
				},
			}),
		);

		this.upsert(data);
		return data;
	}

	async update(
		channel_id: string,
		branch_id: string,
		patch: DocumentBranchPatch,
	): Promise<DocumentBranch> {
		const data = await this.retryWithBackoff<DocumentBranch>(() =>
			this.client.http.PATCH(
				"/api/v1/document/{channel_id}/branch/{branch_id}",
				{
					params: {
						path: { channel_id, branch_id },
					},
					body: patch,
				},
			),
		);

		this.upsert(data);
		return data;
	}

	async close(channel_id: string, branch_id: string): Promise<DocumentBranch> {
		const data = await this.retryWithBackoff<DocumentBranch>(() =>
			this.client.http.POST(
				"/api/v1/document/{channel_id}/branch/{branch_id}/close",
				{
					params: {
						path: { channel_id, branch_id },
					},
				},
			),
		);

		this.upsert(data);
		return data;
	}

	async fork(
		channel_id: string,
		parent_id: string,
		body: DocumentBranchCreate,
	): Promise<DocumentBranch> {
		const data = await this.retryWithBackoff<DocumentBranch>(() =>
			this.client.http.POST(
				"/api/v1/document/{channel_id}/branch/{parent_id}/fork",
				{
					params: {
						path: { channel_id, parent_id },
					},
					body,
				},
			),
		);

		this.upsert(data);
		return data;
	}

	async merge(channel_id: string, branch_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/document/{channel_id}/branch/{branch_id}/merge",
				{
					params: {
						path: { channel_id, branch_id },
					},
					body: {},
				},
			),
		);
	}

	async sync(channel_id: string, branch_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/document/{channel_id}/branch/{branch_id}/sync",
				{
					params: {
						path: { channel_id, branch_id },
					},
					body: {
						sync_from_branch_id: null,
					},
				},
			),
		);
	}

	protected afterUpsert(item: DocumentBranch): void {
		const list = this.branchLists.get(item.document_id);
		if (list) {
			list.prependId(item.id);
		}
	}

	protected afterDelete(id: string, item?: DocumentBranch): void {
		if (item) {
			const list = this.branchLists.get(item.document_id);
			if (list) {
				list.removeId(id);
			}
		}
	}

	clear() {
		super.clear();
		for (const list of this.branchLists.values()) {
			list.clear();
		}
		this.branchLists.clear();
	}
}
