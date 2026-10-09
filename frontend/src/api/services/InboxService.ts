import type {
	Channel,
	InboxListParams,
	Message,
	Notification,
	Pagination,
	Room,
	RoomMember,
	ThreadMember,
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

	// TODO: use PaginatedList instead of InboxListResult
	useList(params: () => InboxListParams): InboxListResult {
		const [resource, { refetch }] = createResource(
			() => [params(), this.store.session()] as const,
			async ([p, session]) => {
				if (session?.status !== "Authorized" && session?.status !== "Sudo") {
					return undefined;
				}

				const data = await this.retryWithBackoff<{
					notifications: Notification[];
					threads: Channel[];
					messages: Message[];
					room_members: RoomMember[];
					thread_members: ThreadMember[];
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

				this.upsertBulk(data.notifications);
				for (const channel of data.threads) {
					this.store.channels.upsert(channel);
				}
				for (const message of data.messages) {
					this.store.messages.upsert(message);
				}
				for (const rm of data.room_members) {
					this.store.room_members.upsert(rm);
				}
				for (const tm of data.thread_members) {
					this.store.thread_members.upsert(tm);
				}

				return {
					items: data.notifications.toReversed(),
					total: data.total,
					has_more: data.has_more,
					messages: data.messages,
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

	clear() {
		super.clear();
		for (const v of this._listings.values()) {
			v.refetch();
		}
		this._listings.clear();
	}
}
