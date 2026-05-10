import type {
	PaginationResponse,
	Script,
	ScriptCreate,
	ScriptId,
	ScriptSubscribe,
} from "sdk";
import { BaseService } from "../core/Service";

export class ScriptsService extends BaseService<Script> {
	protected cacheName = "script";

	getKey(item: Script): string {
		return item.id;
	}

	async fetch(id: string): Promise<Script> {
		// id is expected to be "channel_id:script_id" for fetching
		const [channel_id, script_id] = id.split(":");
		if (!channel_id || !script_id) {
			throw new Error("Invalid script fetch ID, expected channel_id:script_id");
		}

		return await this.retryWithBackoff<Script>(() =>
			(this.client.http as any).GET(
				"/api/v1/channel/{channel_id}/script/{script_id}",
				{
					params: { path: { channel_id, script_id } },
				},
			),
		);
	}

	async list(channel_id: string): Promise<PaginationResponse<Script>> {
		const data = await this.retryWithBackoff<PaginationResponse<Script>>(() =>
			(this.client.http as any).GET("/api/v1/channel/{channel_id}/script", {
				params: { path: { channel_id } },
			}),
		);
		const scripts = (data as any).scripts as PaginationResponse<Script>;
		this.upsertBulk(scripts.items);
		return scripts;
	}

	async create(channel_id: string, script: ScriptCreate): Promise<Script> {
		const data = await this.retryWithBackoff<Script>(() =>
			(this.client.http as any).POST("/api/v1/channel/{channel_id}/script", {
				params: { path: { channel_id } },
				body: { script },
			}),
		);
		this.upsert(data);
		return data;
	}

	async deleteScript(channel_id: string, script_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			(this.client.http as any).DELETE(
				"/api/v1/channel/{channel_id}/script/{script_id}",
				{
					params: { path: { channel_id, script_id } },
				},
			),
		);
		this.cache.delete(script_id);
	}

	subscribe(channel_id: string, script_id: ScriptId) {
		const msg: ScriptSubscribe = {
			type: "ScriptSubscribe",
			channel_id,
			script_id,
		};
		this.client.send(msg);
	}
}
