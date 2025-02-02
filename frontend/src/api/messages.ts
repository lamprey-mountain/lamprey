import {
	Media,
	Message,
	MessageCreate,
	PaginationQuery,
	PaginationResponseMessage,
} from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createComputed,
	createResource,
	onCleanup,
	Resource,
} from "solid-js";
import { uuidv7 } from "uuidv7";
import { MessageType } from "../types.ts";
import { Api } from "../api.tsx";

export type MessageMutator = {
	mutate: (r: MessageRange) => void;
	query: MessageListAnchor;
	thread_id: string;
};

export class MessageRange {
	public mutators = new Set<MessageMutator>();

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

export class Messages {
	public cache = new ReactiveMap<string, Message>();
	public cacheRanges = new Map<string, MessageRanges>();
	public _mutators = new Set<MessageMutator>();
	public api: Api = null as unknown as Api;

	list(
		thread_id_signal: () => string,
		dir_signal: () => MessageListAnchor,
	): Resource<MessageRange> {
		// always have Ranges for the current thread
		createComputed(() => {
			const thread_id = thread_id_signal();
			if (!this.cacheRanges.has(thread_id)) {
				this.cacheRanges.set(thread_id, new MessageRanges());
			}
		});

		const update = async (
			a: { thread_id: string; dir: MessageListAnchor },
			b: { value?: MessageRange },
		): Promise<MessageRange> => {
			try {
				return await _update(a, b);
			} catch (err) {
				console.error(err);
				throw err;
			}
		};

		const fetchList = async (thread_id: string, query: PaginationQuery) => {
			const { data, error } = await this.api.client.http.GET(
				"/api/v1/thread/{thread_id}/message",
				{
					params: {
						path: { thread_id },
						query,
					},
				},
			);
			if (error) throw new Error(error);
			return data;
		};

		const fetchContext = async (
			thread_id: string,
			message_id: string,
			limit: number,
		) => {
			const { data, error } = await this.api.client.http.GET(
				"/api/v1/thread/{thread_id}/context/{message_id}",
				{
					params: {
						path: { thread_id, message_id },
						query: { limit },
					},
				},
			);
			if (error) throw new Error(error);
			return data;
		};

		const mergeRanges = (
			a: MessageRange,
			b: MessageRange,
		) => {
			const bids = new Set(b.items.map((i) => i.id));
			const sharedStart = a.items.findIndex((i) => bids.has(i.id));
			const sharedEnd = a.items.findLastIndex((i) => bids.has(i.id));
			console.log("mergeRanges", a, b, sharedStart, sharedEnd);
			return new MessageRange(
				a.has_forward && b.has_forward,
				a.has_backwards && b.has_backwards,
				// [
				// 	...a.items.slice(
				// 		0,
				// 		sharedStart === -1 ? a.items.length : sharedStart,
				// 	),
				// 	...b.items,
				// 	...a.items.slice(sharedEnd === -1 ? 0 : sharedEnd),
				// ],
				[...new Set([...a.items.map((i) => i.id), ...b.items.map((i) => i.id)])]
					.map((i) =>
						a.items.find((j) => i === j.id) ??
							b.items.find((j) => i === j.id)!
					),
			);
		};

		/** append a set of data to a range, deduplicating ranges if there are multiple */
		const mergeAfter = (
			ranges: MessageRanges,
			range: MessageRange,
			data: PaginationResponseMessage,
		): MessageRange => {
			const items: Array<Message> = [];
			for (const item of data.items) {
				this.cache.set(item.id, item);
				const existing = ranges.find(item.id);
				if (existing && existing !== range) {
					console.log("merge (after)!");
					ranges.ranges.delete(range);
					ranges.ranges.delete(existing);
					range = mergeRanges(range, existing);
					ranges.ranges.add(range);
					if (existing === ranges.live) ranges.live = range;
				} else {
					items.push(item);
				}
			}
			range.items.push(...items);
			range.has_forward = range.has_forward ? data.has_more : false;
			return range;
		};

		/** prepend a set of data to a range, deduplicating ranges if there are multiple */
		const mergeBefore = (
			ranges: MessageRanges,
			range: MessageRange,
			data: PaginationResponseMessage,
		): MessageRange => {
			const items: Array<Message> = [];
			for (const item of data.items) {
				this.cache.set(item.id, item);
				const existing = ranges.find(item.id);
				if (existing && existing !== range) {
					console.log("merge (before)!");
					ranges.ranges.delete(range);
					ranges.ranges.delete(existing);
					range = mergeRanges(range, existing);
					ranges.ranges.add(range);
				} else {
					items.push(item);
				}
			}
			range.items.unshift(...items);
			range.has_backwards = range.has_backwards ? data.has_more : false;
			return range;
		};

		let old: { thread_id: string; dir: MessageListAnchor };
		const _update = async (
			{ thread_id, dir }: { thread_id: string; dir: MessageListAnchor },
			{ value: oldValue }: { value?: MessageRange },
		): Promise<MessageRange> => {
			// HACK: force tracking
			dir.type;
			dir.limit;
			dir.message_id;
			console.log("diff", { thread_id, dir }, old);

			// ugly, but seems to work
			if (
				old && old.thread_id === thread_id && old.dir.limit === dir.limit &&
				old.dir.type === dir.type && old.dir.message_id === dir.message_id
			) return oldValue!;
			old = { thread_id, dir };

			const ranges = this.cacheRanges.get(thread_id)!;

			console.log("recalculate message list", {
				thread_id,
				dir,
				ranges,
			});

			if (dir.type === "forwards") {
				if (dir.message_id) {
					const r = ranges.find(dir.message_id);
					// console.log(ranges, r);
					if (r) {
						const idx = r.items.findIndex((i) => i.id === dir.message_id);
						if (idx !== -1) {
							if (idx + dir.limit < r.len || !r.has_forward) {
								console.log("messages reuse range for forwards");
								const start = idx;
								const end = Math.min(idx + dir.limit, r.len);
								const s = r.slice(start, end);
								assertEq(s.start, dir.message_id);
								return s;
							}

							console.log("messages fetch more for forwards");
							const data = await fetchList(thread_id, {
								dir: "f",
								limit: 100,
								from: r.end,
							});
							const range = mergeAfter(ranges, r, data);
							const idx2 = range.items.findIndex((i) =>
								i.id === dir.message_id
							);
							const end = idx2 + 1;
							const start = Math.max(end - dir.limit, 0);
							const s = range.slice(start, end);
							assertEq(s.end, dir.message_id);
							return s;
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
					// console.log(ranges, r);
					if (r) {
						const idx = r.items.findIndex((i) => i.id === dir.message_id);
						if (idx !== -1) {
							if (idx >= dir.limit) {
								console.log("messages reuse range for backwards");
								const end = idx + 1;
								const start = Math.max(end - dir.limit, 0);
								const s = r.slice(start, end);
								assertEq(s.end, dir.message_id);
								return s;
							}

							// fetch more
							const data = await fetchList(thread_id, {
								dir: "b",
								limit: 100,
								from: r.start,
							});
							const range = mergeBefore(ranges, r, data);
							const idx2 = range.items.findIndex((i) =>
								i.id === dir.message_id
							);
							const end = idx2 + 1;
							const start = Math.max(end - dir.limit, 0);
							const s = range.slice(start, end);
							assertEq(s.end, dir.message_id);
							return s;
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
					const data = await fetchList(thread_id, { dir: "b", limit: 100 });
					mergeBefore(ranges, range, data);
					// batch(() => {
					// 	for (const item of data.items.toReversed()) {
					// 		this.cache.set(item.id, item);
					// 		const existing = ranges.find(item.id);
					// 		if (existing) {
					// 			throw new Error("todo");
					// 		} else {
					// 			range.items.unshift(item);
					// 		}
					// 	}
					// });
					// range.has_backwards = data.has_more;
				} else {
					// don't need to do anything
				}

				const start = Math.max(range.len - dir.limit, 0);
				const end = Math.min(start + dir.limit, range.len);
				return range.slice(start, end);
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
							const end = Math.min(idx + dir.limit, r.len);
							const start = Math.max(idx - dir.limit, 0);
							const s = r.slice(start, end);
							// 	assertEq(s.end, dir.message_id);
							return s;
						}

						console.log("messages fetch more for context");
						if (!hasEnoughBackwards) {
							const data = await fetchList(thread_id, {
								dir: "b",
								limit: 100,
								from: r.start,
							});
							batch(() => {
								mergeBefore(ranges, r, data);
							});
						}

						if (!hasEnoughForwards) {
							const data = await fetchList(thread_id, {
								dir: "f",
								limit: 100,
								from: r.end,
							});
							batch(() => {
								mergeAfter(ranges, r, data);
							});
						}

						const idx2 = r.items.findIndex((i) => i.id === dir.message_id);
						const end = idx2 + 1;
						const start = Math.max(end - dir.limit, 0);
						const s = r.slice(start, end);
						return s;
					} else {
						// fetch thread
						throw new Error("todo");
					}
				} else {
					// new range
					console.log("messages fetch context");
					const data = await fetchContext(thread_id, dir.message_id, dir.limit);
					let range = new MessageRange(false, false, []);
					ranges.ranges.add(range);
					batch(() => {
						range = mergeAfter(ranges, range, {
							items: data.items,
							has_more: data.has_before,
							total: data.total,
						});
						// TODO: unify these names
						range.has_backwards = data.has_before;
						range.has_forward = data.has_after;
					});

					console.log(data, range);
					const idx = range.items.findIndex((i) => i.id === dir.message_id);
					const end = Math.min(idx + dir.limit + 1, range.len);
					const start = Math.max(idx - dir.limit, 0);
					return range.slice(start, end);
				}
			}

			throw new Error("unreachable");
		};

		const query = () => ({
			thread_id: thread_id_signal(),
			dir: dir_signal(),
		});

		const [resource, { mutate }] = createResource(query, update);

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
			type: MessageType.Default,
			id,
			thread_id,
			version_id: id,
			override_name: null,
			reply_id: null,
			content: null,
			author: this.api.users.cache.get("@self")!,
			metadata: null,
			is_pinned: false,
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
			},
		);
		if (error) throw new Error(error);
		return data;
	}

	_updateMutators(r: MessageRanges, thread_id: string) {
		for (const mut of this._mutators) {
			if (mut.thread_id !== thread_id) continue;
			if (mut.query.type !== "backwards") continue;
			if (mut.query.message_id) continue;
			const start = Math.max(r.live.len - mut.query.limit, 0);
			const end = Math.min(start + mut.query.limit, r.live.len);
			mut.mutate(r.live.slice(start, end));
		}
	}
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};
