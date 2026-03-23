import type { Api } from "@/api";
import type {
	RoomAnalyticsChannel,
	RoomAnalyticsInvites,
	RoomAnalyticsMembersCount,
	RoomAnalyticsMembersJoin,
	RoomAnalyticsMembersLeave,
	RoomAnalyticsOverview,
	Time,
} from "sdk";

export type Aggregation = "Hourly" | "Daily" | "Weekly" | "Monthly";

export type RoomAnalyticsParams = {
	start?: Time;
	end?: Time;
	aggregate: Aggregation;
	limit?: number;
};

export class RoomAnalytics {
	api: Api = null as unknown as Api;

	async getOverview(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsOverview[]> {
		const { data, error } = await (this.api.client.http as any).GET(
			"/api/v1/room/{room_id}/analytics/overview",
			{
				params: {
					path: { room_id },
					query: {
						start: query.start,
						end: query.end,
						aggregate: query.aggregate,
						limit: query.limit,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async getMembersCount(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersCount[]> {
		const { data, error } = await (this.api.client.http as any).GET(
			"/api/v1/room/{room_id}/analytics/members-count",
			{
				params: {
					path: { room_id },
					query: {
						start: query.start,
						end: query.end,
						aggregate: query.aggregate,
						limit: query.limit,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async getMembersJoin(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersJoin[]> {
		const { data, error } = await (this.api.client.http as any).GET(
			"/api/v1/room/{room_id}/analytics/members-join",
			{
				params: {
					path: { room_id },
					query: {
						start: query.start,
						end: query.end,
						aggregate: query.aggregate,
						limit: query.limit,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async getMembersLeave(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersLeave[]> {
		const { data, error } = await (this.api.client.http as any).GET(
			"/api/v1/room/{room_id}/analytics/members-leave",
			{
				params: {
					path: { room_id },
					query: {
						start: query.start,
						end: query.end,
						aggregate: query.aggregate,
						limit: query.limit,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async getChannels(
		room_id: string,
		query: RoomAnalyticsParams,
		channel_id?: string,
	): Promise<RoomAnalyticsChannel[]> {
		const { data, error } = await (this.api.client.http as any).GET(
			"/api/v1/room/{room_id}/analytics/channels",
			{
				params: {
					path: { room_id },
					query: {
						start: query.start,
						end: query.end,
						aggregate: query.aggregate,
						limit: query.limit,
						channel_id,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}
}
