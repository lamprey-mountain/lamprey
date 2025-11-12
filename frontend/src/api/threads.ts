import type { Channel, Pagination } from "sdk";
import { batch, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class Threads {
	api: Api = null as unknown as Api;
	_cachedRoomListings = new Map<string, Listing<Channel>>();
	_cachedRoomListingsArchived = new Map<string, Listing<Channel>>();
	_cachedRoomListingsRemoved = new Map<string, Listing<Channel>>();
	_cachedChannelListings = new Map<string, Listing<Channel>>();
	_cachedChannelListingsArchived = new Map<string, Listing<Channel>>();
	_cachedChannelListingsRemoved = new Map<string, Listing<Channel>>();

	private createLister(
		key: () => string,
		endpoint: any,
		cache: Map<string, Listing<Channel>>,
		keyName = "room_id",
	): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(endpoint, {
				params: {
					path: { [keyName]: key() },
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

			batch(() => {
				for (const item of data.items) {
					this.api.channels.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...(pagination?.items ?? []), ...data.items],
			};
		};

		const cacheKey = key();
		const l = cache.get(cacheKey);
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
		cache.set(cacheKey, l2);

		const [resource, { refetch, mutate }] = createResource(
			key,
			async (key) => {
				const l = cache.get(key)!;
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

	listForRoom(room_id: () => string): Resource<Pagination<Channel>> {
		return this.createLister(
			room_id,
			"/api/v1/room/{room_id}/thread",
			this._cachedRoomListings,
		);
	}

	listArchivedForRoom(room_id: () => string): Resource<Pagination<Channel>> {
		return this.createLister(
			room_id,
			"/api/v1/room/{room_id}/thread/archived",
			this._cachedRoomListingsArchived,
		);
	}

	listRemovedForRoom(room_id: () => string): Resource<Pagination<Channel>> {
		return this.createLister(
			room_id,
			"/api/v1/room/{room_id}/thread/removed",
			this._cachedRoomListingsRemoved,
		);
	}

	listForChannel(channel_id: () => string): Resource<Pagination<Channel>> {
		return this.createLister(
			channel_id,
			"/api/v1/channel/{channel_id}/thread",
			this._cachedChannelListings,
			"channel_id",
		);
	}

	listArchivedForChannel(
		channel_id: () => string,
	): Resource<Pagination<Channel>> {
		return this.createLister(
			channel_id,
			"/api/v1/channel/{channel_id}/thread/archived",
			this._cachedChannelListingsArchived,
			"channel_id",
		);
	}

	listRemovedForChannel(
		channel_id: () => string,
	): Resource<Pagination<Channel>> {
		return this.createLister(
			channel_id,
			"/api/v1/channel/{channel_id}/thread/removed",
			this._cachedChannelListingsRemoved,
			"channel_id",
		);
	}
}
