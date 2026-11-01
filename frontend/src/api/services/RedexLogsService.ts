import { ReactiveMap } from "@solid-primitives/map";
import type { EvalId, EvalLogEntry, PaginationResponse, RedexId } from "sdk";
import { BaseService } from "../core/Service";

export class RedexLogsService extends BaseService<EvalLogEntry> {
	protected cacheName = "script_log";

	getKey(item: EvalLogEntry): string {
		return item.id.toString();
	}

	private logsByRun = new ReactiveMap<string, string[]>();
	private _listed = new Set<string>();
	private _listPromises = new Map<string, Promise<void>>();

	// add runId to logs since backend doesnt include it
	private processLogs(runId: string, items: EvalLogEntry[]) {
		const keys = items.map((item) => this.getKey(item));

		const existing = this.logsByRun.get(runId) ?? [];
		const merged = Array.from(new Set([...existing, ...keys])).sort((a, b) => {
			return Number(a) - Number(b);
		});

		this.logsByRun.set(runId, merged);
		this.upsertBulk(items);
	}

	subscribe(channel_id: string, redex_id: RedexId) {
		this.client.send({
			type: "ScriptSubscribe",
			channel_id,
			script_id: redex_id,
		});
	}

	async fetch(_id: string): Promise<EvalLogEntry> {
		throw new Error("Use list() to fetch logs");
	}

	async list(
		channel_id: string,
		redex_id: RedexId,
		eval_id: EvalId,
	): Promise<PaginationResponse<EvalLogEntry>> {
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

	async ensureListed(
		channel_id: string,
		redex_id: RedexId,
		eval_id: EvalId,
	): Promise<void> {
		const key = `${channel_id}:${redex_id}:${eval_id}`;
		if (this._listed.has(key)) return;

		const pending = this._listPromises.get(key);
		if (pending) return pending;

		const promise = this.list(channel_id, redex_id, eval_id)
			.then(() => {
				this._listed.add(key);
			})
			.finally(() => {
				this._listPromises.delete(key);
			});
		this._listPromises.set(key, promise);
		return promise;
	}

	getLogsForRun(eval_id: string): EvalLogEntry[] {
		const ids = this.logsByRun.get(eval_id);
		if (!ids) return [];
		return ids
			.map((id) => this.cache.get(id))
			.filter((l): l is EvalLogEntry => l != null);
	}

	override clear() {
		super.clear();
		this.logsByRun.clear();
		this._listed.clear();
		this._listPromises.clear();
	}
}
