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
		// id is expected to be "channel_id:script_id:run_id"
		const [channel_id, script_id, run_id] = id.split(":");
		if (!channel_id || !script_id || !run_id) {
			throw new Error(
				"Invalid run fetch ID, expected channel_id:script_id:run_id",
			);
		}

		return await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/script/{script_id}/run/{run_id}",
				{
					params: { path: { channel_id, script_id, run_id } },
				},
			),
		);
	}

	async list(
		channel_id: string,
		script_id: ScriptId,
	): Promise<PaginationResponse<Run>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/script/{script_id}/run",
				{
					params: { path: { channel_id, script_id } },
				},
			),
		);
		this.upsertBulk(data.items);
		return data;
	}

	async trigger(
		channel_id: string,
		script_id: ScriptId,
		create: RunCreateTrigger,
	): Promise<Run> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/script/{script_id}/trigger",
				{
					params: { path: { channel_id, script_id } },
					body: create,
				},
			),
		);
		this.upsert(data);
		return data;
	}

	async stop(
		channel_id: string,
		script_id: ScriptId,
		run_id: RunId,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST(
				"/api/v1/channel/{channel_id}/script/{script_id}/run/{run_id}/stop",
				{
					params: { path: { channel_id, script_id, run_id } },
				},
			),
		);
	}
}
