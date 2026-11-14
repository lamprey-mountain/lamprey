import { type Accessor, createSignal, onCleanup } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { type MessageListAnchor, useMessages } from "@/api";
import type { ChannelT } from "@/types";
import { logger } from "@/utils/logger";
import { Queue } from "@/utils/queue";
import { useTimeline } from "./timeline-context";
import {
	estimateSize,
	renderTimeline2,
	type TimelineItemT2,
	type VirtualItem,
	type VirtualizerLayout,
} from "./util";

// TODO: finetune these consts
export const PAGINATE_THRESHOLD = 800; // TODO: calculate dynamically
export const OVERSCAN = 20;
export const HIGHLIGHT_TIMEOUT = 3000;
export const STICKY_THRESHOLD = 80;
/** how long a scroll target is kept alive after landing on it */
export const SCROLL_TARGET_TTL = 500;
/** quiet time after the last scroll event that counts as "settled" */
export const SCROLL_SETTLE_IDLE_MS = 100;
/** how long to wait for a scroll to start before giving up on it */
export const SCROLL_SETTLE_START_MS = 100;
/** hard cap for waiting on a single scroll to settle */
export const SCROLL_SETTLE_TIMEOUT_MS = 1000;
/** fallback for WAIT when requestAnimationFrame is paused (hidden tab) */
export const WAIT_FALLBACK_MS = 250;

// TODO(?): AWAIT_{LAYOUT,PAINT} tasks for sync?
export type TimelineTask =
	| {
			type: "SET_ANCHOR";
			anchor: MessageListAnchor;
			stabilize_message_id?: string;
	  }
	| { type: "RESIZE" }
	| { type: "HIGHLIGHT"; message_id: string }
	| { type: "SCROLL_TOP"; smooth?: boolean }
	| { type: "SCROLL_BOTTOM"; smooth?: boolean }
	| { type: "SCROLL_MESSAGE"; message_id: string; smooth?: boolean }
	| { type: "SCROLL_EXACT"; top: number }
	| { type: "SCROLL_BY"; px: number; smooth?: boolean }
	| { type: "WAIT"; frames?: number };

export interface TimelineVirtualizerOptions {
	scrollEl: Accessor<HTMLElement | null>;
	channel: ChannelT;
}

export interface TimelineScrollState {
	/** a jump is queued: suppress scroll compensation from height changes */
	pending: boolean;
	/** a smooth scroll is in flight: pagination may still be requested */
	settling: boolean;
	/** message to re-center on after height changes */
	target: null | { message_id: string };
}

export interface TimelineVirtualizer {
	queue: Queue<TimelineTask>;
	highlighter: HighlighterState;
	accessLoading: Accessor<boolean>;
	accessTotalSize: Accessor<number>;
	accessVisibleRows: Accessor<VirtualItem[]>;
	measurements: Map<string, number>;
	scrollState: TimelineScrollState;
	/** mark a jump as queued (suppresses resize compensation) */
	beginJump: () => void;
	/** drop pending/target scroll state, e.g. when the user takes over */
	clearScrollState: () => void;
	refreshVisibleRows: () => void;
	getIndexAtScrollTop: (scrollTop: number) => number;
}

export interface HighlighterState {
	timer: null | number;
	pending: string | null;
	reset(): void;
	mark(id: string): void;
}

const log = logger.for("timeline/virtualizer");

type ScrollSettleReason = "scrollend" | "idle" | "timeout" | "no-movement";

/**
 * resolve once a scroll (usually smooth) has stopped moving. resolves early with
 * "no-movement" if nothing starts within SCROLL_SETTLE_START_MS.
 */
function waitForScrollSettle(el: HTMLElement): Promise<ScrollSettleReason> {
	return new Promise((resolve) => {
		let started = false;
		let settled = false;
		let idleTimer: number | undefined;
		let startTimer: number | undefined;
		let timeoutTimer: number | undefined;

		const finish = (reason: ScrollSettleReason) => {
			if (settled) return;
			settled = true;
			el.removeEventListener("scrollend", onScrollEnd);
			el.removeEventListener("scroll", onScroll);
			if (idleTimer !== undefined) window.clearTimeout(idleTimer);
			if (startTimer !== undefined) window.clearTimeout(startTimer);
			if (timeoutTimer !== undefined) window.clearTimeout(timeoutTimer);
			resolve(reason);
		};

		const onScroll = () => {
			if (settled) return;
			started = true;
			if (startTimer !== undefined) window.clearTimeout(startTimer);
			startTimer = undefined;
			if (idleTimer !== undefined) window.clearTimeout(idleTimer);
			// fallback for engines without scrollend
			idleTimer = window.setTimeout(
				() => finish("idle"),
				SCROLL_SETTLE_IDLE_MS,
			);
		};

		// ignore stale scrollend events from a previous scroll
		const onScrollEnd = () => {
			if (started) finish("scrollend");
		};

		el.addEventListener("scrollend", onScrollEnd);
		el.addEventListener("scroll", onScroll, { passive: true });
		startTimer = window.setTimeout(
			() => finish("no-movement"),
			SCROLL_SETTLE_START_MS,
		);
		timeoutTimer = window.setTimeout(
			() => finish("timeout"),
			SCROLL_SETTLE_TIMEOUT_MS,
		);
	});
}

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
	const scrollState: TimelineScrollState = {
		pending: false,
		settling: false,
		target: null,
	};
	let scrollTargetTimer: null | number = null;
	const clearScrollTarget = () => {
		scrollState.target = null;
		if (scrollTargetTimer !== null) {
			clearTimeout(scrollTargetTimer);
			scrollTargetTimer = null;
		}
	};
	const clearScrollState = () => {
		scrollState.pending = false;
		clearScrollTarget();
	};
	const beginJump = () => {
		scrollState.pending = true;
		clearScrollTarget();
	};
	onCleanup(clearScrollTarget);

	const [loading, setLoading] = createSignal(true);
	const [totalSize, setTotalSize] = createSignal(0);
	const [visibleRows, setVisibleRows] = createStore([] as VirtualItem[]);

	let hasLoaded = false;
	const measurements = new Map<string, number>();

	const settleScroll = async (el: HTMLElement) => {
		scrollState.settling = true;
		try {
			return await waitForScrollSettle(el);
		} finally {
			scrollState.settling = false;
		}
	};

	const getSize = (item: TimelineItemT2) => {
		let s = measurements.get(item.key);

		if (s === undefined) {
			s = estimateSize(item);
			measurements.set(item.key, s);
		}

		return s;
	};

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

	let cachedLayout: VirtualizerLayout = calculateLayout();
	let cachedRange: Array<VirtualItem>;

	// PERF: cache calculateLayout and calculateRange more aggressively
	// solidjs makes fine grained deps easy, but im not sure if i can use solid here? at least for something where timing is this critical.

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

	const indexAtScrollTop = (layout: VirtualizerLayout, scrollTop: number) => {
		const { offsets } = layout;
		if (offsets.length === 0) return -1;

		let idx = offsets.findIndex((o) => o > scrollTop);
		idx = idx === -1 ? offsets.length - 1 : idx - 1;
		return Math.max(0, idx);
	};

	const queue = new Queue(async (task: TimelineTask) => {
		log.debug("execute task", task);

		switch (task.type) {
			case "SET_ANCHOR": {
				// 1. fetch messages
				// 2. recalculate/update layout
				// 3. recalculate/update range
				// 4. wait for solidjs to update the dom
				// 5. stabilize scroll position

				const el = scrollEl();
				if (!el) return;

				// TODO: show skeletons while fetching initial messages
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
				const fallbackRefKey =
					("message_id" in task.anchor &&
						`message-${task.anchor.message_id}`) ??
					cachedRange?.find((i) => i.item.type === "message")?.item.key;
				const refKey = task.stabilize_message_id
					? `message-${task.stabilize_message_id}`
					: fallbackRefKey;
				// FIXME: anchor backwards with no message id should probably use range().lastIndexOf
				// or maybe i should pick the center item if possible?

				// PERF: use binary search
				// read the old offset here, not before the fetch: RESIZE tasks that ran
				// meanwhile already compensated their own height changes and updated
				// cachedLayout, so counting them again would double the correction
				const refIdxOld = refKey
					? timeline.items.findIndex((x) => x.key === refKey)
					: -1;
				const refOffsetOld =
					refIdxOld !== -1 ? cachedLayout.offsets[refIdxOld] : undefined;

				timeline.anchor = task.anchor;
				timeline.messages = messageRange;
				timeline.items = rendered;

				cachedLayout = calculateLayout();
				cachedRange = calculateRange();
				setTotalSize(cachedLayout.totalSize);
				setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));

				if (!hasLoaded) {
					hasLoaded = true;
					setLoading(false);
				}

				// wait for solidjs to update dom
				await new Promise<void>((r) => queueMicrotask(r));

				// wait for layout but before paint
				// await new Promise((r) => requestAnimationFrame(r));

				if (el && refOffsetOld !== undefined) {
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

				timeline.events.emit("paginate", task.anchor);

				break;
			}
			case "RESIZE": {
				// wait for layout but before paint
				// await new Promise((r) => requestAnimationFrame(r));

				const el = scrollEl();
				if (!el) return;

				// keep the top visible item pinned so height changes above the
				// viewport don't shift content. derived from the layout diff, so it
				// is idempotent and never double counts SET_ANCHOR's correction
				const prevLayout = cachedLayout;
				const prevIdx = indexAtScrollTop(prevLayout, el.scrollTop);
				const refKey = timeline.items[prevIdx]?.key;
				const refOffsetOld =
					prevIdx === -1 ? undefined : prevLayout.offsets[prevIdx];

				cachedLayout = calculateLayout();
				cachedRange = calculateRange();
				setTotalSize(cachedLayout.totalSize);
				setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));

				const compensate = () => {
					if (refKey === undefined || refOffsetOld === undefined) return;
					const nextIdx = timeline.items.findIndex((x) => x.key === refKey);
					if (nextIdx !== -1) {
						el.scrollTop += cachedLayout.offsets[nextIdx] - refOffsetOld;
					}
				};

				if (scrollState.pending) {
					// a queued jump owns the scroll position
				} else if (scrollState.target) {
					// keep the jumped-to message centered as heights change
					const target = scrollState.target;
					const idx = timeline.items.findIndex(
						(x) => x.type === "message" && x.message.id === target.message_id,
					);
					if (idx === -1) {
						log.warn("RESIZE lost its scroll target", target);
						clearScrollTarget();
						compensate();
					} else {
						const vh = el.clientHeight;
						const targetTop = Math.max(
							0,
							cachedLayout.offsets[idx] - vh / 2 + cachedLayout.sizes[idx] / 2,
						);
						if (Math.abs(targetTop - el.scrollTop) > 1) {
							el.scrollTo({ top: targetTop, behavior: "instant" });
						}
					}
				} else {
					compensate();
				}

				// TODO: if an element resizes below the current viewport, don't call scrollBy()

				// maybe overflow-anchor can keep stuff stable
				// maybe i'd need to disable overflow-anchor during SET_ANCHOR

				const isStuck =
					el.scrollHeight - el.scrollTop - el.clientHeight < STICKY_THRESHOLD;
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
				const el = scrollEl();
				if (el) {
					clearScrollState();
					el.scrollTo({
						top: 0,
						behavior: task.smooth ? "smooth" : "auto",
					});
					if (task.smooth) await settleScroll(el);
				}
				break;
			}
			case "SCROLL_BOTTOM": {
				const el = scrollEl();
				if (el) {
					clearScrollState();
					el.scrollTo({
						top: el.scrollHeight,
						behavior: task.smooth ? "smooth" : "auto",
					});
					if (task.smooth) await settleScroll(el);
				}
				break;
			}
			case "SCROLL_MESSAGE": {
				clearScrollState();
				// PERF: binary search, check timeline.messages (MessageRange)
				const idx = timeline.items.findIndex(
					(x) => x.type === "message" && x.message.id === task.message_id,
				);
				if (idx === -1) {
					log.warn("couldn't find message for SCROLL_MESSAGE", task);
					break;
				}

				const el = options.scrollEl();
				if (!el) break;

				const { offsets, sizes } = calculateLayout();
				// align message to center of view
				const vh = el.clientHeight;
				const itemOffset = offsets[idx];
				const itemSize = sizes[idx];
				const targetTop = Math.max(0, itemOffset - vh / 2 + itemSize / 2);

				if (Math.abs(targetTop - el.scrollTop) > 1) {
					// track the target so later height changes re-center it instead of
					// pushing it out of view
					scrollState.target = { message_id: task.message_id };

					el.scrollTo({
						top: targetTop,
						behavior: task.smooth ? "smooth" : "auto",
					});

					if (task.smooth) {
						let reason = await settleScroll(el);
						if (reason === "no-movement") {
							el.scrollTo({ top: targetTop, behavior: "smooth" });
							reason = await settleScroll(el);
						}
						if (reason === "no-movement") {
							log.warn(
								"SCROLL_MESSAGE smooth scroll did not start; scrolling instantly",
								{
									message_id: task.message_id,
									scrollTop: el.scrollTop,
									targetTop,
								},
							);
							el.scrollTo({ top: targetTop, behavior: "auto" });
						}
					}

					if (scrollTargetTimer !== null) clearTimeout(scrollTargetTimer);
					scrollTargetTimer = window.setTimeout(() => {
						scrollTargetTimer = null;
						scrollState.target = null;
					}, SCROLL_TARGET_TTL);
				}
				break;
			}
			case "SCROLL_EXACT": {
				const el = scrollEl();
				if (!el) break;

				clearScrollState();
				const maxTop = Math.max(0, el.scrollHeight - el.clientHeight);
				el.scrollTo({
					top: Math.min(Math.max(0, task.top), maxTop),
					behavior: "auto",
				});
				break;
			}
			case "SCROLL_BY": {
				const el = scrollEl();
				if (el) {
					clearScrollState();
					el.scrollBy({
						top: task.px,
						behavior: task.smooth ? "smooth" : "auto",
					});
					if (task.smooth) await settleScroll(el);
				}
				break;
			}
			case "WAIT": {
				// let the browser commit pending dom updates before continuing
				const frames = Math.max(1, task.frames ?? 1);
				await new Promise<void>((resolve) => {
					let remaining = frames;
					let timer: number | undefined;
					const finish = () => {
						if (timer !== undefined) window.clearTimeout(timer);
						resolve();
					};
					const step = () => {
						remaining--;
						if (remaining <= 0) finish();
						else requestAnimationFrame(step);
					};
					requestAnimationFrame(step);
					// hidden tabs pause rAF: don't stall the queue forever
					timer = window.setTimeout(finish, WAIT_FALLBACK_MS);
				});
				await new Promise<void>((r) => window.setTimeout(r, 0));
				break;
			}
		}
	});

	return {
		queue,
		highlighter,
		accessLoading: loading,
		accessTotalSize: totalSize,
		accessVisibleRows: () => visibleRows,
		measurements,
		scrollState,
		beginJump,
		clearScrollState,
		refreshVisibleRows: () => {
			// PERF: skip updating visible rows if cachedRange didn't change
			cachedRange = calculateRange();
			setVisibleRows(reconcile(cachedRange, { key: "key", merge: true }));
		},
		getIndexAtScrollTop: (st: number) => indexAtScrollTop(cachedLayout, st),
	};
};
