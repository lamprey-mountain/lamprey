import type {
	Channel,
	ChannelPatch,
	ChannelType,
	Pagination,
	Tag,
	TagCreate,
} from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { batch, createEffect, createResource, type Resource } from "solid-js";
import type { Api, Listing } from "../api.tsx";
import { fetchWithRetry } from "./util.ts";
import { ChannelsService } from "./services/ChannelsService.ts";

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

	// Internal service instance for new code
	private _service: ChannelsService | null = null;

	// Get or create the service instance
	private getService(): ChannelsService {
		if (!this._service) {
			this._service = new ChannelsService(this.api.store);
		}
		return this._service;
	}

	_getOrCreateListing(
		map: Map<string, Listing<Channel>>,
		room_id: string,
	): Listing<Channel> {
		let l = map.get(room_id);
		if (!l) {
			l = {
				resource: (() => {}) as unknown as Resource<Pagination<Channel>>,
				refetch: () => {},
				mutate: (value) => {
					l!.pagination = value;
				},
				prom: null,
				pagination: null,
			};
			map.set(room_id, l);
		}
		return l;
	}

	normalize(channel: Channel): Channel {
		if (!channel.permission_overwrites) channel.permission_overwrites = [];
		if (!channel.recipients) channel.recipients = [];
		if (
			!channel.tags &&
			(channel.type === "ThreadPublic" || channel.type === "ThreadPrivate" ||
				channel.type === "ThreadForum2")
		) {
			channel.tags = [];
		}
		return channel;
	}

	fetch(channel_id_sig: () => string): Resource<Channel> {
		const [resource, { mutate }] = createResource(
			channel_id_sig,
			(channel_id) => {
				const cached = this.cache.get(channel_id);
				if (cached) return cached;
				const existing = this._requests.get(channel_id);
				if (existing) return existing;

				const req = (async () => {
					const data = await fetchWithRetry(() =>
						this.api.client.http.GET(
							"/api/v1/channel/{channel_id}",
							{
								params: { path: { channel_id: channel_id } },
							},
						)
					);
					this._requests.delete(channel_id);
					this.normalize(data);
					this.cache.set(channel_id, data);
					return data;
				})();

				this._requests.set(channel_id, req);
				return req;
			},
		);

		createEffect(() => {
			const channel_id = channel_id_sig();
			const cached = this.cache.get(channel_id);
			if (cached) mutate(cached);
		});

		return resource;
	}

	list(room_id_signal: () => string): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const data = await fetchWithRetry(() =>
				this.api.client.http.GET(
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
				)
			);

			batch(() => {
				for (const item of data.items) {
					this.normalize(item);
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = room_id_signal();
		const l = this._getOrCreateListing(this._cachedListings, room_id);
		if ((l.resource as any).upgraded) {
			return l.resource;
		}

		const [resource, { mutate, refetch }] = createResource(
			() => [room_id_signal(), this.api.session()] as const,
			async ([room_id, session]) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
				const l = this._getOrCreateListing(this._cachedListings, room_id);

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

		(resource as any).upgraded = true;
		l.resource = resource;
		l.refetch = refetch;
		l.mutate = (value: Pagination<Channel>) => {
			l.pagination = value;
			mutate(value);
		};

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutators.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}

	async typing(channel_id: string) {
		await fetchWithRetry(() =>
			this.api.client.http.POST("/api/v1/channel/{channel_id}/typing", {
				params: {
					path: { channel_id },
				},
			})
		);
	}

	async create(
		room_id: string,
		body: { name: string; parent_id?: string },
	): Promise<Channel> {
		const service = this.getService();
		return await service.create(room_id, body);
	}

	async createThreadFromMessage(
		channel_id: string,
		message_id: string,
		body: { name: string; type?: ChannelType },
	): Promise<Channel> {
		return await fetchWithRetry(() =>
			this.api.client.http.POST(
				"/api/v1/channel/{channel_id}/message/{message_id}/thread",
				{
					params: { path: { channel_id, message_id } },
					body: body as any,
				},
			)
		);
	}

	async update(
		channel_id: string,
		body: ChannelPatch,
	): Promise<Channel> {
		const service = this.getService();
		return await service.update(channel_id, body);
	}

	listArchived(room_id_signal: () => string): Resource<Pagination<Channel>> {
		const paginate = async (pagination?: Pagination<Channel>) => {
			if (pagination && !pagination.has_more) return pagination;

			const data = await fetchWithRetry(() =>
				this.api.client.http.POST(
					"/api/v1/search/channels",
					{
						body: {
							room_id: [room_id_signal()],
							archived: true,
							limit: 100,
							offset: pagination?.items.length ?? 0,
						},
					},
				)
			);

			batch(() => {
				for (const item of data.items) {
					this.normalize(item as Channel);
					this.cache.set(item.id, item as Channel);
				}
			});

			return {
				...data,
				items: [...(pagination?.items ?? []), ...(data.items as Channel[])],
			};
		};

		const room_id = room_id_signal();
		const l = this._getOrCreateListing(this._cachedListingsArchived, room_id);
		if ((l.resource as any).upgraded) {
			return l.resource;
		}

		const [resource, { mutate, refetch }] = createResource(
			() => [room_id_signal(), this.api.session()] as const,
			async ([room_id, session]) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
				let l = this._getOrCreateListing(this._cachedListingsArchived, room_id);
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

		(resource as any).upgraded = true;
		l.resource = resource;
		l.refetch = refetch;
		l.mutate = (value: Pagination<Channel>) => {
			l.pagination = value;
			mutate(value);
		};

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

			const data = await fetchWithRetry(() =>
				this.api.client.http.GET(
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
				)
			);

			batch(() => {
				for (const item of data.items) {
					this.normalize(item);
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const room_id = room_id_signal();
		const l = this._getOrCreateListing(this._cachedListingsRemoved, room_id);
		if ((l.resource as any).upgraded) {
			return l.resource;
		}

		const [resource, { mutate, refetch }] = createResource(
			() => [room_id_signal(), this.api.session()] as const,
			async ([room_id, session]) => {
				if (session?.status !== "Authorized") {
					return { items: [], total: 0, has_more: false };
				}
				let l = this._getOrCreateListing(this._cachedListingsRemoved, room_id);
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

		(resource as any).upgraded = true;
		l.resource = resource;
		l.refetch = refetch;
		l.mutate = (value: Pagination<Channel>) => {
			l.pagination = value;
			mutate(value);
		};

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
		{
			const t = this.cache.get(channel_id);
			if (t) {
				const is_unread = version_id < (t.last_version_id ?? "");
				if (
					t.last_read_id === version_id &&
					t.mention_count === 0 &&
					t.is_unread === is_unread
				) {
					return;
				}
			}
		}

		await fetchWithRetry(() =>
			this.api.client.http.PUT("/api/v1/channel/{channel_id}/ack", {
				params: { path: { channel_id: channel_id } },
				body: { message_id, version_id },
			})
		);

		const t = this.cache.get(channel_id);
		if (t) {
			const is_unread = version_id < (t.last_version_id ?? "");
			this.cache.set(channel_id, {
				...t,
				last_read_id: version_id,
				mention_count: 0,
				is_unread,
			});
		}
	}

	async ackBulk(
		acks: Array<
			{
				channel_id: string;
				message_id?: string;
				version_id: string;
				mention_count?: number;
			}
		>,
	) {
		const filteredAcks = acks.filter((ack) => {
			const t = this.cache.get(ack.channel_id);
			if (!t) return true;
			const is_unread = ack.version_id < (t.last_version_id ?? "");
			const mention_count = ack.mention_count ?? 0;
			return !(
				t.last_read_id === ack.version_id &&
				t.mention_count === mention_count &&
				t.is_unread === is_unread
			);
		});

		if (filteredAcks.length === 0) return;

		await fetchWithRetry(() =>
			this.api.client.http.POST("/api/v1/ack", {
				body: { acks: filteredAcks },
			})
		);

		batch(() => {
			for (const ack of filteredAcks) {
				const t = this.cache.get(ack.channel_id);
				if (t) {
					const is_unread = ack.version_id < (t.last_version_id ?? "");
					const mention_count = ack.mention_count ?? 0;
					this.cache.set(ack.channel_id, {
						...t,
						last_read_id: ack.version_id,
						mention_count,
						is_unread,
					});
				}
			}
		});
	}

	async lock(channel_id: string) {
		const service = this.getService();
		await service.update(channel_id, { locked: { allow_roles: [] } });
	}

	async unlock(channel_id: string) {
		const service = this.getService();
		await service.update(channel_id, { locked: null });
	}

	async archive(channel_id: string) {
		const service = this.getService();
		await service.update(channel_id, { archived: true });
	}

	async unarchive(channel_id: string) {
		const service = this.getService();
		await service.update(channel_id, { archived: false });
	}

	async createTag(
		channel_id: string,
		body: TagCreate,
	): Promise<Tag> {
		return await fetchWithRetry(() =>
			this.api.client.http.POST(
				"/api/v1/channel/{channel_id}/tag",
				{
					params: { path: { channel_id } },
					body: body,
				},
			)
		);
	}

	async updateTag(
		channel_id: string,
		tag_id: string,
		body: import("sdk").TagPatch,
	): Promise<import("sdk").Tag> {
		return await fetchWithRetry(() =>
			this.api.client.http.PATCH(
				"/api/v1/channel/{channel_id}/tag/{tag_id}",
				{
					params: { path: { channel_id, tag_id } },
					body: body,
				},
			)
		);
	}

	async deleteTag(
		channel_id: string,
		tag_id: string,
		force: boolean = false,
	): Promise<void> {
		await fetchWithRetry(() =>
			this.api.client.http.DELETE(
				"/api/v1/channel/{channel_id}/tag/{tag_id}",
				{
					params: { path: { channel_id, tag_id } },
					query: { force },
				},
			)
		);
	}
}
