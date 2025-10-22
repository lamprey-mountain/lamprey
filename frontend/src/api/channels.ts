import type { Channel, Pagination } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { batch, createEffect, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Channels {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Channel>();
	_requests = new Map<string, Promise<Channel>>();
	_cachedListings = new Map<string, Listing<Channel>>();
	_listingMutators = new Set<
		{ room_id: string; mutate: (value: Pagination<Channel>) => void }
	>();
	_cachedListingsArchived = new Map<string, Listing<Channel>>();
	_listingMutatorsArchived = new Set<
		{ room_id: string; mutate: (value: Pagination<Channel>) => void }
	>();
	_cachedListingsRemoved = new Map<string, Listing<Channel>>();
	_listingMutatorsRemoved = new Set<
		{ room_id: string; mutate: (value: Pagination<Channel>) => void }
	>();

	fetch(channel_id: () => string): Resource<Channel> {
		const [resource, { mutate }] = createResource(channel_id, (channel_id) => {
			const cached = this.cache.get(channel_id);
			if (cached) return cached;
			const existing = this._requests.get(channel_id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/channel/{channel_id}",
					{
						params: { path: { channel_id: channel_id } },
					},
				);
				if (error) throw error;
				this._requests.delete(channel_id);
				this.cache.set(channel_id, data);
				return data;
			})();

			createEffect(() => {
				mutate(this.cache.get(channel_id));
			});

			this._requests.set(channel_id, req);
			return req;
		});

		return resource;
	}

	list(room_id_signal: () => string): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/channel",
				{
					params: {
						path: { room_id: room_id_signal() },
						query: {
							dir: "f",
							limit: 1024,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = room_id_signal();
		const l = this._cachedListings.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2: Listing<Channel> = {
			resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(room_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			room_id_signal,
			async (room_id) => {
				let l = this._cachedListings.get(room_id)!;
				if (!l) {
					l = {
						resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
						refetch: () => {},
						mutate: () => {},
						prom: null,
						pagination: null,
					};
					this._cachedListings.set(room_id, l);
				}

				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._listingMutators) {
					if (mut.room_id === room_id) mut.mutate(res);
				}

				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutators.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}

	async typing(channel_id: string) {
		await this.api.client.http.POST("/api/v1/channel/{channel_id}/typing", {
			params: {
				path: { channel_id },
			},
		});
	}

	async create(
		room_id: string,
		body: { name: string; parent_id?: string },
	): Promise<Channel> {
		const { data, error } = await this.api.client.http.POST(
			"/api/v1/room/{room_id}/channel",
			{
				params: { path: { room_id } },
				body: body,
			},
		);
		if (error) throw error;
		return data;
	}

	listArchived(room_id_signal: () => string): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/channel/archived",
				{
					params: {
						path: { room_id: room_id_signal() },
						query: {
							dir: "f",
							limit: 1024,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = room_id_signal();
		const l = this._cachedListingsArchived.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2: Listing<Channel> = {
			resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListingsArchived.set(room_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			room_id_signal,
			async (room_id) => {
				let l = this._cachedListingsArchived.get(room_id)!;
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._listingMutatorsArchived) {
					if (mut.room_id === room_id) mut.mutate(res);
				}

				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutatorsArchived.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}

	listRemoved(room_id_signal: () => string): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/channel/removed",
				{
					params: {
						path: { room_id: room_id_signal() },
						query: {
							dir: "f",
							limit: 1024,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = room_id_signal();
		const l = this._cachedListingsRemoved.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2: Listing<Channel> = {
			resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListingsRemoved.set(room_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			room_id_signal,
			async (room_id) => {
				let l = this._cachedListingsRemoved.get(room_id)!;
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._listingMutatorsRemoved) {
					if (mut.room_id === room_id) mut.mutate(res);
				}

				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutatorsRemoved.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}

	async ack(
		channel_id: string,
		message_id: string | undefined,
		version_id: string,
	) {
		await this.api.client.http.PUT("/api/v1/channel/{channel_id}/ack", {
			params: { path: { channel_id: channel_id } },
			body: { message_id, version_id },
		});
		const t = this.cache.get(channel_id);
		if (t) {
			this.cache.set(channel_id, {
				...t,
				last_read_id: version_id,
				is_unread: version_id < (t.last_version_id ?? ""),
			});
		}
	}

	async lock(channel_id: string) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: channel_id } },
			body: { locked: true },
		});
	}

	async unlock(channel_id: string) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: channel_id } },
			body: { locked: false },
		});
	}

	async archive(channel_id: string) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: channel_id } },
			body: { archived: true },
		});
	}

	async unarchive(channel_id: string) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: channel_id } },
			body: { archived: false },
		});
	}
}
