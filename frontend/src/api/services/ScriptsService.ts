import type {
	PaginationResponse,
	Script,
	ScriptCreate,
	ScriptId,
	ScriptSubscribe,
} from "sdk";
import { createUpload } from "sdk";
import { BaseService } from "../core/Service";

export class ScriptsService extends BaseService<Script> {
	protected cacheName = "script";

	getKey(item: Script): string {
		return item.id;
	}

	async fetch(id: string): Promise<Script> {
		// id is expected to be "channel_id:redex_id" for fetching
		const [channel_id, redex_id] = id.split(":");
		if (!channel_id || !redex_id) {
			throw new Error("Invalid script fetch ID, expected channel_id:redex_id");
		}

		return await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/redex/{redex_id}", {
				params: { path: { channel_id, redex_id } },
			}),
		);
	}

	async list(channel_id: string): Promise<PaginationResponse<Script>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET("/api/v1/channel/{channel_id}/redex", {
				params: { path: { channel_id }, query: { limit: 1024 } },
			}),
		);
		this.upsertBulk(data.items);
		return data;
	}

	async create(channel_id: string, script: ScriptCreate): Promise<Script> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/redex", {
				params: { path: { channel_id } },
				body: script,
			}),
		);
		this.upsert(data);
		return data;
	}

	async updateContent(
		channel_id: string,
		redex_id: string,
		media_id: string,
	): Promise<any> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.PUT(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/content",
				{
					params: { path: { channel_id, redex_id } },
					body: {
						format: "Javascript",
						location: {
							type: "Hosted",
							media_id,
						},
					},
				},
			),
		);
		return data;
	}

	async uploadAndSaveContent(
		channel_id: string,
		redex_id: string,
		code: string,
	): Promise<any> {
		const file = new File([code], "script.js", {
			type: "application/javascript",
		});

		return new Promise((resolve, reject) => {
			createUpload({
				client: this.client,
				file,
				onComplete: async (media) => {
					try {
						const res = await this.updateContent(
							channel_id,
							redex_id,
							media.id,
						);
						resolve(res);
					} catch (e) {
						reject(e);
					}
				},
				onFail: (err) => {
					reject(err);
				},
				onPause() {},
				onResume() {},
				onProgress() {},
			});
		});
	}

	async deleteScript(channel_id: string, redex_id: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE("/api/v1/channel/{channel_id}/redex/{redex_id}", {
				params: { path: { channel_id, redex_id } },
			}),
		);
		this.cache.delete(redex_id);
	}

	subscribe(channel_id: string, redex_id: ScriptId) {
		const msg: ScriptSubscribe = {
			type: "ScriptSubscribe",
			channel_id,
			redex_id,
		};
		this.client.send(msg);
	}
}
