import {
	type AuditLogEntry,
	type Channel,
	Pagination,
	type RoomMember,
	type Tag,
	type User,
	type UserWithRelationship,
	type Webhook,
} from "sdk";
import { BaseService } from "../core/Service";
import { createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

const log = logger.for("api/audit_log");

interface AuditLogPaginationResponse {
	audit_log_entries: AuditLogEntry[];
	threads: Channel[];
	users: User[];
	room_members: RoomMember[];
	webhooks: Webhook[];
	tags: Tag[];
	has_more: boolean;
	cursor?: string;
}

export class AuditLogService extends BaseService<AuditLogEntry> {
	protected cacheName = "audit_log";

	private _roomLists = new Map<string, PaginatedList>();

	getKey(item: AuditLogEntry): string {
		return item.id;
	}

	async fetch(id: string): Promise<AuditLogEntry> {
		throw new Error("Use fetchByRoom(room_id) and cache lookup");
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const result = await this.retryWithBackoff<any>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/audit-logs", {
					params: {
						path: { room_id },
						query: {
							dir: "b",
							limit: 100,
							from: cursor,
						},
					},
				})
			);
			const data = result.data as AuditLogPaginationResponse;

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
				this.store.channels.upsert(tag as any); // Tags are stored in channels cache
			}

			// Cache audit log entries
			this.upsertBulk(data.audit_log_entries);

			const newIds = data.audit_log_entries.map((entry) => entry.id);
			list.appendPage(newIds, data.has_more, data.cursor);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useList(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(room_id, async (rid) => {
			if (!rid) return undefined;

			let list = this._roomLists.get(rid);
			if (!list) {
				list = new PaginatedList();
				this._roomLists.set(rid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchRoomPage(rid, list);
			}

			return list;
		});

		return resource;
	}
}
