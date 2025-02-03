import { Pagination, Room } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { batch, createEffect, createResource, Resource } from "solid-js";
import { Api, Listing } from "../api.tsx";

export class Rooms {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Room>();
	_requests = new Map<string, Promise<Room>>();
	_cachedListing: Listing<Room> | null = null;

	fetch(room_id: () => string): Resource<Room> {
		const [resource, { mutate }] = createResource(room_id, (room_id) => {
			const cached = this.cache.get(room_id);
			if (cached) return cached;
			const existing = this._requests.get(room_id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/room/{room_id}",
					{
						params: { path: { room_id } },
					},
				);
				if (error) throw error;
				this._requests.delete(room_id);
				this.cache.set(room_id, data);
				return data;
			})();

			createEffect(() => {
				mutate(this.cache.get(room_id));
			});

			this._requests.set(room_id, req);
			return req;
		});

		return resource;
	}

	list(): Resource<Pagination<Room>> {
		const paginate = async (pagination?: Pagination<Room>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET("/api/v1/room", {
				params: {
					query: {
						dir: "f",
						limit: 100,
						from: pagination?.items.at(-1)?.id,
					},
				},
			});

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

		const l = this._cachedListing;
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		this._cachedListing = {
			resource: (() => {}) as unknown as Resource<Pagination<Room>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};

		const [resource, { refetch, mutate }] = createResource(async () => {
			const l = this._cachedListing!;
			if (l?.prom) {
				await l.prom;
				return l.pagination!;
			}

			const prom = l.pagination ? paginate(l.pagination) : paginate();
			l.prom = prom;
			const res = await prom;
			l!.pagination = res;
			l!.prom = null;
			return res!;
		});

		this._cachedListing.resource = resource;
		this._cachedListing.refetch = refetch;
		this._cachedListing.mutate = mutate;

		return resource;
	}
}
