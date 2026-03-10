import { Client } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { Accessor, createEffect, createResource, Resource } from "solid-js";
import type { RootStore } from "./Store";

export abstract class BaseService<T> {
	protected client: Client;
	protected store: RootStore;
	cache = new ReactiveMap<string, T>();
	protected inflight = new Map<string, Promise<T>>();

	constructor(store: RootStore) {
		this.store = store;
		this.client = store.client;
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
	 * 1. Fetches the item if missing (handling loading state).
	 * 2. Reactively updates when the cache updates (handling socket events).
	 */
	use(id: Accessor<string | undefined>): Resource<T | undefined> {
		const [resource, { mutate }] = createResource(id, async (itemId) => {
			if (!itemId) return undefined;
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
	}

	delete(id: string) {
		this.cache.delete(id);
	}
}
