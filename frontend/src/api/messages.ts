import type {
	Media,
	Message,
	MessageCreate,
	Pagination,
	PaginationQuery,
	PaginationResponseMessage,
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
									this.mergeAfter(ranges, r, data);
								}
							} else {
								throw new Error("unreachable");
							}
						} else {
							console.log("messages fetch initial for forwards");
							throw new Error("todo");
						}
					} else {
						console.log("messages fetch start for forwards");
						throw new Error("todo");
					}
				} else if (dir.type === "backwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === dir.message_id);
							if (idx !== -1) {
								if (idx >= dir.limit) {
									console.log("messages reuse range for backwards");
								}

								// fetch more
								const data = await this.fetchList(thread_id, {
									dir: "b",
									limit: 100,
									from: r.start,
								});
								this.mergeBefore(ranges, r, data);
							} else {
								throw new Error("unreachable");
							}
						} else {
							// new range
							throw new Error("todo");
						}
					}

					const range = ranges.live;
					if (range.isEmpty()) {
						const data = await this.fetchList(thread_id, {
							dir: "b",
							limit: 100,
						});
						this.mergeBefore(ranges, range, data);
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
								let dataBefore: Pagination<Message>;
								let dataAfter: Pagination<Message>;

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
							// fetch thread
							throw new Error("todo");
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
								ranges,
								new MessageRange(false, false, []),
								{
									items: data.items,
									has_more: data.has_before,
									total: data.total,
								},
							);
							// TODO: unify these names
							range.has_backwards = data.has_before;
							range.has_forward = data.has_after;
							ranges.ranges.add(range);
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
						throw new Error("todo");
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
					const r = ranges.find(dir.message_id);
					if (!r) throw new Error("failed to fetch messages");

					const idx = r.items.findIndex((i) => i.id === dir.message_id);
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

	async send(thread_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();
		const local: Message = {
			type: MessageType.DefaultMarkdown,
			id,
			thread_id,
			version_id: id,
			override_name: null,
			reply_id: null,
			content: null,
			author_id: this.api.users.cache.get("@self")!.id,
			metadata: null,
			ordering: 0,
			...body,
			nonce: id,
			is_local: true,
		};

		const r = this.cacheRanges.get(thread_id);
		if (r) {
			r.live.items.push(local);
			this._updateMutators(r, thread_id);
		}

		const { data, error } = await this.api.client.http.POST(
			"/api/v1/thread/{thread_id}/message",
			{
				params: {
					path: { thread_id },
				},
				body: {
					...body,
					attachments: body.attachments?.map((i) => ({ id: i.id })),
					nonce: id,
				},
				headers: {
					"Idempotency-Key": id,
				},
			},
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
		return data;
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
					"/api/v1/thread/{thread_id}/message/{message_id}",
					{
						params: {
							path: { thread_id, message_id },
						},
					},
				);
				if (error) {
					console.error(error);
					throw new Error(error);
				}
				return data;
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
			if (mut.query.type !== "backwards") continue;
			if (mut.query.message_id) continue;
			const start = Math.max(r.live.len - mut.query.limit, 0);
			const end = Math.min(start + mut.query.limit, r.live.len);
			mut.mutate(r.live.slice(start, end));
		}
	}

	private async fetchList(thread_id: string, query: PaginationQuery) {
		const { data, error } = await this.api.client.http.GET(
			"/api/v1/thread/{thread_id}/message",
			{
				params: {
					path: { thread_id },
					query,
				},
			},
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
		return data;
	}

	private async fetchContext(
		thread_id: string,
		message_id: string,
		limit: number,
	) {
		const { data, error } = await this.api.client.http.GET(
			"/api/v1/thread/{thread_id}/context/{message_id}",
			{
				params: {
					path: { thread_id, message_id },
					query: { limit },
				},
			},
		);
		if (error) {
			console.error(error);
			throw new Error(error);
		}
		return data;
	}

	async edit(thread_id: string, message_id: string, content: string) {
		const originalMessage = this.cache.get(message_id);
		if (originalMessage) {
			const updatedMessage = {
				...originalMessage,
				content: content,
				edited_at: new Date().toISOString(),
				version_id: uuidv7(), // fake version_id to show (edited)
				is_local: true,
			} as Message;
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
				"/api/v1/thread/{thread_id}/message/{message_id}",
				{
					params: { path: { thread_id, message_id } },
					body: { content },
				},
			);
			if (error) {
				throw new Error(error);
			}
			return data;
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
			"/api/v1/thread/{thread_id}/pin/{message_id}",
			{
				params: { path: { thread_id, message_id } },
			},
		);
	}

	async unpin(thread_id: string, message_id: string) {
		await this.api.client.http.DELETE(
			"/api/v1/thread/{thread_id}/pin/{message_id}",
			{
				params: { path: { thread_id, message_id } },
			},
		);
	}

	async reorderPins(
		thread_id: string,
		messages: { id: string; position: number }[],
	) {
		await this.api.client.http.PATCH("/api/v1/thread/{thread_id}/pin", {
			params: { path: { thread_id } },
			body: { messages },
		});
	}

	listPinned(thread_id_signal: () => string): Resource<Pagination<Message>> {
		const paginate = async (pagination?: Pagination<Message>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/thread/{thread_id}/pin",
				{
					params: {
						path: { thread_id: thread_id_signal() },
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
		data: PaginationResponseMessage,
	): MessageRange {
		let items: Array<Message> = [];
		for (const item of data.items) {
			this.cache.set(item.id, item);
			const existing = ranges.find(item.id);
			if (existing) {
				if (existing !== range) {
					console.log("merge (after)!");
					range.items.push(...items);
					items = [];
					range = ranges.merge(range, existing);
				}
			} else {
				items.push(item);
			}
		}
		range.items.push(...items);
		range.has_forward &&= data.has_more;
		return range;
	}

	/** prepend a set of data to a range, deduplicating ranges if there are multiple */
	private mergeBefore(
		ranges: MessageRanges,
		range: MessageRange,
		data: PaginationResponseMessage,
	): MessageRange {
		let items: Array<Message> = [];
		for (const item of data.items) {
			this.cache.set(item.id, item);
			const existing = ranges.find(item.id);
			if (existing) {
				if (existing !== range) {
					console.log("merge (before)!");
					range.items.unshift(...items);
					items = [];
					range = ranges.merge(range, existing);
				}
			} else {
				items.push(item);
			}
		}
		range.items.unshift(...items);
		range.has_backwards &&= data.has_more;
		return range;
	}
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};
