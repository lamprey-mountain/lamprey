import type { Pagination, Role } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Roles {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Role>();
	_requests = new Map<string, Promise<Role>>();
	_cachedListings = new Map<string, Listing<Role>>();

	fetch(room_id: () => string, role_id: () => string): Resource<Role> {
		const query = () => ({
			room_id: room_id(),
			role_id: role_id(),
		});

		const [resource, { mutate }] = createResource(
			query,
			({ room_id, role_id }) => {
				const cached = this.cache.get(role_id);
				if (cached) return cached;
				const existing = this._requests.get(role_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/room/{room_id}/role/{role_id}",
						{
							params: { path: { room_id, role_id } },
						},
					);
					if (error) throw error;
					this._requests.delete(role_id);
					this.cache.set(role_id, data);
					return data;
				})();

				this._requests.set(role_id, req);
				return req;
			},
		);

		createEffect(() => {
			const invite = this.cache.get(role_id());
			if (invite) mutate(invite);
		});

		return resource;
	}

	list(room_id_sig: () => string): Resource<Pagination<Role>> {
		const paginate = async (pagination?: Pagination<Role>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/role",
				{
					params: {
						path: { room_id: room_id_sig() },
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

		const room_id = untrack(room_id_sig);
		const l = this._cachedListings.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<Role>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(room_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			room_id_sig,
			async (room_id) => {
				const l = this._cachedListings.get(room_id)!;
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

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		return resource;
	}
}
