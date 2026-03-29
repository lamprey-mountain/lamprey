import type { Pagination, Webhook } from "sdk";
import { BaseService } from "../core/Service";
import { createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

const log = logger.for("api/webhooks");

export class WebhooksService extends BaseService<Webhook> {
	protected cacheName = "webhook";

	private _channelLists = new Map<string, PaginatedList>();

	getKey(item: Webhook): string {
		return item.id;
	}

	async fetch(id: string): Promise<Webhook> {
		const data = await this.retryWithBackoff<Webhook>(() =>
			this.client.http.GET("/api/v1/webhook/{webhook_id}", {
				params: { path: { webhook_id: id } },
			})
		);
		this.upsert(data);
		return data;
	}

	private async fetchChannelPage(
		channel_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<Webhook>>(() =>
				this.client.http.GET("/api/v1/channel/{channel_id}/webhook", {
					params: {
						path: { channel_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((webhook) => webhook.id);
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.id);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useChannelList(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(channel_id, async (cid) => {
			if (!cid) return undefined;

			let list = this._channelLists.get(cid);
			if (!list) {
				list = new PaginatedList();
				this._channelLists.set(cid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchChannelPage(cid, list);
			}

			return list;
		});

		return resource;
	}
}
