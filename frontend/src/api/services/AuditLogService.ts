import type {
	AuditLogEntry,
	Channel,
	RoomMember,
	Tag,
	User,
	UserWithRelationship,
	Webhook,
} from "sdk";
import { createResource, type Resource } from "solid-js";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/audit_log");

interface AuditLogPaginationResponse {
	audit_log_entries: AuditLogEntry[];
	threads: Channel[];
	users: User[];
	room_members: RoomMember[];
	webhooks: Webhook[];
	tags: Tag[];
	has_more: boolean;
	cursor?: string | null;
}

export class AuditLogService extends BaseService<AuditLogEntry> {
	protected cacheName = "audit_log";

	private _lists = new Map<string, PaginatedList>();

	getKey(item: AuditLogEntry): string {
		return item.id;
	}

	async fetch(_id: string): Promise<AuditLogEntry> {
		throw new Error("Use fetchByRoom(room_id) and cache lookup");
	}

	private applyPage(
		data: AuditLogPaginationResponse,
		list: PaginatedList,
	): void {
		// Cache related entities
		for (const thread of data.threads) {
			this.store.channels.upsert(thread);
		}
		for (const user of data.users) {
			const userWithRelationship: UserWithRelationship = {
				...user,
				relationship: {
					note: null,
					relation: null,
					petname: null,
					until: null,
				},
			};
			this.store.users.upsert(userWithRelationship);
		}
		for (const member of data.room_members) {
			this.store.roomMembers.upsert(member);
		}
		for (const webhook of data.webhooks) {
			this.store.webhooks.upsert(webhook);
		}
		for (const tag of data.tags) {
			this.store.tags.upsert(tag);
		}

		// Cache audit log entries
		this.upsertBulk(data.audit_log_entries);

		// the api returns entries in chronological order
		const newIds = data.audit_log_entries.map((entry) => entry.id).toReversed();
		list.appendPage(newIds, data.has_more, data.cursor);
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
		channel_id?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<AuditLogPaginationResponse>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/audit-logs", {
					params: {
						path: { room_id },
						query: {
							dir: "b",
							limit: 100,
							from: cursor,
							channel_id: channel_id ? [channel_id] : undefined,
						},
					},
				}),
			);
			this.applyPage(data, list);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	private async fetchUserPage(
		user_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<AuditLogPaginationResponse>(() =>
				this.client.http.GET("/api/v1/user/{user_id}/audit-logs", {
					params: {
						path: { user_id },
						query: { dir: "b", limit: 100, from: cursor },
					},
				}),
			);
			this.applyPage(data, list);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useList(
		room_id: () => string | undefined,
		channel_id?: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(
			() => {
				const rid = room_id();
				if (!rid) return undefined;
				return { room_id: rid, channel_id: channel_id?.() };
			},
			async (ids) => {
				if (!ids) return undefined;

				const key = `room:${ids.room_id}:${ids.channel_id ?? ""}`;
				let list = this._lists.get(key);
				if (!list) {
					list = new PaginatedList();
					this._lists.set(key, list);
				}

				if (list.state.ids.length === 0 && !list.state.isLoading) {
					await this.fetchRoomPage(
						ids.room_id,
						list,
						undefined,
						ids.channel_id,
					);
				}

				return list;
			},
		);

		return resource;
	}

	useUserList(
		user_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(
			() => user_id(),
			async (uid) => {
				if (!uid) return undefined;

				const key = `user:${uid}`;
				let list = this._lists.get(key);
				if (!list) {
					list = new PaginatedList();
					this._lists.set(key, list);
				}

				if (list.state.ids.length === 0 && !list.state.isLoading) {
					await this.fetchUserPage(uid, list);
				}

				return list;
			},
		);

		return resource;
	}

	clear() {
		super.clear();
		for (const list of this._lists.values()) {
			list.clear();
		}
		this._lists.clear();
	}
}
