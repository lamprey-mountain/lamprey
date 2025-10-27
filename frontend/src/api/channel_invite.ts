import type { Invite, Pagination } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class ChannelInvites {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Invite>();
	_requests = new Map<string, Promise<Invite>>();
	_cachedListings = new Map<string, Listing<Invite>>();

	fetch(invite_code_signal: () => string): Resource<Invite> {
		const [resource, { mutate }] = createResource(
			invite_code_signal,
			(invite_code) => {
				const cached = this.cache.get(invite_code);
				if (cached) return cached;
				const existing = this._requests.get(invite_code);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/invite/{invite_code}",
						{
							params: { path: { invite_code } },
						},
					);
					if (error) throw error;
					this._requests.delete(invite_code);
					this.cache.set(invite_code, data);
					return data;
				})();

				this._requests.set(invite_code, req);
				return req;
			},
		);

		createEffect(() => {
			const invite = this.cache.get(invite_code_signal());
			if (invite) mutate(invite);
		});

		return resource;
	}

	list(channel_id_signal: () => string): Resource<Pagination<Invite>> {
		const paginate = async (pagination?: Pagination<Invite>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/channel/{channel_id}/invite",
				{
					params: {
						path: { channel_id: channel_id_signal() },
						query: {
							dir: "f",
							limit: 100,
							from: pagination?.items.at(-1)?.code,
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
					this.cache.set(item.code, item);
				}
			});

			return {
				...data,
				items: [...(pagination?.items ?? []), ...data.items],
			};
		};

		const channel_id = untrack(channel_id_signal);
		const l = this._cachedListings.get(channel_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<Invite>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(channel_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			channel_id_signal,
			async (channel_id) => {
				const l = this._cachedListings.get(channel_id)!;
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

	async use(invite_code: string) {
		const { error } = await this.api.client.http.POST(
			"/api/v1/invite/{invite_code}",
			{
				params: {
					path: { invite_code },
				},
			},
		);
		if (error) throw error;
	}
}
