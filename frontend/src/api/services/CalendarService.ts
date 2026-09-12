import type { IDBPDatabase } from "idb";
import type {
	CalendarEvent,
	CalendarEventCreate,
	CalendarEventPatch,
} from "sdk/types";
import type { ApiDB } from "@/lib/sync/db";
import { BaseService } from "../core/Service";
import type { RootStore } from "../core/Store";

// TODO: caching for calendar events and overwrites
// cf. frontend/src/api/services/AuditLogService.ts; maybe copy useList?
export class CalendarService extends BaseService<CalendarEvent> {
	protected cacheName = "calendar_events";

	constructor(store: RootStore, getDb?: () => IDBPDatabase<ApiDB> | undefined) {
		super(store, getDb);
	}

	getKey(item: CalendarEvent): string {
		return item.id;
	}

	async fetch(_id: string): Promise<CalendarEvent> {
		throw new Error("fetch not implemented for CalendarService");
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
