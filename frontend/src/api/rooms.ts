import type { Pagination, Room } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { batch, createEffect, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "@/api";
import { fetchWithRetry } from "./util.ts";
import { RoomsService } from "./services/RoomsService.ts";
import { logger } from "../logger.ts";

const log = logger.for("api/rooms");

export class Rooms {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Room>();
	_requests = new Map<string, Promise<Room>>();
	_cachedListing: Listing<Room> = {
		resource: (() => {}) as unknown as Resource<Pagination<Room>>,
		refetch: () => {},
		mutate: (value) => {
			this._cachedListing.pagination = value;
		},
		prom: null,
		pagination: null,
	};
	_cachedListingAll: Listing<Room> = {
		resource: (() => {}) as unknown as Resource<Pagination<Room>>,
		refetch: () => {},
		mutate: (value) => {
			this._cachedListingAll.pagination = value;
		},
		prom: null,
		pagination: null,
	};

	// Internal service instance for new code
	private _service: RoomsService | null = null;

	// Get or create the service instance
	private getService(): RoomsService {
		if (!this._service) {
			this._service = new RoomsService(this.api.store);
		}
		return this._service;
	}

	fetch(room_id_sig: () => string): Resource<Room> {
		const [resource, { mutate }] = createResource(room_id_sig, (room_id) => {
			const cached = this.cache.get(room_id);
			if (cached) return cached;
			const existing = this._requests.get(room_id);
			if (existing) return existing;

			const req = (async () => {
				const data = await fetchWithRetry(() =>
					this.api.client.http.GET(
						"/api/v1/room/{room_id}",
						{
							params: { path: { room_id } },
						},
					)
				);
				this._requests.delete(room_id);
				this.cache.set(room_id, data);
				return data;
			})();

			this._requests.set(room_id, req);
			return req;
		});

		createEffect(() => {
			const room_id = room_id_sig();
			const cached = this.cache.get(room_id);
			if (cached) mutate(cached);
		});

		return resource;
	}

	list(): Resource<Pagination<Room>> {
		const l = this._cachedListing;
		if ((l.resource as any).upgraded) {
			return l.resource;
		}

		const paginate = async (pagination?: Pagination<Room>) => {
			if (pagination && !pagination.has_more) return pagination;

			try {
				const data = await fetchWithRetry(() =>
					this.api.client.http.GET(
						"/api/v1/user/{user_id}/room",
						{
							params: {
								path: { user_id: "@self" },
								query: {
									dir: "f",
									limit: 100,
									from: pagination?.items.at(-1)?.id,
								},
							},
						},
					)
				);

				batch(() => {
					for (const item of data.items) {
						this.cache.set(item.id, item);
					}
				});

				return {
					...data,
					items: [...(pagination?.items ?? []), ...data.items],
				};
			} catch (error) {
				log.error(error);
				throw error;
			}
		};

		const [resource, { refetch, mutate }] = createResource(
			() => [this.api.session(), this.api.preferencesLoaded()] as const,
			async ([session, loaded]) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
				const l = this._cachedListing;
				if (l.pagination) return l.pagination;
				if (!loaded) return { items: [], total: 0, has_more: false };

				if (l.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l.pagination = res;
				l.prom = null;
				return res;
			},
		);

		(resource as any).upgraded = true;
		this._cachedListing.resource = resource;
		this._cachedListing.refetch = refetch;
		this._cachedListing.mutate = (value: Pagination<Room>) => {
			this._cachedListing.pagination = value;
			mutate(value);
		};

		return resource;
	}

	async create(body: { name: string; public?: boolean | null }): Promise<Room> {
		const service = this.getService();
		return await service.create(body);
	}

	async update(room_id: string, body: any): Promise<Room> {
		const service = this.getService();
		return await service.update(room_id, body);
	}

	list_all(): Resource<Pagination<Room>> {
		const l = this._cachedListingAll;
		if ((l.resource as any).upgraded) {
			return l.resource;
		}

		const paginate = async (pagination?: Pagination<Room>) => {
			if (pagination && !pagination.has_more) return pagination;

			try {
				const data = await fetchWithRetry(() =>
					this.api.client.http.GET("/api/v1/room", {
						params: {
							query: {
								dir: "f",
								limit: 100,
								from: pagination?.items.at(-1)?.id,
							},
						},
					})
				);

				batch(() => {
					for (const item of data.items) {
						this.cache.set(item.id, item);
					}
				});

				return {
					...data,
					items: [...(pagination?.items ?? []), ...data.items],
				};
			} catch (error) {
				log.error(error);
				throw error;
			}
		};

		const [resource, { refetch, mutate }] = createResource(
			this.api.session,
			async (session) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
				const l = this._cachedListingAll;
				if (l.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l.pagination = res;
				l.prom = null;
				return res;
			},
		);

		(resource as any).upgraded = true;
		this._cachedListingAll.resource = resource;
		this._cachedListingAll.refetch = refetch;
		this._cachedListingAll.mutate = (value: Pagination<Room>) => {
			this._cachedListingAll.pagination = value;
			mutate(value);
		};

		return resource;
	}

	async markRead(room_id: string) {
		let has_more = true;
		let from: string | undefined = undefined;
		while (has_more) {
			let data;
			try {
				data = await fetchWithRetry(() =>
					this.api.client.http.GET(
						"/api/v1/room/{room_id}/channel",
						{
							params: {
								path: { room_id },
								query: {
									dir: "f",
									limit: 100,
									from,
								},
							},
						},
					)
				);
			} catch (error) {
				log.error("Failed to fetch threads for room", error);
				break;
			}

			for (const thread of data.items) {
				if (thread.last_version_id) {
					await this.store.channels.ack(
						thread.id,
						undefined,
						thread.last_version_id,
					);
				}
			}
			has_more = data.has_more;
			from = data.items.at(-1)?.id;
		}
	}
}

export class Rooms2 {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Room>();
	private _requests = new Map<string, Promise<Room>>();

	/** fetch a single room */
	async fetch(room_id: string): Promise<Room> {
		const existing = this._requests.get(room_id);
		if (existing) return existing;

		const req = (async () => {
			const data = await fetchWithRetry(() =>
				this.api.client.http.GET(
					"/api/v1/room/{room_id}",
					{
						params: { path: { room_id } },
					},
				)
			);
			this._requests.delete(room_id);
			this.cache.set(room_id, data);
			return data;
		})();

		this._requests.set(room_id, req);
		return req;
	}

	/** reactively fetch a single room as a resource */
	fetchResource(room_id: () => string): Resource<Room> {
		const [resource] = createResource(room_id, async (room_id) => {
			const cached = this.cache.get(room_id);
			if (cached) return cached;
			await this.fetch(room_id);
			return this.cache.get(room_id)!;
		});

		return resource;
	}
}
