import type {
	Eval,
	EvalCreateManual,
	EvalId,
	PaginationResponse,
	RedexId,
} from "sdk";
import { BaseService } from "../core/Service";

export class RedexEvalsService extends BaseService<Eval> {
	protected cacheName = "script_run";

	getKey(item: Eval): string {
		return item.id;
	}

	async fetch(id: string): Promise<Eval> {
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

	// PERF: don't refetch if we already have a list
	async list(
		channel_id: string,
		redex_id: RedexId,
	): Promise<PaginationResponse<Eval>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/eval",
				{
					params: {
						path: { channel_id, redex_id },
						query: { limit: 32, dir: "b" },
					},
				},
			),
		);
		this.upsertBulk(data.items);
		return data;
	}

	async trigger(
		channel_id: string,
		redex_id: RedexId,
		create: EvalCreateManual,
	): Promise<Eval> {
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
		redex_id: RedexId,
		eval_id: EvalId,
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
