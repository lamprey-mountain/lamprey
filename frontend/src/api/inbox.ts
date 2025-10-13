import type {
	InboxListParams,
	Notification,
	Pagination,
	Room,
	Thread,
} from "sdk";
import {
	batch,
	createEffect,
	createResource,
	onCleanup,
	type Resource,
} from "solid-js";
import type { Api } from "../api.tsx";
import type { Message } from "sdk";

export interface NotificationPagination extends Pagination<Notification> {
	threads: Thread[];
	messages: Message[];
	rooms: Room[];
}

export class Inbox {
	api: Api = null as unknown as Api;
	cache = new Map<string, Notification>();
	_listings = new Map<string, { refetch: () => void }>();

	list(
		params: () => InboxListParams,
	): [Resource<NotificationPagination>, { refetch: () => void }] {
		const paginate = async (
			p: InboxListParams,
			pagination?: Pagination<Notification>,
		) => {
			if (pagination && !pagination.has_more) {
				return pagination as NotificationPagination;
			}

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
				for (const thread of data.threads) {
					this.api.threads.cache.set(thread.id, thread);
				}
				for (const message of data.messages) {
					this.api.messages.cache.set(message.id, message);
				}
				for (const room of data.rooms) {
					this.api.rooms.cache.set(room.id, room);
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

		createEffect(() => {
			const key = JSON.stringify(params());
			this._listings.set(key, { refetch });
			onCleanup(() => {
				this._listings.delete(key);
			});
		});

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
