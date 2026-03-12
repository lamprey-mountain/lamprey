import {
	Media,
	Message,
	MessageCreate,
	Pagination,
	PaginationQuery,
} from "sdk";
import { BaseService } from "../core/Service";
import {
	Accessor,
	batch,
	createComputed,
	createEffect,
	createResource,
	onCleanup,
	Resource,
} from "solid-js";
import { uuidv7 } from "uuidv7";
import { ReactiveMap } from "@solid-primitives/map";

// --- Message Range Logic (Ported from api/messages.ts) ---

export class MessageRange {
	constructor(
		public has_forward: boolean,
		public has_backwards: boolean,
		public items: Array<Message>,
	) {}

	isEmpty(): boolean {
		return this.items.length === 0;
	}

	get start(): string {
		return this.items[0]!.id;
	}

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
		const c = new MessageRange(
			a.has_forward && b.has_forward,
			a.has_backwards && b.has_backwards,
			[...new Set([...a.items.map((i) => i.id), ...b.items.map((i) => i.id)])]
				.sort((a, b) => (a > b ? 1 : -1))
				.map(
					(i) =>
						a.items.find((j) => i === j.id) ??
							b.items.find((j) => i === j.id)!,
				),
		);
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
	| { type: "backwards"; message_id?: string; limit: number }
	| { type: "forwards"; message_id?: string; limit: number }
	| { type: "context"; message_id: string; limit: number };

export type MessageMutator = {
	mutate: (r: MessageRange) => void;
	query: MessageListAnchor;
	thread_id: string;
};

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

export class MessagesService extends BaseService<Message> {
	protected cacheName = "message";

	getKey(item: Message): string {
		return item.id;
	}

	cacheRanges = new Map<string, MessageRanges>();
	private _mutators = new Set<MessageMutator>();

	async fetch(id: string): Promise<Message> {
		throw new Error("Use fetchInThread(thread_id, message_id)");
	}

	async fetchInThread(thread_id: string, message_id: string): Promise<Message> {
		const data = await this.retryWithBackoff(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id: thread_id, message_id } },
				},
			)
		);
		const m = data as Message;
		this.upsert(m);
		return m;
	}

	/**
	 * Reactively fetch and list messages.
	 */
	useList(
		thread_id_signal: Accessor<string>,
		dir_signal: Accessor<MessageListAnchor>,
	): Resource<MessageRange> {
		const query = () => ({
			thread_id: thread_id_signal(),
			dir: dir_signal(),
		});

		let old: { thread_id: string; dir: MessageListAnchor };

		const [resource, { mutate }] = createResource<
			MessageRange,
			{ thread_id: string; dir: MessageListAnchor }
		>(
			query,
			async ({ thread_id, dir }, { value: oldValue }) => {
				// Dedup check
				if (
					old &&
					old.thread_id === thread_id &&
					old.dir.limit === dir.limit &&
					old.dir.type === dir.type &&
					old.dir.message_id === dir.message_id &&
					oldValue
				) {
					return oldValue!;
				}
				old = { thread_id, dir };

				let ranges = this.cacheRanges.get(thread_id);
				if (!ranges) {
					ranges = new MessageRanges();
					this.cacheRanges.set(thread_id, ranges);
				}

				return await this.resolveRange(thread_id, dir, ranges);
			},
		);

		// Mutator registration
		const mut = ({ mutate } as unknown) as MessageMutator;
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

	private getSlice(
		ranges: MessageRanges,
		dir: MessageListAnchor,
	): MessageRange | undefined {
		if (dir.type === "forwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (!r) return undefined;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx === -1) return undefined;
				return r.slice(idx, Math.min(idx + dir.limit, r.len));
			} else {
				const r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
				if (!r) return undefined;
				return r.slice(0, Math.min(dir.limit, r.len));
			}
		} else if (dir.type === "backwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (!r) return undefined;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx === -1) return undefined;
				const end = idx + 1;
				return r.slice(Math.max(end - dir.limit, 0), end);
			} else {
				const r = ranges.live;
				const start = Math.max(r.len - dir.limit, 0);
				return r.slice(start, Math.min(start + dir.limit, r.len));
			}
		} else {
			// context
			const r = ranges.findNearest(dir.message_id);
			if (!r) return undefined;
			let idx = r.items.findIndex((i) => i.id === dir.message_id);
			if (idx === -1) idx = r.items.findIndex((i) => i.id > dir.message_id);
			if (idx === -1) return undefined;
			return r.slice(
				Math.max(idx - dir.limit, 0),
				Math.min(idx + dir.limit, r.len),
			);
		}
	}

	private async resolveRange(
		thread_id: string,
		dir: MessageListAnchor,
		ranges: MessageRanges,
	): Promise<MessageRange> {
		// 1. Fetch Phase: Fill holes
		if (dir.type === "forwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id);
				if (r) {
					const idx = r.items.findIndex((i) => i.id === dir.message_id);
					if (idx !== -1) {
						if (idx + dir.limit < r.len || !r.has_forward) {
							// reuse
						} else {
							const data = await this.fetchList(thread_id, {
								dir: "f",
								limit: 100,
								from: r.end,
							});
							const nr = this.mergeAfter(ranges, r, data);
							nr.has_forward = data.has_more;
						}
					}
				} else {
					const data = await this.fetchList(thread_id, {
						dir: "f",
						limit: 100,
						from: dir.message_id,
					});
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, true, []),
							data,
						);
						range.has_forward = data.has_more;
						ranges.ranges.add(range);
					});
				}
			} else {
				let r = Array.from(ranges.ranges).find((r) => !r.has_backwards);
				if (!r) {
					const data = await this.fetchList(thread_id, {
						dir: "f",
						limit: 100,
					});
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, false, []),
							data,
						);
						range.has_forward = data.has_more;
						ranges.ranges.add(range);
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
							// reuse
						} else if (r.has_backwards) {
							const data = await this.fetchList(thread_id, {
								dir: "b",
								limit: 100,
								from: r.start,
							});
							const nr = this.mergeBefore(ranges, r, data);
							nr.has_backwards = data.has_more;
						}
					}
				} else {
					const data = await this.fetchList(thread_id, {
						dir: "b",
						limit: 100,
						from: dir.message_id,
					});
					batch(() => {
						const range = this.mergeBefore(
							ranges,
							new MessageRange(true, false, []),
							data,
						);
						range.has_backwards = data.has_more;
						ranges.ranges.add(range);
					});
				}
			} else {
				const range = ranges.live;
				if (range.isEmpty()) {
					const data = await this.fetchList(thread_id, {
						dir: "b",
						limit: 100,
					});
					const nr = this.mergeBefore(ranges, range, data);
					nr.has_backwards = data.has_more;
				}
			}
		} else if (dir.type === "context") {
			const r = ranges.find(dir.message_id);
			if (r) {
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				if (idx !== -1) {
					const hasEnoughForwards = idx <= r.len - dir.limit || !r.has_forward;
					const hasEnoughBackwards = idx >= dir.limit || !r.has_backwards;
					if (!hasEnoughBackwards || !hasEnoughForwards) {
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
					const data = await this.fetchContext(
						thread_id,
						dir.message_id,
						dir.limit,
					);
					batch(() => {
						const range = this.mergeAfter(
							ranges,
							new MessageRange(false, false, []),
							{ items: (data.items as Message[]) },
						);
						range.has_backwards = data.has_before;
						range.has_forward = data.has_after;
						ranges.ranges.add(range);
					});
				}
			} else {
				const data = await this.fetchContext(
					thread_id,
					dir.message_id,
					dir.limit,
				);
				batch(() => {
					const range = this.mergeAfter(
						ranges,
						new MessageRange(false, false, []),
						{ items: (data.items as Message[]) },
					);
					range.has_backwards = data.has_before;
					range.has_forward = data.has_after;
					ranges.ranges.add(range);
				});
			}
		}

		// 2. Read Phase: Get slice
		const slice = this.getSlice(ranges, dir);
		if (!slice) throw new Error("Failed to resolve message range");
		return slice;
	}

	// Update Mutators (Called by Store on sync events)
	updateMutators(thread_id: string) {
		const ranges = this.cacheRanges.get(thread_id);
		if (!ranges) return;

		for (const mut of this._mutators) {
			if (mut.thread_id !== thread_id) continue;
			const slice = this.getSlice(ranges, mut.query);
			if (slice) {
				mut.mutate(slice);
			}
		}
	}

	private updateRangeItem(range: MessageRange, item: Message, nonce?: string) {
		if (nonce) {
			const idx = range.items.findIndex((i) =>
				(i as any).nonce === nonce || i.id === nonce
			);
			if (idx !== -1) {
				range.items[idx] = item;
				return true;
			}
		}
		const id_idx = range.items.findIndex((i) => i.id === item.id);
		if (id_idx !== -1) {
			range.items[id_idx] = item;
			return true;
		}
		return false;
	}

	handleMessageCreate(m: Message) {
		batch(() => {
			const ranges = this.cacheRanges.get(m.channel_id);
			if (ranges) {
				for (const range of ranges.ranges) {
					if (!this.updateRangeItem(range, m, (m as any).nonce)) {
						if (range === ranges.live || range.contains(m.id)) {
							range.items.push(m);
						}
					}
					range.items.sort((a, b) => (a.id > b.id ? 1 : -1));
				}
				this.updateMutators(m.channel_id);
			}

			if ((m as any).nonce) {
				this.cache.delete((m as any).nonce);
			}
			this.upsert(m);
		});
	}

	handleMessageUpdate(m: Message) {
		batch(() => {
			this.upsert(m);
			const ranges = this.cacheRanges.get(m.channel_id);
			if (ranges) {
				for (const range of ranges.ranges) {
					this.updateRangeItem(range, m);
				}
				this.updateMutators(m.channel_id);
			}
		});
	}

	handleMessageDelete(channel_id: string, message_id: string) {
		batch(() => {
			this.delete(message_id);
			const ranges = this.cacheRanges.get(channel_id);
			if (ranges) {
				for (const range of ranges.ranges) {
					const idx = range.items.findIndex((i) =>
						i.id === message_id ||
						((i as any).nonce && (i as any).nonce === message_id)
					);
					if (idx !== -1) range.items.splice(idx, 1);
				}
				this.updateMutators(channel_id);
			}
		});
	}

	async send(channel_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();
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
			const r = this.cacheRanges.get(channel_id);
			if (r) {
				r.live.items.push(local);
				r.live.items.sort((a, b) => (a.id > b.id ? 1 : -1));
				this.updateMutators(channel_id);
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

		// Final update to replace local echo
		this.handleMessageCreate(m);

		return m;
	}

	async edit(thread_id: string, message_id: string, content: string) {
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
					this.upsert(item as Message);
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
		data: { items: any[] },
	): MessageRange {
		const items = (data.items as unknown) as Message[];
		for (const item of items) {
			this.upsert(item);
			if (!this.updateRangeItem(range, item, (item as any).nonce)) {
				range.items.push(item);
			}
		}
		range.items.sort((a, b) => (a.id > b.id ? 1 : -1));
		return range;
	}

	private mergeBefore(
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[] },
	): MessageRange {
		const items = (data.items as unknown) as Message[];
		for (const item of items) {
			this.upsert(item);
			if (!this.updateRangeItem(range, item, (item as any).nonce)) {
				range.items.unshift(item);
			}
		}
		range.items.sort((a, b) => (a.id > b.id ? 1 : -1));
		return range;
	}
}
