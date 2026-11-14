import type { Channel, Pagination } from "sdk";
import { createResource, type Resource } from "solid-js";
import type { ChannelReadState } from "@/types/chat";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/dms");

// TODO: drop the path casts once the openapi schema is regenerated with the dm routes
// biome-ignore lint/suspicious/noExplicitAny: routes are missing from the generated schema
const stale = (path: string): any => path;

export class DmsService extends BaseService<Channel> {
	protected cacheName = "dm";

	private _list: PaginatedList | null = null;

	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(_id: string): Promise<Channel> {
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
			const data = await this.retryWithBackoff<Pagination<Channel>>(() =>
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

	/** refetch the first page of the dm list */
	async refresh(): Promise<void> {
		if (!this._list) return;

		this._list.clear();
		await this.fetchPage(this._list).catch((e) => {
			log.error("while refreshing dms", e);
		});
	}

	/** hide this dm from the dm list until it's shown again or a message is sent */
	async hide(channel_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT(stale("/api/v1/channel/{channel_id}/hide"), {
				params: { path: { channel_id } },
			}),
		);
		this.setHidden(channel_id, true);
	}

	/** show a hidden dm by (re)opening it with the other user */
	async showWithUser(user_id: string): Promise<void> {
		const data = await this.retryWithBackoff<Channel>(() =>
			this.client.http.POST("/api/v1/user/@self/dm/{target_id}", {
				params: { path: { target_id: user_id } },
			}),
		);
		if (data) {
			this.upsert(data);
			this._list?.appendId(data.id);
		}
	}

	private setHidden(channel_id: string, hidden: boolean) {
		const cached = this.cache.get(channel_id);
		if (!cached) return;

		const next: ChannelReadState = { ...cached, dm_hidden: hidden };
		this.cache.set(channel_id, next);
	}

	clear() {
		super.clear();
		if (this._list) {
			this._list.clear();
		}
	}
}
