import { Message } from "sdk";
import { Resource } from "solid-js";

export class Range {
	constructor(
		public has_forward: boolean,
		public has_backwards: boolean,
		public items: Array<Message>,
	) {}

	isEmpty(): boolean {
		return this.items.length === 0;
	}

	/** Requires at least one item */
	start(): string {
		return this.items[0]!.id;
	}

	/** Requires at least one item */
	end(): string {
		return this.items.at(-1)!.id;
	}

	contains(message_id: string): boolean {
		if (this.isEmpty()) return false;
		return message_id >= this.start() && message_id <= this.end();
	}
}

export class Ranges {
	live = new Range(false, true, []);
	ranges: Array<Range> = [this.live];
	resources = new Set<Resource<Array<Message>>>();

	find(message_id: string): Range | null {
		for (const range of this.ranges) {
			if (range.contains(message_id)) return range;
		}
		return null;
	}
}

// export class ThreadMessages {
// 	public ranges = new Ranges();

// 	constructor() {
		
// 	}
// }

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
