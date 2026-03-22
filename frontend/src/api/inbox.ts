import type {
	Channel,
	InboxListParams,
	Notification,
	Pagination,
	Room,
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
import { logger } from "../logger.ts";
import type { RoomsService } from "./services/RoomsService";

const log = logger.for("api/inbox");

export interface NotificationPagination extends Pagination<Notification> {
	channels: Channel[];
	messages: Message[];
	rooms: Room[];
}

export class Inbox {
	api: Api = null as unknown as Api;
	rooms: RoomsService = null as unknown as RoomsService;
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
				log.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.notifications) {
					this.cache.set(item.id, item);
				}
				for (const channel of data.channels) {
					this.api.channels.cache.set(channel.id, channel as any);
				}
				for (const message of data.messages) {
					this.api.store.messages.upsert(message as any);
				}
				for (const room of data.rooms) {
					this.rooms.cache.set(room.id, room as any);
				}
			});

			const prevP = pagination as NotificationPagination | undefined;
			return {
				...data,
				items: [...(pagination?.items ?? []), ...data.notifications],
				channels: [...(prevP?.channels ?? []), ...data.channels as any],
				messages: [...(prevP?.messages ?? []), ...data.messages as any],
				rooms: [...(prevP?.rooms ?? []), ...data.rooms as any],
			} as NotificationPagination;
		};

		const [resource, { refetch }] = createResource(
			() => [params(), this.api.session()] as const,
			async ([p, session]) => {
				if (session?.status !== "Authorized") {
					return {
						items: [],
						total: 0,
						has_more: false,
						channels: [],
						messages: [],
						rooms: [],
					};
				}
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
