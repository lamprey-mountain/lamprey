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
	 * For simple entities, this is usually just `item.id`.
	 * For compound entities (like members), this might be `${room_id}:${user_id}`.
	 */
	abstract getKey(item: T): string;

	abstract fetch(id: string): Promise<T>;

	/**
	 * Returns a resource that fetches the item if not in cache,
	 * and updates when the cache updates.
	 */
	use(id: Accessor<string | undefined>): Resource<T | undefined> {
		const [resource, { mutate }] = createResource(id, async (itemId) => {
			if (!itemId) return undefined;

			// Check cache first - if present, return it immediately to avoid loading state
			// However, createResource is async.
			// Ideally we want to read from cache synchronously if possible.
			const cached = this.cache.get(itemId);
			if (cached) return cached;

			// Check inflight
			const existing = this.inflight.get(itemId);
			if (existing) return existing;

			// Fetch
			try {
				const req = this.fetch(itemId);
				this.inflight.set(itemId, req);
				const data = await req;
				this.inflight.delete(itemId);
				this.upsert(data);
				return data;
			} catch (e) {
				this.inflight.delete(itemId);
				throw e;
			}
		});

		// Reactively update resource when cache changes
		createEffect(() => {
			const itemId = id();
			if (!itemId) return;
			// Tracking the specific item in the map
			if (this.cache.has(itemId)) {
				const item = this.cache.get(itemId);
				// Only mutate if different to avoid infinite loops if strict equality fails
				// But ReactiveMap returns the same object reference usually.
				if (resource() !== item) {
					mutate(item);
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
