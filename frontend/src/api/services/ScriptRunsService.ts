import type {
	PaginationResponse,
	Run,
	RunCreateTrigger,
	RunId,
	ScriptId,
} from "sdk";
import { BaseService } from "../core/Service";

export class ScriptRunsService extends BaseService<Run> {
	protected cacheName = "script_run";

	getKey(item: Run): string {
		return item.id;
	}

	async fetch(id: string): Promise<Run> {
		// id is expected to be "channel_id:redex_id:eval_id"
		const [channel_id, redex_id, eval_id] = id.split(":");
		if (!channel_id || !redex_id || !eval_id) {
			throw new Error(
				"Invalid run fetch ID, expected channel_id:redex_id:eval_id",
			);
		}

		return await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}",
				{
					params: { path: { channel_id, redex_id, eval_id } },
				},
			),
		);
	}

	async list(
		channel_id: string,
		redex_id: ScriptId,
	): Promise<PaginationResponse<Run>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/eval",
				{
					params: { path: { channel_id, redex_id } },
				},
			),
		);
		this.upsertBulk(data.items);
		return data;
	}

	async trigger(
		channel_id: string,
		redex_id: ScriptId,
		create: RunCreateTrigger,
	): Promise<Run> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/trigger",
				{
					params: { path: { channel_id, redex_id } },
					body: create,
				},
			),
		);
		this.upsert(data);
		return data;
	}

	async stop(
		channel_id: string,
		redex_id: ScriptId,
		eval_id: RunId,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}/stop",
				{
					params: { path: { channel_id, redex_id, eval_id } },
				},
			),
		);
	}
}
