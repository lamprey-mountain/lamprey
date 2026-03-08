import {
	Media,
	Message,
	MessageCreate,
	Pagination,
	PaginationQuery,
} from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";
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
				.sort((a, b) => a > b ? 1 : -1)
				.map((i) =>
					a.items.find((j) => i === j.id) ??
						b.items.find((j) => i === j.id)!
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
	getKey(item: Message): string {
		return item.id;
	}

	cacheRanges = new Map<string, MessageRanges>();
	private _mutators = new Set<MessageMutator>();

	async fetch(id: string): Promise<Message> {
		// Usually we don't fetch single messages by ID without thread_id context in current API
		throw new Error("Use fetchInThread(thread_id, message_id)");
	}

	async fetchInThread(thread_id: string, message_id: string): Promise<Message> {
		const data = await fetchWithRetry(() =>
			this.client.http.GET(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: { path: { channel_id: thread_id, message_id } },
				},
			)
		);
		this.upsert(data as Message);
		return data as Message;
	}

	// This is the main "list" method used by the chat view
	list(
		thread_id_signal: Accessor<string>,
		dir_signal: Accessor<MessageListAnchor>,
	): Resource<MessageRange> {
		const query = () => ({
			thread_id: thread_id_signal(),
			dir: dir_signal(),
		});

		let old: { thread_id: string; dir: MessageListAnchor };

		const [resource, { mutate }] = createResource(
			query,
			async ({ thread_id, dir }, { value: oldValue }) => {
				// Dedup check
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

				// The logic here is identical to api/messages.ts `list` method
				// Simplified for brevity in this first pass, but ideally we copy the logic exactly
				// to maintain behavior.

				return await this.resolveRange(thread_id, dir, ranges);
			},
		);

		// Mutator registration
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

	private async resolveRange(
		thread_id: string,
		dir: MessageListAnchor,
		ranges: MessageRanges,
	): Promise<MessageRange> {
		// Ported logic from api/messages.ts
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
					const hasEnoughForwards = (idx <= r.len - dir.limit) ||
						!r.has_forward;
					const hasEnoughBackwards = (idx >= dir.limit) || !r.has_backwards;
					if (!hasEnoughBackwards || !hasEnoughForwards) {
						let dataBefore, dataAfter;
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
							{ items: data.items as Message[] },
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
						{ items: data.items as Message[] },
					);
					range.has_backwards = data.has_before;
					range.has_forward = data.has_after;
					ranges.ranges.add(range);
				});
			}
		}

		// Return slice
		if (dir.type === "forwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id)!;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				return r.slice(idx, Math.min(idx + dir.limit, r.len));
			} else {
				const r = Array.from(ranges.ranges).find((r) => !r.has_backwards)!;
				return r.slice(0, Math.min(dir.limit, r.len));
			}
		} else if (dir.type === "backwards") {
			if (dir.message_id) {
				const r = ranges.find(dir.message_id)!;
				const idx = r.items.findIndex((i) => i.id === dir.message_id);
				const end = idx + 1;
				return r.slice(Math.max(end - dir.limit, 0), end);
			} else {
				const r = ranges.live;
				const start = Math.max(r.len - dir.limit, 0);
				return r.slice(start, Math.min(start + dir.limit, r.len));
			}
		} else { // context
			const r = ranges.findNearest(dir.message_id)!;
			let idx = r.items.findIndex((i) => i.id === dir.message_id);
			if (idx === -1) idx = r.items.findIndex((i) => i.id > dir.message_id);
			return r.slice(
				Math.max(idx - dir.limit, 0),
				Math.min(idx + dir.limit, r.len),
			);
		}
	}

	// Update Mutators (Called by Store on sync events)
	updateMutators(thread_id: string) {
		const ranges = this.cacheRanges.get(thread_id);
		if (!ranges) return;

		for (const mut of this._mutators) {
			if (mut.thread_id !== thread_id) continue;
			// Simplified update logic - assumes logic similar to original implementation
			if (mut.query.type === "backwards" && !mut.query.message_id) {
				const start = Math.max(ranges.live.len - mut.query.limit, 0);
				mut.mutate(ranges.live.slice(start, ranges.live.len));
				continue;
			}

			if (mut.query.message_id) {
				const range = ranges.find(mut.query.message_id);
				if (range && !range.has_forward) {
					const idx = range.items.findIndex((i) =>
						i.id === mut.query.message_id
					);
					if (idx !== -1) {
						let s = 0, e = 0;
						if (mut.query.type === "forwards") {
							s = idx;
							e = Math.min(idx + mut.query.limit, range.len);
						} else if (mut.query.type === "backwards") {
							e = idx + 1;
							s = Math.max(e - mut.query.limit, 0);
						} else { // context
							s = Math.max(idx - mut.query.limit, 0);
							e = Math.min(idx + mut.query.limit, range.len);
						}
						mut.mutate(range.slice(s, e));
					}
				}
			}
		}
	}

	async send(channel_id: string, body: MessageSendReq): Promise<Message> {
		const id = uuidv7();
		const local = {
			id,
			channel_id,
			author_id: this.store.users.cache.get("@self")?.id ?? "",
			created_at: new Date().toISOString(),
			latest_version: {
				version_id: id,
				type: "DefaultMarkdown",
				content: body.content,
				attachments: [], // Simplified for now
				embeds: body.embeds ?? [],
				created_at: new Date().toISOString(),
			},
			nonce: id,
			is_local: true,
		} as unknown as Message;

		batch(() => {
			this.upsert(local);
			const r = this.cacheRanges.get(channel_id);
			if (r) {
				r.live.items.push(local);
				this.updateMutators(channel_id);
			}
		});

		const data = await fetchWithRetry(() =>
			this.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id } },
				body: { ...body, attachments: [] }, // Simplified
				headers: { "Idempotency-Key": id },
			})
		);
		return data as Message;
	}

	// Helpers
	private async fetchList(thread_id: string, query: PaginationQuery) {
		const { data, error } = await this.client.http.GET(
			"/api/v1/channel/{channel_id}/message",
			{
				params: { path: { channel_id: thread_id }, query },
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
		// Simplified merge logic
		const items = data.items as Message[];
		for (const item of items) this.upsert(item);
		range.items.push(...items);
		range.items.sort((a, b) => a.id > b.id ? 1 : -1);
		return range;
	}

	private mergeBefore(
		ranges: MessageRanges,
		range: MessageRange,
		data: { items: any[] },
	): MessageRange {
		const items = data.items as Message[];
		for (const item of items) this.upsert(item);
		range.items.unshift(...items);
		range.items.sort((a, b) => a.id > b.id ? 1 : -1);
		return range;
	}
}
