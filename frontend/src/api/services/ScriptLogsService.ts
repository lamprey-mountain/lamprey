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

	subscribe(channel_id: string, redex_id: ScriptId) {
		this.client.send({
			type: "ScriptSubscribe",
			channel_id,
			script_id: redex_id,
		});
	}

	async fetch(_id: string): Promise<RunLogEntry> {
		throw new Error("Use list() to fetch logs");
	}

	async list(
		channel_id: string,
		redex_id: ScriptId,
		eval_id: RunId,
	): Promise<PaginationResponse<RunLogEntry>> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}/log",
				{
					params: { path: { channel_id, redex_id, eval_id } },
				},
			),
		);
		this.processLogs(eval_id, data.items);
		return data;
	}

	getLogsForRun(eval_id: string): RunLogEntry[] {
		const ids = this.logsByRun.get(eval_id);
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
