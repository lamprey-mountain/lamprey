import {
	type Accessor,
	batch,
	createComputed,
	createEffect,
	createMemo,
	createSignal,
	For,
	on,
	onCleanup,
	untrack,
} from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";
import { createStore } from "solid-js/store";

const OVERSCAN = 30;
const ESTIMATED_H = 80;

export function createList2<
	T extends { id: string; class?: string; nonce?: string },
>(options: {
	items: Accessor<Array<T>>;
	autoscroll?: Accessor<boolean>;
	onPaginate?: (dir: "forwards" | "backwards") => void;
	onRestore?: () => boolean;
	isLoading?: Accessor<boolean>;
	containerRef?: Accessor<HTMLElement | undefined>;
}) {
	const [scrollPos, setScrollPos] = createSignal(0);
	const [isAtBottom, setIsAtBottom] = createSignal(true);
	const [visibleRange, setVisibleRange] = createSignal({ start: 0, end: 0 });

	let wrapperEl: HTMLElement | undefined;
	let containerEl: HTMLDivElement | undefined;

	let isProgrammaticScroll = false;

	// --- height + offset tracking ---
	const [heights, setHeights] = createStore<Record<string, number>>({});

	// Use nonce as the cache key if available, otherwise fall back to id
	const getItemKey = (item: T): string => item.nonce ?? item.id;

	const offsets = createMemo(() => {
		const h = heights;
		const items = options.items();
		const result = [0];
		for (let i = 1; i < items.length; i++) {
			const prevKey = getItemKey(items[i - 1]);
			const prevH = h[prevKey] ?? ESTIMATED_H;
			result[i] = result[i - 1] + prevH;
		}
		return result;
	});

	const totalHeight = createMemo(() => {
		const items = options.items();
		const lastIdx = items.length - 1;
		if (lastIdx < 0) return 0;
		const lastKey = getItemKey(items[lastIdx]);
		return offsets()[lastIdx] + (heights[lastKey] ?? ESTIMATED_H);
	});

	// --- ResizeObserver for height correction ---
	const ro = new ResizeObserver((entries) => {
		if (!wrapperEl) return;
		// console.log("a");
		let heightDiff = 0;
		let changed = false;

		// Note if we were locked to the bottom *before* adjusting layouts
		const wasAtBottom = isAtBottom();
		const currentScrollTop = wrapperEl.scrollTop;
		const currentOffsets = untrack(() => offsets());

		// 1. Accumulate updates
		const updates: Record<string, number> = {};

		for (const entry of entries) {
			const target = entry.target as HTMLElement;

			if (target === wrapperEl) {
				changed = true;
				continue;
			}

			const id = target.dataset.id;
			const nonce = target.dataset.nonce;
			if (!id) continue;

			// Use nonce as the cache key if available (for local-echo -> server transition)
			const cacheKey = nonce || id;
			// Look up the true, current index
			const idx = untrack(() => itemIndices().get(cacheKey) ?? -1);

			if (idx === -1) continue;

			const newH = Math.round(
				entry.borderBoxSize?.[0]?.blockSize ??
					target.getBoundingClientRect().height,
			);
			const oldH = heights[cacheKey] ?? ESTIMATED_H;

			if (newH > 0 && oldH !== newH) {
				const oldOffset = currentOffsets[idx] ?? 0;
				updates[cacheKey] = newH; // Accumulate instead of setHeights
				changed = true;

				// If item's original top offset is above the current viewport,
				// its expansion will push the viewport's current items down.
				// We adjust the scroll position to perfectly track the visual location.
				if (oldOffset < currentScrollTop + 1) {
					heightDiff += newH - oldH;
				}
			}
		}

		// 2. Apply reactively in a single batch
		if (Object.keys(updates).length > 0) {
			batch(() => {
				for (const k in updates) setHeights(k, updates[k]);
			});
		}

		if (changed) {
			queueMicrotask(() => {
				if (!wrapperEl) return;
				const oldPos = wrapperEl.scrollTop;

				if (wasAtBottom && options.autoscroll?.()) {
					// Maintain bottom lock if items inside/below the viewport resize (e.g., images loading)
					wrapperEl.scrollTop = wrapperEl.scrollHeight - wrapperEl.clientHeight;
				} else if (heightDiff !== 0) {
					wrapperEl.scrollTop += heightDiff;
				}

				if (wrapperEl.scrollTop !== oldPos) {
					isProgrammaticScroll = true;
				}
			});
			updateRender();
		}
	});

	// --- visible range ---
	function getVisibleRange(): { start: number; end: number } {
		if (!wrapperEl) return { start: 0, end: 0 };
		// console.log("b");
		const items = options.items();
		const scrollTop = wrapperEl.scrollTop;
		const viewportH = wrapperEl.clientHeight;
		const top = scrollTop - OVERSCAN * ESTIMATED_H;
		const bottom = scrollTop + viewportH + OVERSCAN * ESTIMATED_H;

		// binary search start
		let lo = 0,
			hi = items.length - 1;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			const key = getItemKey(items[mid]);
			if ((offsets()[mid] ?? 0) + (heights[key] ?? ESTIMATED_H) < top) {
				lo = mid + 1;
			} else hi = mid;
		}
		const start = lo;

		lo = start;
		hi = items.length - 1;
		while (lo < hi) {
			const mid = (lo + hi + 1) >> 1;
			if ((offsets()[mid] ?? 0) > bottom) hi = mid - 1;
			else lo = mid;
		}
		return { start, end: lo };
	}

	// --- render loop ---
	function updateRender() {
		if (!wrapperEl || !containerEl) return;
		// console.log("c");
		const range = getVisibleRange();
		const current = visibleRange();
		if (range.start !== current.start || range.end !== current.end) {
			setVisibleRange(range);
		}
	}

	// --- scroll handler ---
	let ticking = false;
	function onScroll() {
		if (!wrapperEl) return;
		// console.log("d");
		const pos = wrapperEl.scrollTop;
		const bottom = wrapperEl.scrollHeight - wrapperEl.clientHeight;
		setScrollPos(pos);
		setIsAtBottom(pos >= bottom - 64);

		// If this scroll event was triggered by our own code, abort pagination checks
		if (isProgrammaticScroll) {
			isProgrammaticScroll = false;
			updateRender();
			return;
		}

		if (!ticking) {
			ticking = true;
			requestAnimationFrame(() => {
				updateRender();
				if (!options.isLoading?.()) {
					if (pos < 200) options.onPaginate?.("backwards");
					if (bottom - pos < 200) options.onPaginate?.("forwards");
				}
				ticking = false;
			});
		}
	}

	// --- items change (the critical part) ---
	let prevItems: Array<T> = [];

	function onItemsChange(newItems: Array<T>) {
		if (!wrapperEl) {
			prevItems = newItems;
			return;
		}

		// console.log("e");
		const prevLen = prevItems.length;
		const newLen = newItems.length;

		const ignoreIds = new Set([
			"spacer-top",
			"thread-header",
			"spacer-bottom",
			"spacer-bottom-mini",
		]);

		let pivotPrevIdx = -1;
		let pivotNewIdx = -1;
		let isSameList = false;

		// 1. Prioritize finding a pivot in the CURRENTLY VISIBLE range so content doesn't jump
		const visStart = untrack(() => visibleRange().start);
		const startIdx = Math.max(0, Math.min(visStart, prevLen - 1));
		const oldScrollTop = wrapperEl ? wrapperEl.scrollTop : 0;

		for (let i = startIdx; i < prevLen; i++) {
			if (
				!ignoreIds.has(prevItems[i].id) &&
				!prevItems[i].id.startsWith("divider-")
			) {
				pivotPrevIdx = i;
				const prevKey = getItemKey(prevItems[i]);
				pivotNewIdx = newItems.findIndex((x) => getItemKey(x) === prevKey);
				if (pivotNewIdx !== -1) {
					isSameList = true;
					break;
				}
			}
		}

		// 2. If nothing visible matched, fallback to searching from the beginning
		if (!isSameList) {
			for (let i = 0; i < prevLen; i++) {
				if (
					!ignoreIds.has(prevItems[i].id) &&
					!prevItems[i].id.startsWith("divider-")
				) {
					pivotPrevIdx = i;
					const prevKey = getItemKey(prevItems[i]);
					pivotNewIdx = newItems.findIndex((x) => getItemKey(x) === prevKey);
					if (pivotNewIdx !== -1) {
						isSameList = true;
						break;
					}
				}
			}
		}

		if (isSameList) {
			let oldPivotOffset = 0;
			for (let i = 0; i < pivotPrevIdx; i++) {
				oldPivotOffset += heights[getItemKey(prevItems[i])] ?? ESTIMATED_H;
			}

			let newPivotOffset = 0;
			for (let i = 0; i < pivotNewIdx; i++) {
				newPivotOffset += heights[getItemKey(newItems[i])] ?? ESTIMATED_H;
			}

			// Calculate absolute target position directly via the logical offsets
			// to bypass mid-update browser scroll clamping limits.
			const relativePivotOffset = oldPivotOffset - oldScrollTop;
			const desiredScrollTop = newPivotOffset - relativePivotOffset;

			// VERY IMPORTANT: Check this BEFORE applying programmatic scroll
			const wasAtBottom = untrack(() => isAtBottom());

			queueMicrotask(() => {
				if (!wrapperEl) return;
				const oldPos = wrapperEl.scrollTop;

				if (wasAtBottom && options.autoscroll?.()) {
					// Always maintain bottom lock if we were at the bottom and autoscroll is enabled.
					wrapperEl.scrollTop = wrapperEl.scrollHeight - wrapperEl.clientHeight;
				} else if (desiredScrollTop !== oldScrollTop) {
					wrapperEl.scrollTop = desiredScrollTop;
				}

				if (wrapperEl.scrollTop !== oldPos) {
					isProgrammaticScroll = true;
				}
			});
		} else {
			// Complete channel clear/reload (usually occurs on channel swap/restore)
			setHeights({});
			if (options.onRestore) {
				setTimeout(() => {
					if (options.onRestore?.()) {
						updateRender();
					}
				}, 0);
			}
		}

		prevItems = newItems;
		updateRender();
	}

	const itemIndices = createMemo(() => {
		const map = new Map<string, number>();
		const items = options.items();
		for (let i = 0; i < items.length; i++) {
			map.set(getItemKey(items[i]), i);
		}
		return map;
	});

	return {
		scrollPos,
		isAtBottom,
		scrollBy(pos: number, smooth = false) {
			if (!wrapperEl) return;
			const oldPos = wrapperEl.scrollTop;
			wrapperEl.scrollBy({
				top: pos,
				behavior: smooth ? "smooth" : "instant",
			});
			if (wrapperEl.scrollTop !== oldPos) isProgrammaticScroll = true;
		},
		scrollTo(pos: number, smooth = false) {
			if (!wrapperEl) return;
			const oldPos = wrapperEl.scrollTop;
			wrapperEl.scrollTo({
				top: pos,
				behavior: smooth ? "smooth" : "instant",
			});
			if (wrapperEl.scrollTop !== oldPos) isProgrammaticScroll = true;
		},
		scrollToBottom(smooth = false) {
			if (!wrapperEl) return;
			const oldPos = wrapperEl.scrollTop;
			wrapperEl.scrollTo({
				top: wrapperEl.scrollHeight,
				behavior: smooth ? "smooth" : "instant",
			});
			if (wrapperEl.scrollTop !== oldPos) isProgrammaticScroll = true;
		},
		getOffset(id: string) {
			const idx = options.items().findIndex((x) => x.id === id);
			return idx !== -1 ? (offsets()[idx] ?? 0) : null;
		},
		getViewportHeight() {
			return wrapperEl?.clientHeight ?? 0;
		},
		List(listProps: {
			children: (item: T, idx: Accessor<number>) => JSX.Element;
		}) {
			createComputed(on(options.items, onItemsChange));
			onCleanup(() => ro.disconnect());

			return (
				<ul
					class="list"
					ref={(el) => {
						wrapperEl = el;
						ro.observe(el);
					}}
					onScroll={onScroll}
					style="position: relative; overflow-y: scroll; overflow-anchor: none; height: 100%; width: 100%;"
				>
					<div
						style={`width: 100%; position: relative; height: ${totalHeight()}px;`}
					>
						<div
							ref={(el) => {
								containerEl = el;
							}}
							style="position: absolute; left: 0; right: 0; top: 0;"
						>
							<For
								each={options
									.items()
									.slice(visibleRange().start, visibleRange().end + 1)}
							>
								{(item) => {
									const globalIdx = createMemo(() => {
										const key = item.nonce ?? item.id;
										return itemIndices().get(key) ?? -1;
									});

									return (
										<div
											class="list-inner"
											classList={{ [item?.class ?? ""]: !!item?.class }}
											ref={(el) => {
												el.dataset.id = item.id;
												if (item.nonce) el.dataset.nonce = item.nonce;
												ro.observe(el);
												onCleanup(() => ro.unobserve(el));
											}}
											style={{
												position: "absolute",
												left: "0",
												right: "0",
												transform: `translateY(${offsets()[globalIdx()] ?? 0}px)`,
											}}
										>
											{listProps.children(item, globalIdx)}
										</div>
									);
								}}
							</For>
						</div>
					</div>
				</ul>
			);
		},
	};
}
