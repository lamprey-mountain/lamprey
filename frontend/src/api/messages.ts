import type {
	Media,
	Message,
	MessageCreate,
	Pagination,
	PaginationQuery,
} from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createComputed,
	createEffect,
	createResource,
	onCleanup,
	type Resource,
} from "solid-js";
import { uuidv7 } from "uuidv7";
import { MessageType } from "../types.ts";
import type { Api } from "../api.tsx";
import { fetchWithRetry } from "./util.ts";

export type MessageMutator = {
	mutate: (r: MessageRange) => void;
	query: MessageListAnchor;
	thread_id: string;
};

export class MessageRange {
	constructor(
		public has_forward: boolean,
		public has_backwards: boolean,
		public items: Array<Message>,
	) {}

	isEmpty(): boolean {
		return this.items.length === 0;
	}

	/** Requires at least one item */
	get start(): string {
		return this.items[0]!.id;
	}

	/** Requires at least one item */
	get end(): string {
		return this.items.at(-1)!.id;
	}

	get len(): number {
		return this.items.length;
	}

	contains(message_id: string): boolean {
		if (this.isEmpty()) return false;
		return message_id >= this.start && message_id <= this.end;
	}

	slice(start: number, end: number): MessageRange {
		return new MessageRange(
			this.has_forward || end < this.len - 1,
			this.has_backwards || start !== 0,
			this.items.slice(start, end),
		);
	}
}

export class MessageRanges {
	live = new MessageRange(false, true, []);
	ranges = new Set([this.live]);

	find(message_id: string): MessageRange | null {
		for (const range of this.ranges) {
			if (range.contains(message_id)) return range;
		}
		return null;
	}

	findNearest(message_id: string): MessageRange | null {
		const r = this.find(message_id);
		if (r) return r;

		let best: MessageRange | null = null;
		for (const range of this.ranges) {
			if (range.isEmpty()) continue;
			if (range.start > message_id) {
				if (!best || range.start < best.start) {
					best = range;
				}
			}
		}

		if (!best) {
			for (const range of this.ranges) {
				if (range.isEmpty()) continue;
				if (!best || range.end > best.end) {
					best = range;
				}
			}
		}

		return best;
	}

	merge(a: MessageRange, b: MessageRange) {
		// TODO: profile performance
		const c = new MessageRange(
			a.has_forward && b.has_forward,
			a.has_backwards && b.has_backwards,
			[...new Set([...a.items.map((i) => i.id), ...b.items.map((i) => i.id)])]
				.toSorted((a, b) => a > b ? 1 : -1)
				.map((i) =>
					a.items.find((j) => i === j.id) ??
						b.items.find((j) => i === j.id)!
				),
		);
		console.log("mergeRanges", a, b, c);
		this.ranges.delete(a);
		this.ranges.delete(b);
		this.ranges.add(c);
		if (a === this.live || b === this.live) {
			this.live = c;
		}
		return c;
	}
}

export type MessageListAnchor =
	| {
		type: "backwards";
		message_id?: string;
		limit: number;
	}
	| {
		type: "forwards";
		message_id?: string;
		limit: number;
	}
	| { type: "context"; message_id: string; limit: number };

function assertEq<T>(a: T, b: T) {
	if (a !== b) throw new Error(`assert failed: ${a} !== ${b}`);
}

// TODO: save message media in cache, may need some more restructuring
export class Messages {
	public cache = new ReactiveMap<string, Message>();
	public cacheRanges = new Map<string, MessageRanges>();
	public _mutators = new Set<MessageMutator>();
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
	public _pinnedListingMutators = new Set<
		{ thread_id: string; mutate: (value: Pagination<Message>) => void }
	>();
	public api: Api = null as unknown as Api;

	list(
		thread_id_signal: () => string,
		dir_signal: () => MessageListAnchor,
	): Resource<MessageRange> {
		const query = () => ({
			thread_id: thread_id_signal(),
			dir: dir_signal(),
		});

		let old: { thread_id: string; dir: MessageListAnchor };
		const [resource, { mutate }] = createResource(
			query,
			async ({ thread_id, dir }, { value: oldValue }) => {
				// ugly, but seems to work
				if (
					old && old.thread_id === thread_id && old.dir.limit === dir.limit &&
					old.dir.type === dir.type && old.dir.message_id === dir.message_id &&
					oldValue
				) return oldValue!;
				old = { thread_id, dir };

				let ranges = this.cacheRanges.get(thread_id);
				if (!ranges) {
					ranges = new MessageRanges();
					this.cacheRanges.set(thread_id, ranges);
				}

				console.log("recalculate message list", {
					thread_id,
					dir,
					ranges,
				});

				if (!ranges) throw new Error("missing ranges!");

				// step 1. fetch more messages if needed
				if (dir.type === "forwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === dir.message_id);
							if (idx !== -1) {
								if (idx + dir.limit < r.len || !r.has_forward) {
									console.log("messages reuse range for forwards");
								} else {
									console.log("messages fetch more for forwards");
									const data = await this.fetchList(thread_id, {
										dir: "f",
										limit: 100,
										from: r.end,
									});
									const nr = this.mergeAfter(ranges, r, data);
									nr.has_forward = data.has_more;
								}
							} else {
								throw new Error("unreachable");
							}
						} else {
							console.log("messages fetch initial for forwards");
							const data = await this.fetchList(thread_id, {
								dir: "f",
								limit: 100,
								from: dir.message_id,
							});
							batch(() => {
								const range = this.mergeAfter(
									ranges!,
									new MessageRange(false, true, []),
									data,
								);
								range.has_forward = data.has_more;
								ranges!.ranges.add(range);
							});
						}
					} else {
						console.log("messages fetch start for forwards");
						// find a range that starts at the beginning
						let r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
						if (!r) {
							const data = await this.fetchList(thread_id, {
								dir: "f",
								limit: 100,
							});
							batch(() => {
								const range = this.mergeAfter(
									ranges!,
									new MessageRange(false, false, []),
									data,
								);
								range.has_forward = data.has_more;
								ranges!.ranges.add(range);
							});
						}
					}
				} else if (dir.type === "backwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === dir.message_id);
							if (idx !== -1) {
								if (idx >= dir.limit) {
									console.log("messages reuse range for backwards");
								} else if (r.has_backwards) {
									// fetch more
									const data = await this.fetchList(thread_id, {
										dir: "b",
										limit: 100,
										from: r.start,
									});
									const nr = this.mergeBefore(ranges, r, data);
									nr.has_backwards = data.has_more;
								}
							} else {
								throw new Error("unreachable");
							}
						} else {
							// new range
							console.log("messages fetch initial for backwards");
							const data = await this.fetchList(thread_id, {
								dir: "b",
								limit: 100,
								from: dir.message_id,
							});
							batch(() => {
								const range = this.mergeBefore(
									ranges!,
									new MessageRange(true, false, []),
									data,
								);
								range.has_backwards = data.has_more;
								ranges!.ranges.add(range);
							});
						}
					}

					const range = ranges.live;
					if (range.isEmpty()) {
						const data = await this.fetchList(thread_id, {
							dir: "b",
							limit: 100,
						});
						const nr = this.mergeBefore(ranges, range, data);
						nr.has_backwards = data.has_more;
					} else {
						// don't need to do anything
					}
				} else if (dir.type === "context") {
					const r = ranges.find(dir.message_id);

					if (r) {
						const idx = r.items.findIndex((i) => i.id === dir.message_id);
						if (idx !== -1) {
							const hasEnoughForwards = (idx <= r.len - dir.limit) ||
								!r.has_forward;
							const hasEnoughBackwards = (idx >= dir.limit) || !r.has_backwards;

							if (hasEnoughBackwards && hasEnoughForwards) {
								console.log("messages reuse range for context");
							} else {
								console.log("messages fetch more for context");
								let dataBefore: Pagination<Message> | undefined;
								let dataAfter: Pagination<Message> | undefined;

								if (!hasEnoughBackwards) {
									dataBefore = await this.fetchList(thread_id, {
										dir: "b",
										limit: 100,
										from: r.start,
									});
								}

								if (!hasEnoughForwards) {
									dataAfter = await this.fetchList(thread_id, {
										dir: "f",
										limit: 100,
										from: r.end,
									});
								}

								batch(() => {
									if (dataBefore) this.mergeBefore(ranges, r, dataBefore);
									if (dataAfter) this.mergeAfter(ranges, r, dataAfter);
								});
							}
						} else {
							// fetch thread (or hole)
							console.log("messages fetch context (hole)");
							const data = await this.fetchContext(
								thread_id,
								dir.message_id,
								dir.limit,
							);
							batch(() => {
								const range = this.mergeAfter(
									ranges!,
									new MessageRange(false, false, []),
									{
										items: data.items as Message[],
									},
								);
								range.has_backwards = data.has_before;
								range.has_forward = data.has_after;
								ranges!.ranges.add(range);
							});
						}
					} else {
						// new range
						console.log("messages fetch context");
						const data = await this.fetchContext(
							thread_id,
							dir.message_id,
							dir.limit,
						);
						console.log("messages done fetching context");
						batch(() => {
							const range = this.mergeAfter(
								ranges!,
								new MessageRange(false, false, []),
								{
									items: data.items as Message[],
								},
							);
							// TODO: unify these names
							range.has_backwards = data.has_before;
							range.has_forward = data.has_after;
							ranges!.ranges.add(range);
						});
					}
				}

				// step 2. get a slice of the message range
				if (dir.type === "forwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						if (!r) throw new Error("failed to fetch messages");
						const idx = r.items.findIndex((i) => i.id === dir.message_id);
						if (idx === -1) throw new Error("failed to fetch messages");
						const start = idx;
						const end = Math.min(idx + dir.limit, r.len);
						const s = r.slice(start, end);
						assertEq(s.start, dir.message_id);
						return s;
					} else {
						let r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
						if (!r) throw new Error("failed to fetch messages");
						const start = 0;
						const end = Math.min(dir.limit, r.len);
						return r.slice(start, end);
					}
				} else if (dir.type === "backwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						if (!r) throw new Error("failed to fetch messages");
						const idx = r.items.findIndex((i) => i.id === dir.message_id);
						if (idx === -1) throw new Error("failed to fetch messages");
						const end = idx + 1;
						const start = Math.max(end - dir.limit, 0);
						const s = r.slice(start, end);
						assertEq(s.end, dir.message_id);
						return s;
					} else {
						const r = ranges.live;
						const start = Math.max(r.len - dir.limit, 0);
						const end = Math.min(start + dir.limit, r.len);
						return r.slice(start, end);
					}
				} else if (dir.type === "context") {
					// fall back to nearest/next message if it doesnt exist
					const r = ranges.findNearest(dir.message_id);
					if (!r) throw new Error("failed to fetch messages");

					let idx = r.items.findIndex((i) => i.id === dir.message_id);
					if (idx === -1) {
						idx = r.items.findIndex((i) => i.id > dir.message_id);
					}
					if (idx === -1) throw new Error("failed to fetch messages");

					const end = Math.min(idx + dir.limit, r.len);
					const start = Math.max(idx - dir.limit, 0);
					const s = r.slice(start, end);
					return s;
				}

				throw new Error("unreachable");
			},
		);

		// HACK: surely there's a better way...
		const mut = { mutate } as unknown as MessageMutator;
		createComputed(() => {
			mut.query = dir_signal();
			mut.thread_id = thread_id_signal();
		});
		this._mutators.add(mut);
		onCleanup(() => {
			this._mutators.delete(mut);
		});

		return resource;
	}

	async send(channel_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();

		const local = {
			id,
			channel_id: channel_id,
			author_id: this.api.users.cache.get("@self")!.id,
			created_at: new Date().toISOString(),
			latest_version: {
				version_id: id,
				type: "DefaultMarkdown",
				content: body.content,
				attachments: body.attachments.map((a) => ({
					type: "Media",
					media: this.api.media.cacheInfo.get(a.id),
					spoiler: false,
				})),
				embeds: body.embeds ?? [],
				created_at: new Date().toISOString(),
			},
			nonce: id,
			is_local: true,
		} as unknown as Message;

		console.log("[message:send] local message", local);

		batch(() => {
			this.cache.set(id, local);
			const r = this.cacheRanges.get(channel_id);
			if (r) {
				r.live.items.push(local);
				this._updateMutators(r, channel_id);
			}
		});

		const data = await fetchWithRetry(() =>
			this.api.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: {
					path: { channel_id: channel_id },
				},
				body: {
					...body,
					attachments: body.attachments.map((a) => ({
						type: "Media",
						media_id: a.id,
					})),
				},
				headers: {
					"Idempotency-Key": id,
				},
			})
		);
		return data as Message;
	}

	fetch(thread_id: () => string, message_id: () => string): Resource<Message> {
		const query = () => ({
			thread_id: thread_id(),
			message_id: message_id(),
		});
		const [resource, { mutate }] = createResource(
			query,
			async ({ thread_id, message_id }) => {
				const m = this.cache.get(message_id);
				if (m) return m;
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/channel/{channel_id}/message/{message_id}",
					{
						params: {
							path: { channel_id: thread_id, message_id },
						},
					},
				);
				if (error) throw error;
				return data as Message;
			},
		);
		createEffect(() => {
			const m = this.cache.get(message_id());
			if (m) mutate(m);
		});
		return resource;
	}

	_updateMutators(r: MessageRanges, thread_id: string) {
		console.log("update mutators", this._mutators);
		for (const mut of this._mutators) {
			if (mut.thread_id !== thread_id) continue;

			// 1. Handle live backwards queries (the most common case for being at the bottom)
			if (mut.query.type === "backwards" && !mut.query.message_id) {
				const start = Math.max(r.live.len - mut.query.limit, 0);
				const end = r.live.len;
				mut.mutate(r.live.slice(start, end));
				continue;
			}

			// 2. Handle queries pinned to a specific message ID
			if (mut.query.message_id) {
				const range = r.find(mut.query.message_id);
				// We only care about ranges that are at the "live" end (no forward messages known)
				// because only those will be affected by a newly sent message.
				if (range && !range.has_forward) {
					const idx = range.items.findIndex((i) =>
						i.id === mut.query.message_id
					);
					if (idx !== -1) {
						let start: number;
						let end: number;

						if (mut.query.type === "forwards") {
							start = idx;
							end = Math.min(idx + mut.query.limit, range.len);
						} else if (mut.query.type === "backwards") {
							end = idx + 1;
							start = Math.max(end - mut.query.limit, 0);
						} else if (mut.query.type === "context") {
							start = Math.max(idx - mut.query.limit, 0);
							end = Math.min(idx + mut.query.limit, range.len);
						} else {
							continue;
						}

						mut.mutate(range.slice(start, end));
					}
				}
			}
		}
	}

	private async fetchList(thread_id: string, query: PaginationQuery) {
		const { data, error } = await this.api.client.http.GET(
			"/api/v1/channel/{channel_id}/message",
			{
				params: {
					path: { channel_id: thread_id },
					query,
				},
			},
		);
		if (error) throw error;
		return data as Pagination<Message>;
	}

	private async fetchContext(
		thread_id: string,
		message_id: string,
		limit: number,
	) {
		const { data, error } = await this.api.client.http.GET(
			"/api/v1/channel/{channel_id}/context/{message_id}",
			{
				params: {
					path: { channel_id: thread_id, message_id },
					query: { limit },
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async edit(thread_id: string, message_id: string, content: string) {
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
			this.cache.set(message_id, updatedMessage);
			const ranges = this.cacheRanges.get(thread_id);
			if (ranges) {
				const range = ranges.find(message_id);
				if (range) {
					const index = range.items.findIndex((m) => m.id === message_id);
					if (index !== -1) {
						range.items[index] = updatedMessage;
						this._updateMutators(ranges, thread_id);
					}
				}
			}
		}

		try {
			const { data, error } = await this.api.client.http.PATCH(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id: thread_id, message_id } },
					body: { content },
				},
			);
			if (error) throw error;
			return data as Message;
		} catch (e) {
			if (originalMessage) {
				this.cache.set(message_id, originalMessage);
				const ranges = this.cacheRanges.get(thread_id);
				if (ranges) {
					this._updateMutators(ranges, thread_id);
				}
			}
			throw e;
		}
	}

	async pin(thread_id: string, message_id: string) {
		await this.api.client.http.PUT(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id: thread_id, message_id } },
			},
		);
	}

	async unpin(thread_id: string, message_id: string) {
		await this.api.client.http.DELETE(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id: thread_id, message_id } },
			},
		);
	}

	listReplies(
		channel_id: () => string,
		message_id: () => string | undefined,
		query?: () => { depth?: number; breadth?: number } & PaginationQuery,
	): Resource<Pagination<Message>> {
		const [resource] = createResource(
			() => ({
				channel_id: channel_id(),
				message_id: message_id(),
				query: query?.(),
			}),
			async ({ channel_id, message_id, query }) => {
				const { data, error } = await (message_id
					? this.api.client.http.GET(
						"/api/v1/channel/{channel_id}/reply/{message_id}",
						{
							params: { path: { channel_id, message_id }, query },
						},
					)
					: this.api.client.http.GET(
						"/api/v1/channel/{channel_id}/reply",
						{
							params: { path: { channel_id }, query },
						},
					));
				if (error) throw error;

				batch(() => {
					for (const item of data.items) {
						this.cache.set(item.id, item as Message);
					}
				});

				return data as Pagination<Message>;
			},
		);
		return resource;
	}

	async reorderPins(
		thread_id: string,
		messages: { id: string; position: number }[],
	) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}/pin", {
			params: { path: { channel_id: thread_id } },
			body: { messages },
		});
	}

	async deleteBulk(thread_id: string, message_ids: string[]) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id: thread_id } },
			body: { delete: message_ids },
		});
	}

	async removeBulk(thread_id: string, message_ids: string[]) {
		await this.api.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id: thread_id } },
			body: { remove: message_ids },
		});
	}

	async search(body: any): Promise<import("sdk").MessageSearch> {
		const { data, error } = await this.api.client.http.POST(
			"/api/v1/search/message",
			{
				body,
			},
		);
		if (error) throw error;

		const { users, threads, room_members, thread_members, messages } = data;

		for (const message of messages) {
			this.cache.set(message.id, message as Message);
		}

		if (users) {
			for (const user of users) {
				this.api.users.cache.set(user.id, user);
			}
		}

		if (threads) {
			for (const thread of threads) {
				this.api.channels.normalize(thread);
				this.api.channels.cache.set(thread.id, thread);
			}
		}

		if (room_members) {
			for (const member of room_members) {
				let roomCache = this.api.room_members.cache.get(member.room_id);
				if (!roomCache) {
					roomCache = new ReactiveMap();
					this.api.room_members.cache.set(member.room_id, roomCache);
				}
				roomCache.set(member.user_id, member);
			}
		}

		if (thread_members) {
			for (const member of thread_members) {
				let threadCache = this.api.thread_members.cache.get(member.thread_id);
				if (!threadCache) {
					threadCache = new ReactiveMap();
					this.api.thread_members.cache.set(member.thread_id, threadCache);
				}
				threadCache.set(member.user_id, member);
			}
		}

		return {
			...data,
			approximate_total: data.total,
			messages: messages as Message[],
		};
	}

	listPinned(thread_id_signal: () => string): Resource<Pagination<Message>> {
		const paginate = async (pagination?: Pagination<Message>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/channel/{channel_id}/pin",
				{
					params: {
						path: { channel_id: thread_id_signal() },
						query: {
							dir: "f",
							limit: 1024,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);
			if (error) throw error;

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item as Message);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items as Message[]],
			} as Pagination<Message>;
		};

		const thread_id = thread_id_signal();
		const l = this._pinnedListings.get(thread_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<Message>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._pinnedListings.set(thread_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			thread_id_signal,
			async (thread_id) => {
				let l = this._pinnedListings.get(thread_id)!;
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._pinnedListingMutators) {
					if (mut.thread_id === thread_id) mut.mutate(res);
				}

				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		const mut = { thread_id: thread_id_signal(), mutate };
		this._pinnedListingMutators.add(mut);

		createEffect(() => {
			mut.thread_id = thread_id_signal();
		});

		return resource;
	}

	/** append a set of data to a range, deduplicating ranges if there are multiple */
	private mergeAfter(
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[]; has_more?: boolean },
	): MessageRange {
		let items: Array<Message> = [];
		for (const item of data.items) {
			const convertedItem = item as Message;
			this.cache.set(convertedItem.id, convertedItem);
			const existing = ranges.find(convertedItem.id);
			if (existing) {
				if (existing !== range) {
					console.log("merge (after)!");
					range.items.push(...items);
					items = [];
					range = ranges.merge(range, existing);
				}
			} else {
				items.push(convertedItem);
			}
		}
		range.items.push(...items);
		// NOTE: Timsort (used by V8) is O(N) for nearly sorted arrays, so this is fine.
		// If performance becomes an issue, we could use a binary-search insertion or merge.
		range.items.sort((a, b) => a.id > b.id ? 1 : -1);
		return range;
	}

	/** prepend a set of data to a range, deduplicating ranges if there are multiple */
	private mergeBefore(
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[]; has_more?: boolean },
	): MessageRange {
		let items: Array<Message> = [];
		for (const item of data.items) {
			const convertedItem = item as Message;
			this.cache.set(convertedItem.id, convertedItem);
			const existing = ranges.find(convertedItem.id);
			if (existing) {
				if (existing !== range) {
					console.log("merge (before)!");
					range.items.unshift(...items);
					items = [];
					range = ranges.merge(range, existing);
				}
			} else {
				items.push(convertedItem);
			}
		}
		range.items.unshift(...items);
		range.items.sort((a, b) => a.id > b.id ? 1 : -1);
		return range;
	}
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};
