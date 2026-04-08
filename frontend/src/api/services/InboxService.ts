import type {
	Channel,
	InboxListParams,
	Message,
	Notification,
	Pagination,
	Room,
} from "sdk";
import {
	createEffect,
	createResource,
	onCleanup,
	type Resource,
} from "solid-js";
import { logger } from "@/utils/logger";
import { BaseService } from "../core/Service";

const _log = logger.for("api/inbox");

export interface NotificationPagination extends Pagination<Notification> {
	channels: Channel[];
	messages: Message[];
	rooms: Room[];
}

export interface InboxListResult {
	resource: Resource<NotificationPagination | undefined>;
	refetch: () => void;
}

export class InboxService extends BaseService<Notification> {
	protected cacheName = "inbox_notification";

	private _listings = new Map<string, { refetch: () => void }>();

	getKey(item: Notification): string {
		return item.id;
	}

	async fetch(_id: string): Promise<Notification> {
		throw new Error("Use useList() to fetch inbox notifications");
	}

	useList(params: () => InboxListParams): InboxListResult {
		const [resource, { refetch }] = createResource(
			() => [params(), this.store.session()] as const,
			async ([p, session]) => {
				if (session?.status !== "Authorized") {
					return undefined;
				}

				const data = await this.retryWithBackoff<{
					notifications: Notification[];
					channels: Channel[];
					messages: Message[];
					rooms: Room[];
					has_more: boolean;
					total: number;
				}>(() =>
					this.client.http.GET("/api/v1/inbox", {
						params: {
							query: {
								...p,
								dir: "b",
								limit: 50,
							},
						},
					}),
				);

				// Cache notifications
				this.upsertBulk(data.notifications);

				// Cache related entities
				for (const channel of data.channels) {
					this.store.channels.upsert(channel);
				}
				for (const message of data.messages) {
					this.store.messages.upsert(message);
				}
				for (const room of data.rooms) {
					this.store.rooms.upsert(room);
				}

				return {
					items: data.notifications,
					total: data.total,
					has_more: data.has_more,
					channels: data.channels,
					messages: data.messages,
					rooms: data.rooms,
				} as NotificationPagination;
			},
		);

		createEffect(() => {
			const key = JSON.stringify(params());
			this._listings.set(key, { refetch });
			onCleanup(() => {
				this._listings.delete(key);
			});
		});

		return { resource, refetch };
	}

	async markRead(message_ids: string[]): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/inbox/mark-read", {
				body: { message_ids },
			}),
		);
	}

	async markUnread(message_ids: string[]): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/inbox/mark-unread", {
				body: { message_ids },
			}),
		);
	}
}
