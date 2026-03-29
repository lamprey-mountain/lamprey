import type { Channel, Pagination } from "sdk";
import { createResource, type Resource } from "solid-js";
import { logger } from "../../logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/dms");

export class DmsService extends BaseService<Channel> {
	protected cacheName = "dm";

	private _list: PaginatedList | null = null;

	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(id: string): Promise<Channel> {
		throw new Error("Use channels.fetch() for DM channels");
	}

	private async fetchPage(list: PaginatedList, cursor?: string): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;

		// return empty list if session is logged out
		const session = this.store.session();
		if (!session || session.status === "Unauthorized") {
			list.setLoading(false);
			return;
		}

		list.setLoading(true);

		try {
			const result = await this.retryWithBackoff<any>(() =>
				this.client.http.GET("/api/v1/user/{user_id}/dm", {
					params: {
						path: { user_id: "@self" },
						query: {
							dir: "b", // newest first
							limit: 100,
							from: cursor,
						},
					},
				}),
			);
			const data = result as Pagination<Channel>;

			this.upsertBulk(data.items);

			const newIds = data.items.map((item) => item.id);
			const nextCursor = data.items.at(-1)?.last_version_id ?? undefined;
			list.appendPage(newIds, data.has_more, nextCursor);
		} catch (e) {
			log.error("while fetching dms", e);
			list.setError(e);
			throw e;
		}
	}

	useList(): Resource<PaginatedList | undefined> {
		const [resource] = createResource(async () => {
			if (!this._list) {
				this._list = new PaginatedList();
			}

			if (this._list.state.ids.length === 0 && !this._list.state.isLoading) {
				await this.fetchPage(this._list);
			}

			return this._list;
		});

		return resource;
	}
}
