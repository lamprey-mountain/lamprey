import type { PushCreate, PushInfo } from "sdk";
import { BaseService } from "../core/Service";
import { logger } from "../../logger";

const log = logger.for("api/push");

export class PushService extends BaseService<PushInfo> {
	protected cacheName = "push";

	getKey(item: PushInfo): string {
		// PushInfo doesn't have a unique ID, use endpoint as key
		return item.endpoint;
	}

	async fetch(id: string): Promise<PushInfo> {
		return await this.retryWithBackoff<PushInfo>(() =>
			this.client.http.GET("/api/v1/push")
		);
	}

	async register(body: PushCreate): Promise<PushInfo> {
		const data = await this.retryWithBackoff<PushInfo>(() =>
			this.client.http.POST("/api/v1/push", {
				body,
			})
		);
		this.upsert(data);
		return data;
	}

	async delete(): Promise<void> {
		await this.retryWithBackoff(() => this.client.http.DELETE("/api/v1/push"));
		// Clear cached data
		this.cache.clear();
	}

	async getInfo(): Promise<PushInfo> {
		return await this.fetch("@self");
	}
}
