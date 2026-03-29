import type {
	Media,
	Message,
	MessageCreate,
	Pagination,
	PaginationQuery,
} from "sdk";
import { BaseService } from "../core/Service";
import {
	type Accessor,
	batch,
	createComputed,
	createEffect,
	createMemo,
	createResource,
	onCleanup,
	type Resource,
} from "solid-js";
import { uuidv7 } from "uuidv7";
import { ReactiveMap } from "@solid-primitives/map";
import { logger } from "../../logger";
import { deepEqual } from "../../utils/deepEqual";

export type MessageListAnchor =
	| { type: "backwards"; message_id?: string; limit: number }
	| { type: "forwards"; message_id?: string; limit: number }
	| { type: "context"; message_id: string; limit: number };

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

/** sort messages and return a new message range */
function sortMessagesById(msgs: Message[]): Message[] {
	return [...msgs].sort((a, b) => a.id < b.id ? -1 : 1);
}

const log = logger.for("api/messages");

export class MessageRange {
	constructor(
		public has_forward: boolean,
		public has_backwards: boolean,
		public items: Array<Message>,
		public stale = false,
	) {}

	isEmpty(): boolean {
		return this.items.length === 0;
	}

	// TODO: make this return `string | undefined`
	get start(): string {
		return this.items[0]?.id ?? "";
	}

	get end(): string {
		return this.items.at(-1)?.id ?? "";
	}

	get len(): number {
		return this.items.length;
	}

	contains(message_id: string): boolean {
		if (this.isEmpty()) return false;
		return message_id >= this.start && message_id <= this.end;
	}

	/** return a new range of messages between these two indexes */
	slice(start: number, end: number): MessageRange {
		return new MessageRange(
			this.has_forward || end < this.len,
			this.has_backwards || start !== 0,
			this.items.slice(start, end),
		);
	}

	// NOTE: has_forwards/has_backwards may act strangely here
	mergeMessages(newItems: Message[], markFresh = false): MessageRange {
		const byId = new Map<string, Message>();
		for (const m of this.items) byId.set(m.id, m);
		for (const m of newItems) byId.set(m.id, m);
		return new MessageRange(
			this.has_forward,
			this.has_backwards,
			sortMessagesById([...byId.values()]),
			markFresh ? false : this.stale,
		);
	}

	mergeMessageWithNonce(message: Message, nonce?: string): MessageRange {
		const items = [...this.items];
		let idx = nonce
			? items.findIndex((m) => (m as any).nonce === nonce || m.id === nonce)
			: -1;
		if (idx === -1) idx = items.findIndex((m) => m.id === message.id);

		if (idx !== -1) {
			items[idx] = message;
		} else {
			items.push(message);
		}

		return new MessageRange(
			this.has_forward,
			this.has_backwards,
			sortMessagesById(items),
		);
	}

	mergeRange(other: MessageRange): MessageRange {
		if (this.isEmpty()) return other;
		if (other.isEmpty()) return this;
		const isStale = this.stale && other.stale;

		const byId = new Map<string, Message>();
		for (const m of this.items) byId.set(m.id, m);
		for (const m of other.items) byId.set(m.id, m);
		const items = sortMessagesById([...byId.values()]);

		let has_forward = false;
		if (this.end > other.end) has_forward = this.has_forward;
		else if (other.end > this.end) has_forward = other.has_forward;
		else has_forward = this.has_forward && other.has_forward;

		let has_backwards = false;
		if (this.start < other.start) has_backwards = this.has_backwards;
		else if (other.start < this.start) has_backwards = other.has_backwards;
		else has_backwards = this.has_backwards && other.has_backwards;

		return new MessageRange(has_forward, has_backwards, items, isStale);
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

	replace(old: MessageRange, updated: MessageRange) {
		this.ranges.delete(old);
		this.ranges.add(updated);
		if (this.live === old) this.live = updated;
	}

	add(r: MessageRange) {
		this.ranges.add(r);
	}

	tryMerge(): boolean {
		const sorted = [...this.ranges]
			.filter((r) => !r.isEmpty())
			.sort((a, b) => a.start < b.start ? -1 : 1);

		for (let i = 0; i < sorted.length - 1; i++) {
			const a = sorted[i]!, b = sorted[i + 1]!;
			const adjacent = !a.has_forward && !b.has_backwards;
			const overlapping = a.end >= b.start;

			if (adjacent || overlapping) {
				this.ranges.delete(a);
				this.ranges.delete(b);
				const fused = a.mergeRange(b);
				this.ranges.add(fused);
				if (this.live === a || this.live === b) this.live = fused;
				return true;
			}
		}
		return false;
	}
}

export type MessageMutator = {
	mutate: (r: MessageRange) => void;
	query: MessageListAnchor;
	thread_id: string;
};

export class MessagesService extends BaseService<Message> {
	protected cacheName = "message";

	getKey(item: Message): string {
		return item.id;
	}

	// TEMP: make this public for backwards compatibility
	// TODO: make this private
	public _ranges = new Map<string, MessageRanges>();

	private _versions = new ReactiveMap<string, number>();
	private _mutators = new Set<MessageMutator>();

	private getOrCreateCache(channel_id: string): MessageRanges {
		let c = this._ranges.get(channel_id);
		if (!c) {
			c = new MessageRanges();
			this._ranges.set(channel_id, c);
		}
		return c;
	}

	private bumpVersion(channel_id: string) {
		this._versions.set(channel_id, (this._versions.get(channel_id) ?? 0) + 1);
	}

	async fetch(id: string): Promise<Message> {
		throw new Error("Use fetchInThread(thread_id, message_id)");
	}

	async fetchInChannel(
		channel_id: string,
		message_id: string,
	): Promise<Message> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id: channel_id, message_id } },
				},
			)
		);
		const m = data as Message;
		this.upsert(m);
		return m;
	}

	useList(
		thread_id: Accessor<string>,
		dir: Accessor<MessageListAnchor>,
	): Resource<MessageRange> {
		const source = createMemo(
			() => ({
				thread_id: thread_id(),
				dir: dir(),
				_v: this._versions.get(thread_id()) ?? 0,
			}),
			undefined,
			{
				equals: (a, b) =>
					a._v === b._v && a.thread_id === b.thread_id &&
					deepEqual(a.dir, b.dir),
			},
		);

		const [resource, { mutate }] = createResource(
			source,
			async ({ thread_id, dir }) => {
				await this.ensureHydrated(thread_id);
				const cache = this.getOrCreateCache(thread_id);
				const slice = this.getSlice(cache, dir);

				if (slice && !slice.stale) return slice;

				if (slice?.stale) {
					// immediately show stale data, but keep fetching
					mutate(slice);
				}

				return await this.fetchRange(thread_id, dir, cache);
			},
		);

		return resource;
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

					const hasEnoughForwards = start + dir.limit <= r.len ||
						!r.has_forward;
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
							{ items: (data.items as Message[]), has_more: data.has_after },
							data.has_after,
						);
						ranges.ranges.add(range);
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
						{ items: (data.items as Message[]), has_more: data.has_after },
						data.has_after,
					);
					ranges.ranges.add(range);
				});
			}
		}

		// try to merge as many ranges together as possible
		while (ranges.tryMerge());

		// insert everything into the cache
		const allItems = [...ranges.ranges].flatMap((r) => r.items as Message[]);
		batch(() => {
			for (const m of allItems) this.upsert(m);
		});

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
			const nonce = (m as any).nonce;
			if (nonce) this.cache.delete(nonce);

			const ranges = this._ranges.get(m.channel_id);
			if (!ranges) return;

			for (const range of [...ranges.ranges]) {
				const isLive = range === ranges.live;
				const alreadyContains = range.contains(m.id);
				const hasNonce = nonce && range.items.some(
					(i) => (i as any).nonce === nonce || i.id === nonce,
				);

				if (isLive || alreadyContains || hasNonce) {
					ranges.replace(range, range.mergeMessageWithNonce(m, nonce));
				}
			}

			this.bumpVersion(m.channel_id);
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
				this.bumpVersion(m.channel_id);
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
				this.bumpVersion(channel_id);
			}
		});
	}

	async send(channel_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();

		// TODO: move local = ... into a function
		const local = ({
			id,
			channel_id,
			author_id: (this.store.session() as any)?.user_id ?? "",
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
		} as unknown) as Message;

		batch(() => {
			this.upsert(local);
			const ranges = this._ranges.get(channel_id);
			if (ranges) {
				ranges.replace(
					ranges.live,
					ranges.live.mergeMessageWithNonce(local, id),
				);
				this.bumpVersion(channel_id);
			}
		});

		const data = await this.retryWithBackoff<Message>(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id } },
				body: {
					...body,
					attachments: body.attachments.map((a: any) => ({
						type: "Media" as const,
						media_id: a.media_id ?? a.id,
						spoiler: a.spoiler ?? false,
					})),
				},
				headers: { "Idempotency-Key": id },
			})
		);
		const m = data as Message;
		(m as any).nonce = id;

		// replace local echo
		this.handleMessageCreate(m);

		return m;
	}

	async edit(
		thread_id: string,
		message_id: string,
		content: string,
	): Promise<Message> {
		const originalMessage = this.cache.get(message_id);
		if (originalMessage) {
			const updatedMessage = ({
				...originalMessage,
				latest_version: {
					...originalMessage.latest_version,
					content: content,
					created_at: new Date().toISOString(),
					version_id: uuidv7(),
				},
				is_local: true,
			} as unknown) as Message;
			this.handleMessageUpdate(updatedMessage);
		}

		try {
			const { data, error } = await this.client.http.PATCH(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id: thread_id, message_id } },
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

	async deleteBulk(thread_id: string, message_ids: string[]) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id: thread_id } },
			body: { delete: message_ids },
		});
	}

	async removeBulk(thread_id: string, message_ids: string[]) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/message", {
			params: { path: { channel_id: thread_id } },
			body: { remove: message_ids },
		});
	}

	// TODO: the next few methods are directly ported from the old api impl. these should probably return promises rather than resources, and have useFoo variants for resources
	// FIXME: rewrite these methods to be idiomatic
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
					? this.client.http.GET(
						"/api/v1/channel/{channel_id}/reply/{message_id}",
						{
							params: { path: { channel_id, message_id }, query },
						},
					)
					: this.client.http.GET(
						"/api/v1/channel/{channel_id}/reply",
						{
							params: { path: { channel_id }, query },
						},
					));
				if (error) throw error;

				batch(() => {
					for (const item of data.items) {
						this.upsert(item as Message);
					}
				});

				return data as Pagination<Message>;
			},
		);
		return resource;
	}

	// TODO: pinned message cache?
	listPinned(thread_id_signal: () => string): Resource<Pagination<Message>> {
		const paginate = async (pagination?: Pagination<Message>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.client.http.GET(
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
					this.upsert(item as unknown as Message);
				}
			});

			return {
				...data,
				items: [
					...pagination?.items ?? [],
					...data.items as unknown as Message[],
				],
			} as Pagination<Message>;
		};

		const thread_id = thread_id_signal();
		const l = this._pinnedListings.get(thread_id);
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
		this._pinnedListings.set(thread_id, l2);

		const [resource, { mutate, refetch }] = createResource(
			thread_id_signal,
			async (thread_id) => {
				const l = this._pinnedListings.get(thread_id)!;
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

	async reorderPins(
		thread_id: string,
		messages: { id: string; position: number }[],
	) {
		await this.client.http.PATCH("/api/v1/channel/{channel_id}/pin", {
			params: { path: { channel_id: thread_id } },
			body: { messages },
		});
	}

	async pin(thread_id: string, message_id: string) {
		await this.client.http.PUT(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id: thread_id, message_id } },
			},
		);
	}

	async unpin(thread_id: string, message_id: string) {
		await this.client.http.DELETE(
			"/api/v1/channel/{channel_id}/pin/{message_id}",
			{
				params: { path: { channel_id: thread_id, message_id } },
			},
		);
	}

	async search(body: any): Promise<import("sdk").MessageSearch> {
		const { data, error } = await this.client.http.POST(
			"/api/v1/search/message",
			{
				body,
			},
		);
		if (error) throw error;

		const { users, threads, room_members, thread_members, messages } = data;

		for (const message of messages) {
			this.upsert(message as import("sdk").Message);
		}

		if (users) {
			for (const user of users) {
				const userWithRelationship: import("sdk").UserWithRelationship = {
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
			messages: messages as import("sdk").Message[],
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
	public _pinnedListingMutators = new Set<
		{ thread_id: string; mutate: (value: Pagination<Message>) => void }
	>();

	// Helpers
	private async fetchList(thread_id: string, query: PaginationQuery) {
		const { data, error } = await this.client.http.GET(
			"/api/v1/channel/{channel_id}/message",
			{
				params: { path: { channel_id: thread_id }, query },
			},
		);
		if (error) throw error;
		return (data as unknown) as Pagination<Message>;
	}

	private async fetchContext(
		thread_id: string,
		message_id: string,
		limit: number,
	) {
		const { data, error } = await this.client.http.GET(
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

	private mergeAfter(
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[]; has_more?: boolean },
		has_more: boolean,
		markFresh = false,
	): MessageRange {
		const newItems = (data.items as unknown) as Message[];
		for (const item of newItems) this.upsert(item);

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
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[]; has_more?: boolean },
		has_more: boolean,
		markFresh = false,
	): MessageRange {
		const newItems = (data.items as unknown) as Message[];
		for (const item of newItems) this.upsert(item);

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
		return;

		if (this._hydrated.has(channel_id)) return;
		const rehydrated = await this.rehydrateRanges(channel_id);
		this._ranges.set(channel_id, rehydrated);
		this._hydrated.add(channel_id);
	}

	private async rehydrateRanges(
		channel_id: string,
	): Promise<MessageRanges> {
		const cache = new MessageRanges();
		if (!this.db) return cache;

		const tx = this.db.transaction(["message_ranges", "messages"], "readonly");
		const rangeStore = tx.objectStore("message_ranges");
		const messageStore = tx.objectStore("messages");

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

	private async persistRanges(channel_id: string) {
		if (!this.db) return;

		const ranges = this._ranges.get(channel_id);
		if (!ranges) return;

		const tx = this.db.transaction("message_ranges", "readwrite");
		const store = tx.objectStore("message_ranges");

		// for now, just delete all ranges and recreate
		const existing = await store.index("channel_id").getAllKeys(channel_id);
		for (const key of existing) await store.delete(key);

		for (const r of ranges.ranges) {
			if (r.isEmpty()) continue;
			await store.put({
				id: uuidv7(),
				channel_id,
				start_id: r.start,
				end_id: r.end,
				has_forward: r.has_forward,
				has_backwards: r.has_backwards,
			});
		}
	}
}
