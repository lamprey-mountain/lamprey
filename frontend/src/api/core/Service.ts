import { Client } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { Accessor, createEffect, createResource, Resource } from "solid-js";
import type { RootStore } from "./Store";
import type { IDBPDatabase } from "idb";
import { logger } from "../../logger";

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

	/**
	 * Retry a failed HTTP request with exponential backoff and jitter.
	 * Automatically extracts data from the response and throws on error.
	 */
	protected async retryWithBackoff<T>(
		fn: () => Promise<{ data?: T; error?: any; response: Response }>,
		retries = 3,
		baseDelay = 1000,
	): Promise<T> {
		for (let i = 0; i < retries; i++) {
			let res;
			try {
				res = await fn();
			} catch (e: any) {
				// Don't retry on client errors (4xx except 429)
				if (
					e?.response?.status && e.response.status < 500 &&
					e.response.status !== 429
				) {
					throw e;
				}
				if (i === retries - 1) throw e;
				// Exponential backoff with jitter
				const delay = baseDelay * Math.pow(2, i) + Math.random() * 100;
				await new Promise((r) => setTimeout(r, delay));
				continue;
			}

			const { data, error } = res;
			if (!error) return data!;

			if (res.response.status < 500 && res.response.status !== 429) {
				throw error;
			}

			if (i === retries - 1) throw error;
			// Exponential backoff with jitter
			const delay = baseDelay * Math.pow(2, i) + Math.random() * 100;
			await new Promise((r) => setTimeout(r, delay));
		}
		throw new Error("too many errors");
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
		if (this.cache.has(id)) {
			return this.cache.get(id)!;
		}

		if (this.inflight.has(id)) {
			return this.inflight.get(id)!;
		}

		const promise = this.fetch(id).then((data) => {
			this.upsert(data);
			return data;
		}).finally(() => {
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

			// Attempt to load from IndexedDB first
			if (this.db && this.cacheName) {
				try {
					const cached = await this.db.get(this.cacheName, itemId);
					if (cached) {
						// Load stale data into cache immediately
						this.upsert(cached);
						// Fetch fresh data in background without awaiting
						this.fetchOrQueue(itemId).catch(() => {});
						return cached;
					}
				} catch (e) {
					// IndexedDB error, continue with API fetch
				}
			}

			// Use fetchOrQueue to handle loading/dedup logic
			return this.fetchOrQueue(itemId);
		});

		// Reactively update resource when cache changes.
		// This splits the "source of truth" to be the cache, overriding the resource's internal value logic
		// if the cache changes independently of the fetch.
		createEffect(() => {
			const itemId = id();
			if (!itemId) return;

			// Track the cache entry
			if (this.cache.has(itemId)) {
				const item = this.cache.get(itemId);
				// Mutate the resource to match the cache
				if (resource() !== item) {
					mutate(item as any);
				}
			}
		});

		return resource;
	}

	get(id: string): T | undefined {
		return this.cache.get(id);
	}

	upsert(item: T) {
		this.cache.set(this.getKey(item), item);
		// Save to IndexedDB
		if (this.db && this.cacheName) {
			this.db.put(this.cacheName, item).catch((e) => {
				logger.for("idb").warn(`Failed to write to ${this.cacheName}`, {
					key: this.getKey(item),
					error: e,
				});
			});
		}
	}

	delete(id: string) {
		this.cache.delete(id);
	}
}
