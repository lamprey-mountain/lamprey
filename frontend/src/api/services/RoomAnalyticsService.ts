import type {
	RoomAnalyticsChannel,
	RoomAnalyticsMembersCount,
	RoomAnalyticsMembersJoin,
	RoomAnalyticsMembersLeave,
	RoomAnalyticsOverview,
	Time,
} from "sdk";
import { BaseService } from "../core/Service";

export type Aggregation = "Hourly" | "Daily" | "Weekly" | "Monthly";

export type RoomAnalyticsParams = {
	start?: Time;
	end?: Time;
	aggregate: Aggregation;
	limit?: number;
};

export class RoomAnalyticsService extends BaseService<never> {
	protected cacheName = "room_analytics";

	getKey(_item: never): string {
		throw new Error("RoomAnalyticsService does not cache items");
	}

	async fetch(_id: string): Promise<never> {
		throw new Error("RoomAnalyticsService does not fetch single items");
	}

	async getOverview(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsOverview[]> {
		const result = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room/{room_id}/analytics/overview", {
				params: {
					path: { room_id },
					query,
				},
			}),
		);
		return result as RoomAnalyticsOverview[];
	}

	async getMembersCount(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersCount[]> {
		const result = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room/{room_id}/analytics/members-count", {
				params: {
					path: { room_id },
					query,
				},
			}),
		);
		return result as RoomAnalyticsMembersCount[];
	}

	async getMembersJoin(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersJoin[]> {
		const result = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room/{room_id}/analytics/members-join", {
				params: {
					path: { room_id },
					query,
				},
			}),
		);
		return result as RoomAnalyticsMembersJoin[];
	}

	async getMembersLeave(
		room_id: string,
		query: RoomAnalyticsParams,
	): Promise<RoomAnalyticsMembersLeave[]> {
		const result = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room/{room_id}/analytics/members-leave", {
				params: {
					path: { room_id },
					query,
				},
			}),
		);
		return result as RoomAnalyticsMembersLeave[];
	}

	async getChannels(
		room_id: string,
		query: RoomAnalyticsParams,
		channel_id?: string,
	): Promise<RoomAnalyticsChannel[]> {
		const result = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/room/{room_id}/analytics/channels", {
				params: {
					path: { room_id },
					query: { ...query, channel_id },
				},
			}),
		);
		return result as RoomAnalyticsChannel[];
	}
}
