import type { EmojiCustom, Pagination } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Emoji {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, EmojiCustom>();
	_requests = new Map<string, Promise<EmojiCustom>>();
	_cachedListings = new Map<string, Listing<EmojiCustom>>();

	fetch(
		room_id_signal: () => string,
		emoji_id_signal: () => string,
	): Resource<EmojiCustom> {
		const [resource, { mutate }] = createResource(
			() => [room_id_signal(), emoji_id_signal()],
			([room_id, emoji_id]) => {
				const cached = this.cache.get(emoji_id);
				if (cached) return cached;
				const existing = this._requests.get(emoji_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/room/{room_id}/emoji/{emoji_id}",
						{
							params: { path: { room_id, emoji_id } },
						},
					);
					if (error) throw error;
					this._requests.delete(emoji_id);
					this.cache.set(emoji_id, data);
					return data;
				})();

				this._requests.set(emoji_id, req);
				return req;
			},
		);

		createEffect(() => {
			const Emoji = this.cache.get(emoji_id_signal());
			if (Emoji) mutate(Emoji);
		});

		return resource;
	}

	list(room_id_signal: () => string): Resource<Pagination<EmojiCustom>> {
		const paginate = async (pagination?: Pagination<EmojiCustom>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/emoji",
				{
					params: {
						path: { room_id: room_id_signal() },
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
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = untrack(room_id_signal);
		const l = this._cachedListings.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<EmojiCustom>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(room_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			room_id_signal,
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
