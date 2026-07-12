import { MessageListAnchor, useMessages } from "@/api";
import {
	estimateSize,
	renderTimeline2,
	TimelineItemT2,
	VirtualItem,
	VirtualizerLayout,
} from "./util";
import { Queue } from "@/utils/queue";
import { logger } from "@/utils/logger";
import { createStore, reconcile } from "solid-js/store";
import { Accessor, createSignal } from "solid-js";
import { useTimeline } from "./timeline-context";
import { ChannelT } from "@/types";

// TODO: finetune these consts
export const PAGINATE_THRESHOLD = 800; // TODO: calculate dynamically
export const OVERSCAN = 20;
export const HIGHLIGHT_TIMEOUT = 3000;
export const STICKY_THRESHOLD = 80;

// TODO: some kind of SCROLL_EXACT task type to scroll to an exact offset?
// TODO(?): AWAIT_{LAYOUT,PAINT} tasks for sync?
export type TimelineTask =
	| { type: "SET_ANCHOR"; anchor: MessageListAnchor }
	| { type: "RESIZE"; delta: number }
	| { type: "HIGHLIGHT"; message_id: string }
	| { type: "SCROLL_TOP"; smooth?: boolean }
	| { type: "SCROLL_BOTTOM"; smooth?: boolean }
	| { type: "SCROLL_MESSAGE"; message_id: string; smooth?: boolean };

export interface TimelineVirtualizerOptions {
	scrollEl: Accessor<HTMLElement | null>;
	channel: ChannelT;
}

export interface TimelineVirtualizer {
	queue: Queue<TimelineTask>;
	highlighter: HighlighterState;
	accessTotalSize: Accessor<number>;
	accessVisibleRows: Accessor<VirtualItem[]>;
	measurements: Map<string, number>;
	refreshVisibleRows: () => void;
}

export interface HighlighterState {
	timer: null | number;
	pending: string | null;
	reset(): void;
	mark(id: string): void;
}

const log = logger.for("timeline/virtualizer");

export const createTimelineHighlighter = (): HighlighterState => {
	const [pending, setPending] = createSignal<string | null>(null);
	let timer: number | null = null;

	const reset = () => {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		setPending(null);
	};

	const mark = (id: string) => {
		if (timer) {
			clearTimeout(timer);
		}
		setPending(id);
		timer = window.setTimeout(() => {
			setPending(null);
			timer = null;
		}, HIGHLIGHT_TIMEOUT);
	};

	return {
		get timer() {
			return timer;
		},
		get pending() {
			return pending();
		},
		reset,
		mark,
	};
};

export const createTimelineVirtualizer = (
	options: TimelineVirtualizerOptions,
): TimelineVirtualizer => {
	const messages = useMessages();
	const timeline = useTimeline();
	const highlighter = createTimelineHighlighter();

	const [totalSize, setTotalSize] = createSignal(0);
	const [visibleRows, setVisibleRows] = createStore([] as VirtualItem[]);

	const measurements = new Map<string, number>();

	const getSize = (item: TimelineItemT2) => {
		let s = measurements.get(item.key);

		if (s === undefined) {
			s = estimateSize(item);
			measurements.set(item.key, s);
		}

		return s;
	};

	let cachedLayout: VirtualizerLayout;
	let cachedRange: Array<VirtualItem>;

	// PERF: cache calculateLayout and calculateRange more aggressively
	// solidjs makes fine grained deps easy, but im not sure if i can use solid here? at least for something where timing is this critical.

	const calculateLayout = (): VirtualizerLayout => {
		const items = timeline.items;
		const el = options.scrollEl();
		const containerHeight = el ? el.clientHeight : 0;

		// calculate sizes and offsets
		const sizes = new Float64Array(items.length);
		const offsets = new Float64Array(items.length);
		let totalSize = 0;
		for (let i = 0; i < items.length; i++) {
			const s = getSize(items[i]);
			sizes[i] = s;
			offsets[i] = totalSize;
			totalSize += s;
		}

		let finalTotalSize = totalSize;
		if (totalSize < containerHeight) {
			const offset = containerHeight - totalSize;
			for (let i = 0; i < offsets.length; i++) {
				offsets[i] += offset;
			}
			finalTotalSize = containerHeight;
		}

		return { sizes, offsets, totalSize: finalTotalSize };
	};

	const calculateRange = (): Array<VirtualItem> => {
		const el = options.scrollEl();
		if (!el) throw new Error("scrollEl() is null");

		const items = timeline.items;
		const { sizes, offsets } = cachedLayout;

		// calculate visible items
		const st = el.scrollTop;
		const vh = el.clientHeight;

		// PERF: use binary search
		let start = offsets.findIndex((o) => o > st);
		start = start === -1 ? items.length - 1 : start - 1;

		let end = offsets.findIndex((o) => o > st + vh);
		end = end === -1 ? items.length - 1 : end - 1;

		start = Math.max(0, start - OVERSCAN);
		end = Math.min(items.length - 1, end + OVERSCAN);

		const visible: Array<VirtualItem> = [];
		for (let i = start; i <= end; i++) {
			visible.push({
				index: i,
				item: items[i],
				offset: offsets[i],
				size: sizes[i],
				key: items[i].key,
			});
		}

		return visible;
	};

	// TEMP: compat
	const scrollEl = options.scrollEl;

	const queue = new Queue(async (task: TimelineTask) => {
		log.debug("execute task", task);

		switch (task.type) {
			case "SET_ANCHOR": {
				// 1. fetch messages
				// 2. recalculate/update layout
				// 3. recalculate/update range
				// 4. wait for solidjs to update the dom
				// 5. stabilize scroll position

				highlighter.reset();

				const el = scrollEl();
				if (!el) return;

				// TODO: show skeletons while fetching messages?
				const messageRange = await messages.fetchSlice(
					options.channel.id,
					task.anchor,
				);

				// update timeline
				const rendered = renderTimeline2(
					messageRange,
					timeline.readMarkerId ?? null,
				);

				log.debug("update timeline", rendered);

				// pick a reference item: prefer the anchor's message_id, else first visible item
				// const refKey =
				// 	("message_id" in task.anchor &&
				// 		`message-${task.anchor.message_id}`) ||
				// 	range()[0]?.item.key;
				const refKey =
					("message_id" in task.anchor &&
						`message-${task.anchor.message_id}`) ??
					cachedRange?.find((i) => i.item.type === "message")?.item.key;
				// FIXME: anchor backwards with no message id should probably use range().lastIndexOf
				// or maybe i should pick the center item if possible?

				// PERF: use binary search
				const refOffsetOld = refKey
					? cachedLayout?.offsets[
							timeline.items.findIndex((x) => x.key === refKey)
						]
					: undefined;

				timeline.anchor = task.anchor;
				timeline.messages = messageRange;
				timeline.items = rendered;

				cachedLayout = calculateLayout();
				cachedRange = calculateRange();
				setTotalSize(cachedLayout.totalSize);
				setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));

				// wait for solidjs to update dom
				await new Promise<void>((r) => queueMicrotask(r));

				// wait for layout but before paint
				// await new Promise((r) => requestAnimationFrame(r));

				if (el && refOffsetOld) {
					// PERF: use binary search
					const newIdx = timeline.items.findIndex((x) => x.key === refKey);
					if (newIdx !== -1) {
						const refOffsetNew = calculateLayout().offsets[newIdx];
						log.debug("stabilize scroll", {
							refKey,
							refOffsetOld,
							refOffsetNew,
						});
						el.scrollTop += refOffsetNew - refOffsetOld;
					}
				}

				break;
			}
			case "RESIZE": {
				// wait for layout but before paint
				// await new Promise((r) => requestAnimationFrame(r));

				const el = scrollEl();
				if (!el) return;

				el.scrollBy({ top: task.delta, behavior: "instant" });

				const isStuck =
					el.scrollHeight - el.scrollTop - el.clientHeight < STICKY_THRESHOLD;

				cachedLayout = calculateLayout();
				cachedRange = calculateRange();
				setTotalSize(cachedLayout.totalSize);
				setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));

				// TODO: if an element resizes below the current viewport, don't call scrollBy()

				// maybe overflow-anchor can keep stuff stable
				// maybe i'd need to disable overflow-anchor during SET_ANCHOR

				if (isStuck) {
					el.scrollTo({ top: el.scrollHeight, behavior: "instant" });
				}

				break;
			}
			case "HIGHLIGHT": {
				highlighter.mark(task.message_id);
				break;
			}
			case "SCROLL_TOP": {
				scrollEl()?.scrollTo({
					top: 0,
					behavior: task.smooth ? "smooth" : "auto",
				});
				break;
			}
			case "SCROLL_BOTTOM": {
				const el = scrollEl();
				if (el) {
					el.scrollTo({
						top: el.scrollHeight,
						behavior: task.smooth ? "smooth" : "auto",
					});
				}
				break;
			}
			case "SCROLL_MESSAGE": {
				// PERF: binary search, check timeline.messages (MessageRange)
				const idx = timeline.items.findIndex(
					(x) => x.type === "message" && x.message.id === task.message_id,
				);
				if (idx !== -1) {
					const el = options.scrollEl();
					if (!el) break;

					const { offsets, sizes } = calculateLayout();
					// align message to center of view
					const vh = el.clientHeight;
					const itemOffset = offsets[idx];
					const itemSize = sizes[idx];
					const targetTop = Math.max(0, itemOffset - vh / 2 + itemSize / 2);

					scrollEl()?.scrollTo({
						top: targetTop,
						behavior: task.smooth ? "smooth" : "auto",
					});
				} else {
					log.warn("couldn't find message for SCROLL_MESSAGE", task);
				}
				break;
			}
		}
	});

	return {
		queue,
		highlighter,
		accessTotalSize: totalSize,
		accessVisibleRows: () => visibleRows,
		measurements,
		refreshVisibleRows: () => {
			// PERF: skip updating visible rows if cachedRange didn't change
			cachedRange = calculateRange();
			setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));
		},
	};
};
