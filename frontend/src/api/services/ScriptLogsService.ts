import { ReactiveMap } from "@solid-primitives/map";
import type { PaginationResponse, RunId, RunLogEntry, ScriptId } from "sdk";
import { BaseService } from "../core/Service";

export class ScriptLogsService extends BaseService<RunLogEntry> {
	protected cacheName = "script_log";

	getKey(item: RunLogEntry): string {
		return item.id.toString();
	}

	private logsByRun = new ReactiveMap<string, string[]>();

	// add runId to logs since backend doesnt include it
	private processLogs(runId: string, items: RunLogEntry[]) {
		const keys = items.map((item) => this.getKey(item));

		const existing = this.logsByRun.get(runId) ?? [];
		const merged = Array.from(new Set([...existing, ...keys])).sort((a, b) => {
			return Number(a) - Number(b);
		});

		this.logsByRun.set(runId, merged);
		this.upsertBulk(items);
	}

	subscribe(channel_id: string, script_id: ScriptId) {
		this.client.send({
			type: "ScriptSubscribe",
			channel_id,
			script_id,
		});
	}

	async fetch(_id: string): Promise<RunLogEntry> {
		throw new Error("Use list() to fetch logs");
	}

	async list(
		channel_id: string,
		script_id: ScriptId,
		run_id: RunId,
	): Promise<PaginationResponse<RunLogEntry>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/script/{script_id}/run/{run_id}/log",
				{
					params: { path: { channel_id, script_id, run_id } },
				},
			),
		);
		this.processLogs(run_id, data.items);
		return data;
	}

	getLogsForRun(run_id: string): RunLogEntry[] {
		const ids = this.logsByRun.get(run_id);
		if (!ids) return [];
		return ids
			.map((id) => this.cache.get(id))
			.filter((l): l is RunLogEntry => l != null);
	}

	override clear() {
		super.clear();
		this.logsByRun.clear();
	}
}
