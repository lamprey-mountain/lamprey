import { MessageListAnchor } from "@/api";
import { TimelineItemT2 } from "./util";
import { Queue } from "@/utils/queue";

export interface VirtualItem {
	index: number;
	item: TimelineItemT2;
	offset: number;
	size: number;
}

export interface Layout {
	sizes: Float64Array;
	offsets: Float64Array;
	totalSize: number;
}

// TODO: some kind of SCROLL_EXACT task type to scroll to an exact offset?
// TODO(?): AWAIT_{LAYOUT,PAINT} tasks for sync?
export type TimelineTask =
	| { type: "SET_ANCHOR"; anchor: MessageListAnchor }
	| { type: "RESIZE"; delta: number }
	| { type: "HIGHLIGHT"; message_id: string }
	| { type: "SCROLL_TOP"; smooth?: boolean }
	| { type: "SCROLL_BOTTOM"; smooth?: boolean }
	| { type: "SCROLL_MESSAGE"; message_id: string; smooth?: boolean };

export type TimelineVirtualizerOptions = {
	scrollEl: HTMLElement;
};

export type TimelineVirtualizer = {
	queue: Queue<TimelineTask>;
	// reactive getters for layout, range
};

export const createTimelineVirtualizer = (
	options: TimelineVirtualizerOptions,
) => {
	let layout: Layout;
	let range: Array<VirtualItem>;
	const queue = new Queue(async (task: TimelineTask) => {
		// TODO
	});

	// options.scrollEl.scrollTo({
	// 	top: 0,
	// 	behavior: "instant"
	// });

	return {
		queue,
	};
};
