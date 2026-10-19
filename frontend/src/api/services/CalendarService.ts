import type { IDBPDatabase } from "idb";
import type {
	CalendarEvent,
	CalendarEventCreate,
	CalendarEventPatch,
} from "sdk/types";
import { createResource, type Resource } from "solid-js";
import type { ApiDB } from "@/lib/sync/db";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";
import type { RootStore } from "../core/Store";

// TODO: caching for calendar overwrites
// cf. frontend/src/api/services/AuditLogService.ts
// sync events are applied by the store, which upserts/deletes through this
// service; the afterUpsert/afterDelete hooks keep the per-channel id lists in
// sync so the UI can render reactively.
export class CalendarService extends BaseService<CalendarEvent> {
	protected cacheName = "calendar_events";

	private eventLists = new Map<string, PaginatedList>();

	constructor(store: RootStore, getDb?: () => IDBPDatabase<ApiDB> | undefined) {
		super(store, getDb);
	}

	getKey(item: CalendarEvent): string {
		return item.id;
	}

	async fetch(_id: string): Promise<CalendarEvent> {
		throw new Error("fetch not implemented for CalendarService");
	}

	useList(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(channel_id, async (id) => {
			if (!id) return undefined;

			let list = this.eventLists.get(id);
			if (!list) {
				list = new PaginatedList({ has_more: false });
				this.eventLists.set(id, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				list.setLoading(true);
				try {
					await this.listEvents(id);
				} catch (e) {
					list.setError(e);
				} finally {
					list.setLoading(false);
				}
			}

			return list;
		});

		return resource;
	}

	protected afterUpsert(item: CalendarEvent): void {
		const list = this.eventLists.get(item.channel_id);
		if (list) list.appendId(item.id);
	}

	protected afterDelete(id: string, item?: CalendarEvent): void {
		if (!item) return;
		const list = this.eventLists.get(item.channel_id);
		list?.removeId(id);
	}

	clear() {
		super.clear();
		for (const list of this.eventLists.values()) {
			list.clear();
		}
		this.eventLists.clear();
	}

	async listEvents(channelId: string): Promise<CalendarEvent[]> {
		const { data, error } = await this.client.http.GET(
			"/api/v1/calendar/{channel_id}/event",
			{
				params: { path: { channel_id: channelId } },
			},
		);

		if (error) throw error;
		this.upsertBulk(data.items);
		return data.items;
	}

	async createEvent(
		channelId: string,
		event: CalendarEventCreate,
	): Promise<CalendarEvent> {
		const { data, error } = await this.client.http.POST(
			"/api/v1/calendar/{channel_id}/event",
			{
				params: { path: { channel_id: channelId } },
				body: event,
			},
		);

		if (error) throw error;
		this.upsert(data);
		return data;
	}

	async updateEvent(
		channelId: string,
		eventId: string,
		patch: CalendarEventPatch,
	): Promise<CalendarEvent> {
		const { data, error } = await this.client.http.PATCH(
			"/api/v1/calendar/{channel_id}/event/{event_id}",
			{
				params: { path: { channel_id: channelId, event_id: eventId } },
				body: patch,
			},
		);

		if (error) throw error;
		this.upsert(data);
		return data;
	}

	async deleteEvent(channelId: string, eventId: string): Promise<void> {
		const { data, error } = await this.client.http.DELETE(
			"/api/v1/calendar/{channel_id}/event/{event_id}",
			{
				params: { path: { channel_id: channelId, event_id: eventId } },
			},
		);

		if (error) throw error;
		this.delete(eventId);
	}
}
