import { BaseService } from "../core/Service";
import type { RootStore } from "../core/Store";
import type { IDBPDatabase } from "idb";
import type { ApiDB } from "@/lib/sync/db";
import { ReactiveMap } from "@solid-primitives/map";
import type { Relationship } from "sdk";
import { batch } from "solid-js";

type UserId = string;

export class RelationshipsService extends BaseService<Relationship> {
	cache: ReactiveMap<UserId, Relationship> = new ReactiveMap();

	constructor(
		private store: RootStore,
		private getDb?: () => IDBPDatabase<ApiDB> | undefined,
	) {
		super(store, getDb);
	}

	clear() {
		this.cache.clear();
	}

	get(user_id: UserId) {
		return this.cache.get(user_id);
	}

	upsert() {
		throw new Error("Cannot upsert relationship without user id.");
	}

	delete(user_id: UserId) {
		this.cache.delete(user_id);
	}

	get relationships() {
		return [...this.cache.entries()];
	}

	async accept(target_id: UserId) {
		await this.store.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id } },
		});
	}

	async reject(target_id: UserId) {
		await this.store.client.http.DELETE(
			"/api/v1/user/@self/friend/{target_id}",
			{
				params: { path: { target_id } },
			},
		);
	}

	// NOTE: merge with accept()?
	async send(target_id: UserId) {
		await this.store.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id } },
		});
	}
}
