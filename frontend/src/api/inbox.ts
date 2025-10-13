import type { Notification, Pagination } from "sdk";
import { batch, createResource, type Resource } from "solid-js";
import type { Api } from "../api.tsx";
import type { InboxListParams } from "sdk";

export class Inbox {
	api: Api = null as unknown as Api;
	cache = new Map<string, Notification>();

	list(
		params: () => InboxListParams,
	): [Resource<Pagination<Notification>>, { refetch: () => void }] {
		const paginate = async (
			p: InboxListParams,
			pagination?: Pagination<Notification>,
		) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET("/api/v1/inbox", {
				params: {
					query: {
						...p,
						dir: "b",
						limit: 50,
						from: pagination?.items.at(-1)?.id,
					},
				},
			});

			if (error) {
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...(pagination?.items ?? []), ...data.items],
			};
		};

		const [resource, { refetch }] = createResource(
			params,
			async (p) => {
				// This is not a paginating list right now, so just return the first page
				return await paginate(p);
			},
		);

		return [resource, { refetch }];
	}

	async markRead(message_ids: string[]) {
		const { error } = await this.api.client.http.POST(
			"/api/v1/inbox/mark-read",
			{
				body: { message_ids },
			},
		);
		if (error) throw error;
	}

	async markUnread(message_ids: string[]) {
		const { error } = await this.api.client.http.POST(
			"/api/v1/inbox/mark-unread",
			{
				body: { message_ids },
			},
		);
		if (error) throw error;
	}
}
