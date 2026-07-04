import type { Message } from "./types";

/** sort messages and return a new message range */
function sortMessagesById(msgs: Message[]): Message[] {
	return [...msgs].sort((a, b) => (a.id < b.id ? -1 : 1));
}

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
		let idx =
			nonce !== undefined
				? items.findIndex(
						(m) =>
							("nonce" in m && (m as { nonce?: string }).nonce === nonce) ||
							m.id === nonce,
					)
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
		let mergedAny = false;
		const rangesArr = [...this.ranges]
			.filter((r) => !r.isEmpty())
			.sort((a, b) => (a.start < b.start ? -1 : 1));

		let i = 0;
		while (i < rangesArr.length - 1) {
			const a = rangesArr[i];
			const b = rangesArr[i + 1];
			if (a === undefined || b === undefined) break;
			const adjacent = !a.has_forward && !b.has_backwards;
			const overlapping = a.end >= b.start;

			if (adjacent || overlapping) {
				const fused = a.mergeRange(b);
				rangesArr.splice(i, 2, fused); // Replace the two with the fused
				if (this.live === a || this.live === b) this.live = fused;
				mergedAny = true;
			} else {
				i++;
			}
		}

		if (mergedAny) {
			this.ranges = new Set(rangesArr);
		}
		return mergedAny;
	}
}
