import type { ApiError, api } from "sdk";
import { createSignal } from "solid-js";
import { BaseService } from "../core/Service";

export type Harvest = api["Harvest"];
export type HarvestCreateUser = api["HarvestCreateUser"];

const KEY = "@self";

function isApiError(err: unknown): err is ApiError {
	return typeof err === "object" && err !== null && "code" in err;
}

export class HarvestService extends BaseService<Harvest> {
	protected cacheName = "harvest";
	private _loading = createSignal(false);
	private _error = createSignal<unknown>(null);
	readonly loading = this._loading[0];
	readonly error = this._error[0];

	getKey(_item: Harvest): string {
		return KEY;
	}

	async fetch(_id: string): Promise<Harvest> {
		return await this.retryWithBackoff<Harvest>(() =>
			this.client.http.GET("/api/v1/user/{user_id}/harvest", {
				params: { path: { user_id: KEY } },
			}),
		);
	}

	override upsert(item: Harvest) {
		this.cache.set(KEY, item);
		this._error[1](null);
	}

	current(): Harvest | undefined {
		return this.cache.get(KEY);
	}

	async load(): Promise<void> {
		const setLoading = this._loading[1];
		const setError = this._error[1];
		setLoading(true);
		try {
			this.upsert(await this.fetch(KEY));
			setError(null);
		} catch (err) {
			if (isApiError(err) && err.code === "UnknownHarvest") {
				this.cache.delete(KEY);
			} else {
				setError(err);
			}
		} finally {
			setLoading(false);
		}
	}

	async create(options: HarvestCreateUser): Promise<void> {
		await this.retryWithBackoff<void>(() =>
			this.client.http.POST("/api/v1/user/{user_id}/harvest", {
				params: { path: { user_id: KEY } },
				body: options,
			}),
		);
		await this.load();
	}
}
