import { ReactiveMap } from "@solid-primitives/map";
import type { PaginationResponse, RunId, RunLogEntry, ScriptId } from "sdk";
import { batch } from "solid-js";
import { BaseService } from "../core/Service";

// FIXME: run_id and seq don't exist

export class ScriptLogsService extends BaseService<RunLogEntry> {
	protected cacheName = "script_log";

	getKey(item: RunLogEntry): string {
		return `${item.run_id}:${item.seq}`;
	}
	private logsByRun = new ReactiveMap<string, string[]>();

	protected override afterUpsert(log: RunLogEntry) {
		const runId = log.run_id;
		const logs = this.logsByRun.get(runId) ?? [];
		const key = this.getKey(log);
		if (!logs.includes(key)) {
			// Insert while maintaining sequence order
			// Usually we just append since logs arrive in order, but let's be safe
			const newLogs = [...logs, key].sort((a, b) => {
				const seqA = Number.parseInt(a.split(":")[1], 10);
				const seqB = Number.parseInt(b.split(":")[1], 10);
				return seqA - seqB;
			});
			this.logsByRun.set(runId, newLogs);
		}
	}

	protected override afterUpsertBulk(logs: RunLogEntry[]) {
		const byRun = new Map<string, string[]>();
		for (const log of logs) {
			const runId = log.run_id;
			let s = byRun.get(runId);
			if (!s) {
				s = [...(this.logsByRun.get(runId) ?? [])];
				byRun.set(runId, s);
			}
			const key = this.getKey(log);
			if (!s.includes(key)) {
				s.push(key);
			}
		}

		batch(() => {
			for (const [runId, ids] of byRun) {
				const sortedIds = ids.sort((a, b) => {
					const seqA = Number.parseInt(a.split(":")[1], 10);
					const seqB = Number.parseInt(b.split(":")[1], 10);
					return seqA - seqB;
				});
				this.logsByRun.set(runId, sortedIds);
			}
		});
	}

	getLogsForRun(run_id: string): RunLogEntry[] {
		const ids = this.logsByRun.get(run_id);
		if (!ids) return [];
		return ids
			.map((id) => this.cache.get(id))
			.filter((l): l is RunLogEntry => l != null);
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
		this.upsertBulk(data.items);
		return data;
	}

	override clear() {
		super.clear();
		this.logsByRun.clear();
	}
}
