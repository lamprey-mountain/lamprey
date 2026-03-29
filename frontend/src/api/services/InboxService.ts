import type { Channel, InboxListParams, Notification, Pagination, Room } from "sdk";
import type { Message } from "sdk";
import { BaseService } from "../core/Service";
import {
	createEffect,
	createResource,
	onCleanup,
	type Resource,
} from "solid-js";
import { logger } from "../../logger";

const log = logger.for("api/inbox");

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

	async fetch(id: string): Promise<Notification> {
		throw new Error("Use useList() to fetch inbox notifications");
	}

	useList(
		params: () => InboxListParams,
	): InboxListResult {
		const [resource, { refetch }] = createResource(
			() => [params(), this.store.session()] as const,
			async ([p, session]) => {
				if (session?.status !== "Authorized") {
					return undefined;
				}

				const result = await this.retryWithBackoff<any>(() =>
					this.client.http.GET("/api/v1/inbox", {
						params: {
							query: {
								...p,
								dir: "b",
								limit: 50,
							},
						},
					})
				);

				const data = result.data as {
					notifications: Notification[];
					channels: Channel[];
					messages: Message[];
					rooms: Room[];
					has_more: boolean;
					total: number;
				};

				// Cache notifications
				this.upsertBulk(data.notifications);

				// Cache related entities
				for (const channel of data.channels) {
					this.store.channels.upsert(channel as any);
				}
				for (const message of data.messages) {
					this.store.messages.upsert(message as any);
				}
				for (const room of data.rooms) {
					this.store.rooms.upsert(room as any);
				}

				return {
					items: data.notifications,
					total: data.total,
					has_more: data.has_more,
					channels: data.channels as any,
					messages: data.messages as any,
					rooms: data.rooms as any,
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
			})
		);
	}

	async markUnread(message_ids: string[]): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/inbox/mark-unread", {
				body: { message_ids },
			})
		);
	}
}
