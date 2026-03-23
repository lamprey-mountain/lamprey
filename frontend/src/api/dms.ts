import type { Channel, Pagination } from "sdk";
import { batch, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "../api.tsx";
import { logger } from "../logger.ts";

const log = logger.for("api/dms");

export class Dms {
	api: Api = null as unknown as Api;
	_cachedListing: Listing<Channel> | null = null;

	list(): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/user/{user_id}/dm",
				{
					params: {
						path: { user_id: "@self" },
						query: {
							dir: "b", // newest first
							limit: 100,
							from: pagination?.items.at(-1)?.last_version_id ?? undefined,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				log.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.store.channels.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const l = this._cachedListing;
		if (l) {
			return l.resource;
		}

		this._cachedListing = {
			resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};

		const [resource, { refetch, mutate }] = createResource(
			this.api.session,
			async (session) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
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
			},
		);

		this._cachedListing.resource = resource;
		this._cachedListing.refetch = refetch;
		this._cachedListing.mutate = mutate;

		return resource;
	}
}
