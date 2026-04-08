import { ReactiveMap } from "@solid-primitives/map";
import type { IDBPDatabase } from "idb";
import type { Client } from "sdk";
import {
	type Accessor,
	batch,
	createEffect,
	createResource,
	type Resource,
} from "solid-js";
import { logger } from "../../logger";
import type { RootStore } from "./Store";

export type Item<T> =
	| { status: "loading" } // the item is currently being loaded
	| { status: "ready"; data: T } // up to date and currently in sync
	| { status: "stale"; data: T } // loaded from cache (eg. http cache, indexeddb. 304 not modified is `ready` iff the client is connected to sync)
	| { status: "error"; error: unknown }; // an error occurred while fetching this resource

export abstract class BaseService<T> {
	protected client: Client;
	protected store: RootStore;
	cache = new ReactiveMap<string, T>();
	protected inflight = new Map<string, Promise<T>>();
	protected getDb?: () => IDBPDatabase<unknown> | undefined;
	protected abstract cacheName: string;

	constructor(
		store: RootStore,
		getDb?: () => IDBPDatabase<unknown> | undefined,
	) {
		this.store = store;
		this.client = store.client;
		this.getDb = getDb;
	}

	/**
	 * Get the current database instance (may be undefined during initialization)
	 */
	protected get db(): IDBPDatabase<unknown> | undefined {
		return this.getDb?.();
	}

	protected getDbKey(id: string): IDBValidKey {
		return id;
	}

	/**
	 * Retry a failed HTTP request with exponential backoff and jitter.
	 * Automatically extracts data from the response and throws on error.
	 * Respects the Retry-After header for 429 responses.
	 */
	protected async retryWithBackoff<T>(
		fn: () => Promise<{ data?: T; error?: unknown; response: Response }>,
		retries = 3,
		baseDelay = 1000,
	): Promise<T> {
		for (let i = 0; i < retries; i++) {
			let res: { data?: T; error?: unknown; response: Response } | undefined;
			try {
				res = await fn();
			} catch (e: unknown) {
				// Don't retry on client errors (4xx except 429)
				const error = e as {
					response?: { status?: number; headers?: Headers };
				};
				if (
					error?.response?.status &&
					error.response.status < 500 &&
					error.response.status !== 429
				) {
					throw e;
				}
				if (i === retries - 1) throw e;
				// Use Retry-After header if available, otherwise exponential backoff with jitter
				const delay = this.getRetryDelay(error.response, i, baseDelay);
				await new Promise((r) => setTimeout(r, delay));
				continue;
			}

			const { data, error } = res;
			if (!error) return data as T;

			if (res.response.status < 500 && res.response.status !== 429) {
				throw error;
			}

			if (i === retries - 1) throw error;
			// Use Retry-After header if available, otherwise exponential backoff with jitter
			const delay = this.getRetryDelay(res.response, i, baseDelay);
			await new Promise((r) => setTimeout(r, delay));
		}
		throw new Error("too many errors");
	}

	/**
	 * Calculate retry delay based on Retry-After header or exponential backoff with jitter.
	 */
	private getRetryDelay(
		response: { headers?: Headers } | undefined,
		retryAttempt: number,
		baseDelay: number,
	): number {
		const retryAfter = response?.headers?.get("Retry-After");
		if (retryAfter) {
			// Retry-After can be either:
			// 1. A delay in seconds (e.g., "5")
			// 2. An HTTP date timestamp (e.g., "Wed, 21 Oct 2015 07:28:00 GMT")
			const delayInSeconds = Number.parseInt(retryAfter, 10);
			if (!Number.isNaN(delayInSeconds)) {
				return delayInSeconds * 1000;
			}

			// Try parsing as a date
			const retryDate = new Date(retryAfter);
			if (!Number.isNaN(retryDate.getTime())) {
				const delay = retryDate.getTime() - Date.now();
				return Math.max(0, delay);
			}
		}

		// Fallback to exponential backoff with jitter
		return baseDelay * 2 ** retryAttempt + Math.random() * 100;
	}

	/**
	 * Defines how to derive the unique cache key from an item.
	 */
	abstract getKey(item: T): string;

	/**
	 * The raw fetch implementation.
	 */
	abstract fetch(id: string): Promise<T>;

	/**
	 * Fetches the item if not in cache, deduplicating requests.
	 * Updates the cache with the result.
	 */
	async fetchOrQueue(id: string): Promise<T> {
		const cached = this.cache.get(id);
		if (cached !== undefined) {
			return cached;
		}

		const inflightPromise = this.inflight.get(id);
		if (inflightPromise !== undefined) {
			return inflightPromise;
		}

		const promise = this.fetch(id)
			.then((data) => {
				this.upsert(data);
				return data;
			})
			.finally(() => {
				this.inflight.delete(id);
			});

		this.inflight.set(id, promise);
		return promise;
	}

	/**
	 * Returns a resource that:
	 * 1. Attempts to load from IndexedDB (stale data).
	 * 2. Fetches the item from API if missing (handling loading state).
	 * 3. Reactively updates when the cache updates (handling socket events).
	 */
	use(id: Accessor<string | undefined>): Resource<T | undefined> {
		const [resource, { mutate }] = createResource(id, async (itemId) => {
			if (!itemId) return undefined;

			const cached = this.cache.get(itemId);
			if (cached) return cached;

			if (this.db && this.cacheName) {
				try {
					const cached = await this.db.get(
						this.cacheName,
						this.getDbKey(itemId),
					);
					if (cached) {
						this.upsert(cached);
						this.fetchOrQueue(itemId).catch(() => {});
						return cached;
					}
				} catch (_e) {
					// IndexedDB error, continue with API fetch
				}
			}

			return this.fetchOrQueue(itemId);
		});

		createEffect(() => {
			const itemId = id();
			if (!itemId) return;

			const item = this.cache.get(itemId);
			if (item !== undefined && resource() !== item) {
				mutate(() => item);
			}
		});

		return resource;
	}

	get(id: string): T | undefined {
		return this.cache.get(id);
	}

	/**
	 * Prepare an item for upsert (e.g. normalization).
	 */
	protected prepareUpsert(item: T): T {
		return item;
	}

	/**
	 * Hook called after an item is upserted.
	 */
	protected afterUpsert(_item: T): void {}

	/**
	 * Hook called after a bulk upsert of items.
	 */
	protected afterUpsertBulk(items: T[]): void {
		for (const item of items) {
			this.afterUpsert(item);
		}
	}

	/**
	 * Hook called after an item is deleted.
	 */
	protected afterDelete(_id: string, _item?: T): void {}

	upsert(item: T) {
		const prepared = this.prepareUpsert(item);
		this.cache.set(this.getKey(prepared), prepared);
		this.afterUpsert(prepared);

		if (this.db && this.cacheName) {
			this.db.put(this.cacheName, prepared).catch((e) => {
				logger.for("idb").warn(`Failed to write to ${this.cacheName}`, {
					key: this.getKey(prepared),
					error: e,
				});
			});
		}
	}

	upsertBulk(items: T[]) {
		if (items.length === 0) return;

		const preparedItems = items.map((item) => this.prepareUpsert(item));

		// update in memory cache
		batch(() => {
			for (const item of preparedItems) {
				this.cache.set(this.getKey(item), item);
			}
			this.afterUpsertBulk(preparedItems);
		});

		// update indexeddb
		const db = this.db;
		const storeName = this.cacheName;

		if (db && storeName) {
			// run in background
			(async () => {
				try {
					const tx = db.transaction(storeName, "readwrite");
					const store = tx.objectStore(storeName);

					for (const item of preparedItems) {
						store.put(item);
					}

					await tx.done;
				} catch (e) {
					logger.for("idb").error(`Failed to bulk write to ${storeName}`, {
						count: preparedItems.length,
						error: e,
					});
				}
			})();
		}
	}

	delete(id: string) {
		const item = this.cache.get(id);
		this.cache.delete(id);
		this.afterDelete(id, item);
	}
}
