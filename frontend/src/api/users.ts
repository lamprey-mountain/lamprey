import type { Pagination, User, UserWithRelationship } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Users {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, UserWithRelationship>();
	requests = new Map<string, Promise<UserWithRelationship>>();
	_cachedListing: Listing<User> | null = null;

	fetch(user_id: () => string): Resource<UserWithRelationship> {
		const [resource, { mutate }] = createResource(user_id, (user_id) => {
			const cached = this.cache.get(user_id);
			if (cached) return cached;
			const existing = this.requests.get(user_id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/user/{user_id}",
					{
						params: { path: { user_id } },
					},
				);
				if (error) throw error;
				this.requests.delete(user_id);
				this.cache.set(user_id, data);
				return data;
			})();

			this.requests.set(user_id, req);
			return req;
		});

		createEffect(() => {
			const id = user_id();
			if (!id) return;
			const user = this.cache.get(id);
			if (user) {
				mutate(user);
			}
		});

		return resource;
	}

	list(): Resource<Pagination<User>> {
		const paginate = async (pagination?: Pagination<User>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET("/api/v1/user", {
				params: {
					query: {
						dir: "f",
						limit: 100,
						from: pagination?.items.at(-1)?.id,
					},
				},
			});

			if (error) {
				console.error(error);
				throw error;
			}

			// User list doesn't contain relationship data, so we don't cache it here
			// to avoid partial data in the main user cache.

			return {
				...data,
				items: [...(pagination?.items ?? []), ...data.items],
			};
		};

		const l = this._cachedListing;
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		this._cachedListing = {
			resource: (() => {}) as unknown as Resource<Pagination<User>>,
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
