import type { PaginationResponse, Run, RunId, ScriptId } from "sdk";
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

		return await this.retryWithBackoff<Run>(() =>
			(this.client.http as any).GET(
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
		const data = await this.retryWithBackoff<PaginationResponse<Run>>(() =>
			(this.client.http as any).GET(
				"/api/v1/channel/{channel_id}/script/{script_id}/run",
				{
					params: { path: { channel_id, script_id } },
				},
			),
		);
		const runs = (data as any).runs as PaginationResponse<Run>;
		this.upsertBulk(runs.items);
		return runs;
	}

	async trigger(
		channel_id: string,
		script_id: ScriptId,
		run: any,
	): Promise<Run> {
		const data = await this.retryWithBackoff<Run>(() =>
			(this.client.http as any).POST(
				"/api/v1/channel/{channel_id}/script/{script_id}/trigger",
				{
					params: { path: { channel_id, script_id } },
					body: { run },
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
			(this.client.http as any).POST(
				"/api/v1/channel/{channel_id}/script/{script_id}/run/{run_id}/stop",
				{
					params: { path: { channel_id, script_id, run_id } },
					body: {},
				},
			),
		);
	}
}
