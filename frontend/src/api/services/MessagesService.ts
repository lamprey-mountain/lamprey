import { ReactiveMap } from "@solid-primitives/map";
import type {
	Media,
	Message,
	MessageCreate,
	MessageSearch,
	Pagination,
	PaginationQuery,
	RepliesMessage,
	RepliesResponse,
	UserWithRelationship,
} from "sdk";
import {
	type Accessor,
	batch,
	createEffect,
	createMemo,
	createResource,
	type Resource,
} from "solid-js";
import { uuidv7 } from "uuidv7";
import { deepEqual } from "@/utils/deepEqual";
import { logger } from "@/utils/logger";
import { BaseService } from "../core/Service";
import { MessageRange, MessageRanges, sortMessagesById } from "sdk";
export { MessageRange, MessageRanges } from "sdk";

export type MessageListAnchor =
	| { type: "backwards"; message_id?: string; limit: number }
	| { type: "forwards"; message_id?: string; limit: number }
	| { type: "context"; message_id: string; limit: number };

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

const log = logger.for("api/messages");

export type MessageMutator = {
	mutate: (r: MessageRange) => void;
	query: MessageListAnchor;
	channel_id: string;
};

export class MessagesService extends BaseService<Message> {
	protected cacheName = "message";

	getKey(item: Message): string {
		return item.id;
	}

	upsert(item: Message) {
		if (item.thread) {
			this.store.channels.upsert(item.thread);
		}
		super.upsert(item);
	}

	upsertBulk(items: Message[]) {
		for (const item of items) {
			if (item.thread) {
				// PERF: collect threads, call upsertBulk on channels service
				this.store.channels.upsert(item.thread);
			}
		}
		super.upsertBulk(items);
	}

	// TEMP: make this public for backwards compatibility
	// TODO: make this private
	public _ranges = new Map<string, MessageRanges>();

	public _versions = new ReactiveMap<string, number>();
	private _pendingFetches = new Map<string, Promise<unknown>>();

	private deduplicatedFetch<T>(
		key: string,
		fetcher: () => Promise<T>,
	): Promise<T> {
		if (this._pendingFetches.has(key)) {
			return this._pendingFetches.get(key) as Promise<T>;
		}
		const promise = fetcher().finally(() => {
			this._pendingFetches.delete(key);
		});
		this._pendingFetches.set(key, promise);
		return promise;
	}

	private getOrCreateCache(channel_id: string): MessageRanges {
		let c = this._ranges.get(channel_id);
		if (!c) {
			c = new MessageRanges();
			this._ranges.set(channel_id, c);
		}
		return c;
	}

	_bumpVersion(channel_id: string) {
		this._versions.set(channel_id, (this._versions.get(channel_id) ?? 0) + 1);
	}

	async fetch(_id: string): Promise<Message> {
		throw new Error("Use fetchInChannel(channel_id, message_id)");
	}

	use(
		channel_id: Accessor<string>,
		message_id: Accessor<string | undefined>,
	): Resource<Message | undefined> {
		const [resource, { mutate }] = createResource(
			() => ({ channel_id: channel_id(), message_id: message_id() }),
			async ({ channel_id, message_id }) => {
				if (!message_id) return undefined;

				const cached = this.cache.get(message_id);
				if (cached) return cached;

				if (this.db && this.cacheName) {
					try {
						const cached = await this.db.get(
							this.cacheName,
							this.getDbKey(message_id),
						);
						if (cached) {
							this.upsert(cached);
							this.fetchInChannel(channel_id, message_id).catch(() => {});
							return cached;
						}
					} catch (_e) {
						// IndexedDB error, continue with API fetch
					}
				}

				return this.fetchInChannel(channel_id, message_id);
			},
		);

		createEffect(() => {
			const messageId = message_id();
			if (!messageId) return;

			const item = this.cache.get(messageId);
			if (item !== undefined && resource() !== item) {
				mutate(() => item);
			}
		});

		return resource;
	}

	async fetchInChannel(
		channel_id: string,
		message_id: string,
	): Promise<Message> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id, message_id } },
				},
			),
		);
		const m = data as Message;
		this.upsert(m);
		return m;
	}

	useList(
		channel_id: Accessor<string>,
		dir: Accessor<MessageListAnchor>,
	): Resource<MessageRange> {
		const source = createMemo(
			() => ({
				channel_id: channel_id(),
				dir: { ...dir() },
				_v: this._versions.get(channel_id()) ?? 0,
			}),
			undefined,
			{
				equals: (a, b) => {
					return (
						a._v === b._v &&
						a.channel_id === b.channel_id &&
						deepEqual(a.dir, b.dir)
					);
				},
			},
		);

		const [resource, { mutate }] = createResource(
			source,
			async ({ channel_id, dir }) => {
				await this.ensureHydrated(channel_id);
				const cache = this.getOrCreateCache(channel_id);
				const slice = this.getSlice(cache, dir);

				if (slice && !slice.stale) return slice;

				if (slice?.stale) {
					// immediately show stale data, but keep fetching
					mutate(slice);
				}

				return await this.fetchRange(channel_id, dir, cache);
			},
		);

		return resource;
	}

	async fetchSlice(
		channel_id: string,
		anchor: MessageListAnchor,
	): Promise<MessageRange> {
		await this.ensureHydrated(channel_id);
		const cache = this.getOrCreateCache(channel_id);
		const slice = this.getSlice(cache, anchor);

		if (slice && !slice.stale) return slice;

		return await this.fetchRange(channel_id, anchor, cache);
	}

	getCachedSlice(
		channel_id: string,
		anchor: MessageListAnchor,
	): MessageRange | undefined {
		const cache = this._ranges.get(channel_id);
		if (!cache) return undefined;
		const slice = this.getSlice(cache, anchor);
		return slice && !slice.stale ? slice : undefined;
	}

	private getSlice(
		ranges: MessageRanges,
		dir: MessageListAnchor,
	): MessageRange | undefined {
		log.debug("get slice", { ...dir, ranges: ranges.ranges.size });
		while (ranges.tryMerge());

		if (dir.type === "forwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (!r) return undefined;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx === -1) return undefined;

				// not enough messages, trigger fetch
				if (idx + dir.limit > r.items.length && r.has_forward) {
					return undefined;
				}

				const end = Math.min(idx + dir.limit, r.items.length);
				return r.slice(idx, end);
			} else {
				const r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
				if (!r) return undefined;

				if (dir.limit > r.items.length && r.has_forward) {
					return undefined;
				}

				return r.slice(0, Math.min(dir.limit, r.items.length));
			}
		}

		if (dir.type === "backwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (!r) return undefined;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx === -1) return undefined;
				const end = idx + 1;

				// not enough messages, trigger fetch
				if (end - dir.limit < 0 && r.has_backwards) {
					return undefined;
				}

				const start = Math.max(end - dir.limit, 0);
				return r.slice(start, end);
			} else {
				const r = ranges.live;
				if (r.items.length < dir.limit && r.has_backwards) {
					return undefined;
				}

				const start = Math.max(r.items.length - dir.limit, 0);
				const slice = r.slice(start, r.items.length);
				if (!slice.isEmpty()) {
					log.debug("returning live timeline", slice);
				}
				return slice;
			}
		}

		const r = ranges.findNearest(dir.message_id);
		if (!r) return undefined;
		let idx = r.items.findIndex((i) => i.id === dir.message_id);
		if (idx === -1) idx = r.items.findIndex((i) => i.id > dir.message_id);
		if (idx === -1) return undefined;

		const half = Math.floor(dir.limit / 2);
		const start = Math.max(idx - half, 0);

		if (
			(idx - half < 0 && r.has_backwards) ||
			(start + dir.limit > r.items.length && r.has_forward)
		) {
			return undefined;
		}

		const end = Math.min(start + dir.limit, r.items.length);
		return r.slice(start, end);
	}

	private async fetchRange(
		channel_id: string,
		dir: MessageListAnchor,
		ranges: MessageRanges,
	): Promise<MessageRange> {
		log.debug("fetch range", {
			channel_id,
			ranges: ranges.ranges.size,
			...dir,
		});

		// 1. Fetch Phase: Fill holes
		if (dir.type === "forwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (r) {
					const idx = r.items.findIndex((i) => i.id === dir.message_id);
					if (idx !== -1) {
						if (idx + dir.limit <= r.len || !r.has_forward) {
							// reuse
						} else {
							const data = await this.fetchList(channel_id, {
								dir: "f",
								limit: Math.max(100, dir.limit),
								from: r.end,
							});
							const nr = this.mergeAfter(ranges, r, data, data.has_more);
							ranges.replace(r, nr);
						}
					}
				} else {
					const data = await this.fetchList(channel_id, {
						dir: "f",
						limit: Math.max(100, dir.limit),
						from: dir.message_id,
					});
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, true, []),
							data,
							data.has_more,
						);
						ranges.ranges.add(range);
					});
				}
			} else {
				const r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
				if (!r) {
					const data = await this.fetchList(channel_id, {
						dir: "f",
						limit: Math.max(100, dir.limit),
					});
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, false, []),
							data,
							data.has_more,
						);
						ranges.ranges.add(range);
					});
				} else if (r.len < dir.limit && r.has_forward) {
					const data = await this.fetchList(channel_id, {
						dir: "f",
						limit: Math.max(100, dir.limit),
						from: r.end,
					});
					const nr = this.mergeAfter(ranges, r, data, data.has_more);
					ranges.replace(r, nr);
				}
			}
		} else if (dir.type === "backwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (r) {
					const idx = r.items.findIndex((i) => i.id === dir.message_id);
					if (idx !== -1) {
						if (idx + 1 >= dir.limit || !r.has_backwards) {
							// reuse
						} else {
							const data = await this.fetchList(channel_id, {
								dir: "b",
								limit: Math.max(100, dir.limit),
								from: r.start,
							});
							const nr = this.mergeBefore(ranges, r, data, data.has_more);
							ranges.replace(r, nr);
						}
					}
				} else {
					const data = await this.fetchList(channel_id, {
						dir: "b",
						limit: Math.max(100, dir.limit),
						from: dir.message_id,
					});
					batch(() => {
						const range = this.mergeBefore(
							ranges,
							new MessageRange(true, false, []),
							data,
							data.has_more,
						);
						ranges.ranges.add(range);
					});
				}
			} else {
				const range = ranges.live;
				if (range.isEmpty()) {
					const data = await this.fetchList(channel_id, {
						dir: "b",
						limit: Math.max(100, dir.limit),
					});
					const nr = this.mergeBefore(ranges, range, data, data.has_more);
					ranges.replace(range, nr);
				} else if (range.len < dir.limit && range.has_backwards) {
					const data = await this.fetchList(channel_id, {
						dir: "b",
						limit: Math.max(100, dir.limit),
						from: range.start,
					});
					const nr = this.mergeBefore(ranges, range, data, data.has_more);
					ranges.replace(range, nr);
				}
			}
		} else if (dir.type === "context") {
			const r = ranges.find(dir.message_id);
			if (r) {
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx !== -1) {
					const half = Math.floor(dir.limit / 2);
					const start = Math.max(idx - half, 0);

					const hasEnoughForwards =
						start + dir.limit <= r.len || !r.has_forward;
					const hasEnoughBackwards = idx - half >= 0 || !r.has_backwards;

					if (!hasEnoughBackwards || !hasEnoughForwards) {
						let dataBefore: Pagination<Message> | undefined;
						let dataAfter: Pagination<Message> | undefined;
						if (!hasEnoughBackwards) {
							dataBefore = await this.fetchList(channel_id, {
								dir: "b",
								limit: Math.max(100, dir.limit),
								from: r.start,
							});
						}
						if (!hasEnoughForwards) {
							dataAfter = await this.fetchList(channel_id, {
								dir: "f",
								limit: Math.max(100, dir.limit),
								from: r.end,
							});
						}
						batch(() => {
							let updated = r;
							if (dataBefore) {
								updated = this.mergeBefore(
									ranges,
									updated,
									dataBefore,
									dataBefore.has_more,
								);
							}
							if (dataAfter) {
								updated = this.mergeAfter(
									ranges,
									updated,
									dataAfter,
									dataAfter.has_more,
								);
							}
							if (updated !== r) {
								ranges.replace(r, updated);
							}
						});
					}
				} else {
					const data = await this.fetchContext(
						channel_id,
						dir.message_id,
						dir.limit,
					);
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, data.has_before ?? false, []),
							{ items: data.items as Message[], has_more: data.has_after },
							data.has_after,
						);
						ranges.ranges.add(range);
						if (!range.has_forward) {
							ranges.live = range;
						}
					});
				}
			} else {
				const data = await this.fetchContext(
					channel_id,
					dir.message_id,
					dir.limit,
				);
				batch(() => {
					const range = this.mergeAfter(
						ranges,
						new MessageRange(false, data.has_before ?? false, []),
						{ items: data.items as Message[], has_more: data.has_after },
						data.has_after,
					);
					ranges.ranges.add(range);
					if (!range.has_forward) {
						ranges.live = range;
					}
				});
			}
		}

		// try to merge as many ranges together as possible
		while (ranges.tryMerge());

		// insert everything into the cache
		const allItems = [...ranges.ranges].flatMap((r) => r.items as Message[]);
		this.upsertBulk(allItems);

		const slice = this.getSlice(ranges, dir);
		if (!slice) {
			if ("message_id" in dir && dir.message_id) {
				log.warn("slice not resolved, falling back to live tail", dir);
				return await this.fetchRange(
					channel_id,
					{ type: "backwards", limit: dir.limit },
					ranges,
				);
			}
			throw new Error(`Failed to resolve slice for ${JSON.stringify(dir)}`);
		}
		return slice;
	}

	handleMessageCreate(m: Message) {
		batch(() => {
			this.upsert(m);
			const nonce = m.nonce;
			if (nonce) this.cache.delete(nonce);

			const ranges = this._ranges.get(m.channel_id);
			if (!ranges) return;

			for (const range of [...ranges.ranges]) {
				const isLive = range === ranges.live;
				const alreadyContains = range.contains(m.id);
				const hasNonce =
					nonce && range.items.some((i) => i.nonce === nonce || i.id === nonce);

				if (isLive || alreadyContains || hasNonce) {
					ranges.replace(range, range.mergeMessageWithNonce(m, nonce));
				}
			}

			this._bumpVersion(m.channel_id);
		});
	}

	handleMessageUpdate(m: Message) {
		batch(() => {
			this.upsert(m);
			const ranges = this._ranges.get(m.channel_id);
			if (ranges) {
				for (const range of [...ranges.ranges]) {
					if (range.contains(m.id)) {
						ranges.replace(range, range.mergeMessages([m]));
					}
				}
				this._bumpVersion(m.channel_id);
			}
		});
	}

	handleMessageDelete(channel_id: string, message_id: string) {
		batch(() => {
			this.delete(message_id);
			const ranges = this._ranges.get(channel_id);
			if (ranges) {
				for (const range of [...ranges.ranges]) {
					const idx = range.items.findIndex((m) => m.id === message_id);
					if (idx !== -1) {
						const items = [...range.items];
						items.splice(idx, 1);
						ranges.replace(
							range,
							new MessageRange(range.has_forward, range.has_backwards, items),
						);
					}
				}
				this._bumpVersion(channel_id);
			}
		});
	}

	async send(channel_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();
		const session = this.store.session();
		const user_id = session && "user_id" in session ? session.user_id : "";

		// TODO: move local = ... into a function
		const local = {
			id,
			channel_id,
			author_id: user_id,
			created_at: new Date().toISOString(),
			latest_version: {
				version_id: id,
				type: "DefaultMarkdown",
				content: body.content,
				attachments: body.attachments,
				embeds: body.embeds ?? [],
				created_at: new Date().toISOString(),
				mentions: { users: [], roles: [], everyone: false },
			},
			nonce: id,
			is_local: true,
		} as unknown as Message;

		batch(() => {
			this.upsert(local);
			const ranges = this._ranges.get(channel_id);
			if (ranges) {
				ranges.replace(
					ranges.live,
					ranges.live.mergeMessageWithNonce(local, id),
				);
				this._bumpVersion(channel_id);
			}
		});

		const data = await this.retryWithBackoff<Message>(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id } },
				body: {
					...body,
					attachments: body.attachments.map(
						(a: { media_id?: string; id?: string; spoiler?: boolean }) => ({
							type: "Media" as const,
							media_id: a.media_id ?? a.id ?? "",
							spoiler: a.spoiler ?? false,
						}),
					),
				},
				headers: { "Idempotency-Key": id },
			}),
		);
		const m = data as Message;
		m.nonce = id;

		// replace local echo
		this.handleMessageCreate(m);

		return m;
	}

	async edit(
		channel_id: string,
		message_id: string,
		content: string,
	): Promise<Message> {
		const originalMessage = this.cache.get(message_id);
		if (originalMessage) {
			const updatedMessage = {
				...originalMessage,
				latest_version: {
					...originalMessage.latest_version,
					content: content,
					created_at: new Date().toISOString(),
					version_id: uuidv7(),
				},
				is_local: true,
			} as unknown as Message;
			this.handleMessageUpdate(updatedMessage);
		}

		try {
			const { data, error } = await this.client.http.PATCH(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id, message_id } },
					body: { content },
				},
			);
			if (error) throw error;
			const m = data as Message;
			this.handleMessageUpdate(m);
			return m;
		} catch (e) {
			if (originalMessage) {
				this.handleMessageUpdate(originalMessage);
			}
			throw e;
		}
	}

	async deleteBulk(channel_id: string, message_ids: string[]) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id } },
			body: { delete: message_ids },
		});
	}

	async removeBulk(channel_id: string, message_ids: string[]) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id } },
			body: { remove: message_ids },
		});
	}

	// TODO: the next few methods are directly ported from the old api impl. these should probably return promises rather than resources, and have useFoo variants for resources
	// FIXME: rewrite these methods to be idiomatic
	listReplies(
		channel_id: () => string,
		message_id: () => string | undefined,
		query?: () => { depth?: number; breadth?: number } & PaginationQuery,
	): Resource<RepliesResponse> {
		const [resource] = createResource(
			() => ({
				channel_id: channel_id(),
				message_id: message_id(),
				query: query?.(),
			}),
			async ({ channel_id, message_id, query }) => {
				const { data, error } = await (message_id
					? this.client.http.GET(
							"/api/v1/channel/{channel_id}/reply/{message_id}",
							{
								params: { path: { channel_id, message_id }, query },
							},
						)
					: this.client.http.GET("/api/v1/channel/{channel_id}/reply", {
							params: { path: { channel_id }, query },
						}));
				if (error) throw error;

				const extractMessages = (nodes: RepliesMessage[]): Message[] => {
					let messages: Message[] = [];
					for (const node of nodes) {
						messages.push(node.message);
						if (node.children) {
							messages = messages.concat(extractMessages(node.children));
						}
					}
					return messages;
				};

				const messages = extractMessages((data as RepliesResponse).children);
				this.upsertBulk(messages);

				return data as RepliesResponse;
			},
		);
		return resource;
	}

	// TODO: pinned message cache?
	listPinned(channel_id_signal: () => string): Resource<Pagination<Message>> {
		const paginate = async (pagination?: Pagination<Message>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.client.http.GET(
				"/api/v1/channel/{channel_id}/pin",
				{
					params: {
						path: { channel_id: channel_id_signal() },
						query: {
							dir: "f",
							limit: 1024,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);
			if (error) throw error;

			this.upsertBulk(data.items as unknown as Message[]);

			return {
				...data,
				items: [
					...(pagination?.items ?? []),
					...(data.items as unknown as Message[]),
				],
			} as Pagination<Message>;
		};

		const channel_id = channel_id_signal();
		const l = this._pinnedListings.get(channel_id);
		if (l) {
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<Message>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._pinnedListings.set(channel_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			channel_id_signal,
			async (channel_id) => {
				const l = this._pinnedListings.get(channel_id);
				if (l === undefined) {
					return {
						items: [],
						has_more: false,
						total: 0,
					} as Pagination<Message>;
				}
				if (l.prom) {
					await l.prom;
					if (l.pagination === null) {
						return {
							items: [],
							has_more: false,
							total: 0,
						} as Pagination<Message>;
					}
					return l.pagination;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l.pagination = res;
				l.prom = null;

				for (const mut of this._pinnedListingMutators) {
					if (mut.channel_id === channel_id) mut.mutate(res);
				}

				return res;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		const mut = { channel_id: channel_id_signal(), mutate };
		this._pinnedListingMutators.add(mut);

		createEffect(() => {
			mut.channel_id = channel_id_signal();
		});

		return resource;
	}

	async reorderPins(
		channel_id: string,
		messages: { id: string; position: number }[],
	) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/pin", {
			params: { path: { channel_id } },
			body: { messages },
		});
	}

	async pin(channel_id: string, message_id: string) {
		await this.client.http.PUT(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id, message_id } },
			},
		);
	}

	async unpin(channel_id: string, message_id: string) {
		await this.client.http.DELETE(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id, message_id } },
			},
		);
	}

	async search(body: Record<string, unknown>): Promise<MessageSearch> {
		const { data, error } = await this.client.http.POST(
			"/api/v1/search/message",
			{
				body,
			},
		);
		if (error) throw error;

		const { users, threads, room_members, thread_members, messages } = data;

		this.upsertBulk(messages as Message[]);

		if (users) {
			for (const user of users) {
				const userWithRelationship: UserWithRelationship = {
					...user,
					relationship: {
						relation: null,
						until: null,
						note: null,
						petname: null,
					},
				};
				this.store.users.upsert(userWithRelationship);
			}
		}

		if (threads) {
			for (const thread of threads) {
				this.store.channels.upsert(thread);
			}
		}

		if (room_members) {
			for (const member of room_members) {
				this.store.roomMembers.upsert(member);
			}
		}

		if (thread_members) {
			for (const member of thread_members) {
				this.store.threadMembers.upsert(member);
			}
		}

		return {
			...data,
			approximate_total: data.total,
			messages: messages as Message[],
		};
	}

	public _pinnedListings = new Map<
		string,
		{
			resource: Resource<Pagination<Message>>;
			refetch: () => void;
			mutate: (value: Pagination<Message>) => void;
			prom: Promise<Pagination<Message>> | null;
			pagination: Pagination<Message> | null;
		}
	>();
	public _pinnedListingMutators = new Set<{
		channel_id: string;
		mutate: (value: Pagination<Message>) => void;
	}>();

	// Helpers
	private async fetchList(channel_id: string, query: PaginationQuery) {
		const key = `list:${channel_id}:${JSON.stringify(query)}`;
		return this.deduplicatedFetch(key, async () => {
			const { data, error } = await this.client.http.GET(
				"/api/v1/channel/{channel_id}/message",
				{
					params: { path: { channel_id }, query },
				},
			);
			if (error) throw error;
			return data as unknown as Pagination<Message>;
		});
	}

	private async fetchContext(
		channel_id: string,
		message_id: string,
		limit: number,
	) {
		const key = `context:${channel_id}:${message_id}:${limit}`;
		return this.deduplicatedFetch(key, async () => {
			const { data, error } = await this.client.http.GET(
				"/api/v1/channel/{channel_id}/context/{message_id}",
				{
					params: {
						path: { channel_id, message_id },
						query: { limit },
					},
				},
			);
			if (error) throw error;
			return data;
		});
	}

	private mergeAfter(
		_ranges: MessageRanges,
		range: MessageRange,
		data: { items: Message[]; has_more?: boolean },
		has_more: boolean,
		markFresh = false,
	): MessageRange {
		const newItems = data.items;
		this.upsertBulk(newItems);

		const merged = range.mergeMessages(newItems, markFresh);
		// If no items were fetched, treat as no more to prevent infinite loops
		const effectiveHasMore = newItems.length === 0 ? false : has_more;
		return new MessageRange(
			effectiveHasMore,
			merged.has_backwards,
			merged.items,
			merged.stale,
		);
	}

	private mergeBefore(
		_ranges: MessageRanges,
		range: MessageRange,
		data: { items: Message[]; has_more?: boolean },
		has_more: boolean,
		markFresh = false,
	): MessageRange {
		const newItems = data.items as unknown as Message[];
		this.upsertBulk(newItems);

		const merged = range.mergeMessages(newItems, markFresh);
		// If no items were fetched, treat as no more to prevent infinite loops
		const effectiveHasMore = newItems.length === 0 ? false : has_more;
		return new MessageRange(
			merged.has_forward,
			effectiveHasMore,
			merged.items,
			merged.stale,
		);
	}

	private _hydrated = new Set<string>();

	private async ensureHydrated(channel_id: string) {
		// TODO: hydration logic is extremely sketchy
		// return;

		if (this._hydrated.has(channel_id)) return;
		const rehydrated = await this.rehydrateRanges(channel_id);
		this._ranges.set(channel_id, rehydrated);
		this._hydrated.add(channel_id);
	}

	private async rehydrateRanges(channel_id: string): Promise<MessageRanges> {
		const cache = new MessageRanges();
		if (!this.db) return cache;

		const tx = this.db.transaction(["message_ranges", "message"], "readonly");
		const rangeStore = tx.objectStore("message_ranges");
		const messageStore = tx.objectStore("message");

		const allRanges = await rangeStore.index("channel_id").getAll(channel_id);

		for (const r of allRanges) {
			const bound = IDBKeyRange.bound(r.start_id, r.end_id);
			const messages = await messageStore.index("channel_id").getAll(bound);

			const memoryRange = new MessageRange(
				r.has_forward,
				r.has_backwards,
				sortMessagesById(messages),
				true,
			);
			cache.ranges.add(memoryRange);
		}

		// try to remerge ranges just in case
		while (cache.tryMerge());

		const live = Array.from(cache.ranges).find((r) => !r.has_forward);
		if (live) cache.live = live;

		return cache;
	}

	clear() {
		super.clear();
		for (const ranges of this._ranges.values()) {
			ranges.ranges.clear();
			ranges.live = new MessageRange(false, true, []);
		}
		this._ranges.clear();
		this._versions.clear();
		this._pendingFetches.clear();
		for (const v of this._pinnedListings.values()) {
			v.refetch();
		}
		this._pinnedListings.clear();
		this._pinnedListingMutators.clear();
		this._hydrated.clear();
	}
}
