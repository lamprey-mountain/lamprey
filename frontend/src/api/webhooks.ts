import type { Pagination, Webhook } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Webhooks {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Webhook>();
	_requests = new Map<string, Promise<Webhook>>();
	_cachedListings = new Map<string, Listing<Webhook>>();

	fetch(webhook_id_signal: () => string): Resource<Webhook> {
		const [resource, { mutate }] = createResource(
			webhook_id_signal,
			(webhook_id) => {
				const cached = this.cache.get(webhook_id);
				if (cached) return cached;
				const existing = this._requests.get(webhook_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/webhook/{webhook_id}",
						{
							params: { path: { webhook_id } },
						},
					);
					if (error) throw error;
					this._requests.delete(webhook_id);
					this.cache.set(webhook_id, data);
					return data;
				})();

				this._requests.set(webhook_id, req);
				return req;
			},
		);

		createEffect(() => {
			const webhook = this.cache.get(webhook_id_signal());
			if (webhook) mutate(webhook);
		});

		return resource;
	}

	list(channel_id_signal: () => string): Resource<Pagination<Webhook>> {
		const paginate = async (pagination?: Pagination<Webhook>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/channel/{channel_id}/webhook",
				{
					params: {
						path: { channel_id: channel_id_signal() },
						query: {
							dir: "f",
							limit: 100,
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
			resource: (() => {}) as unknown as Resource<Pagination<Webhook>>,
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
}
