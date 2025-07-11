import type { JSX } from "solid-js/jsx-runtime";
import { createComputed, For, on } from "solid-js";
import { type Accessor, createEffect, createSignal } from "solid-js";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { createResizeObserver } from "@solid-primitives/resize-observer";
// import { throttle } from "@solid-primitives/scheduled";

// export type TimelineStatus = "loading" | "update" | "ready";

// export type SliceInfo = {
// 	start: number;
// 	end: number;
// };

// TODO: dynamically calculate how many events are needed
// const SLICE_COUNT = 100;
// const PAGINATE_COUNT = SLICE_COUNT * 3;
// const PAGINATE_COUNT = SLICE_COUNT;
// const AUTOSCROLL_MARGIN = 5;
// const SCROLL_MARGIN = 100;
// const PAGINATE_MARGIN = SCROLL_MARGIN + 50;

// export function shouldSplit(msg: Event, prev?: Event) {
//   if (!prev) return true;
//   if (msg.sender !== prev.sender) return true;
//   if (msg.originTs - prev.originTs > 1000 * 60 * 5) return true;
//   return false;
// }

// /** A list that retains its scroll position when items are added/removed */
// export function StableList() {}

export function createList<T>(options: {
	items: Accessor<Array<T>>;
	autoscroll?: Accessor<boolean>;
	// topPos?: Accessor<number>,
	// bottomPos?: Accessor<number>,
	topQuery: string;
	bottomQuery: string;
	onPaginate?: (dir: "forwards" | "backwards") => void;
	// onScroll?: (pos: number) => void;
	onContextMenu?: (e: MouseEvent) => void;
	onRestore?: () => boolean;
	containerRef?: Accessor<HTMLElement | undefined>;
}) {
	const [wrapperEl, setWrapperEl] = createSignal<HTMLElement>();

	createEffect(() => {
		if (options.containerRef) {
			setWrapperEl(options.containerRef());
		}
	});
	const [topEl, setTopEl] = createSignal<HTMLElement>();
	const [bottomEl, setBottomEl] = createSignal<HTMLElement>();
	const [isAtBottom, setIsAtBottom] = createSignal(true); // FIXME: should only be true if at slice end
	const [scrollPos, setScrollPos] = createSignal(0);
	let anchorRef: Element;
	let anchorRect: DOMRect;

	createIntersectionObserver(
		() => [topEl(), bottomEl()].filter((i) => i) as Element[],
		handleIntersections,
	);

	function handleIntersections(entries: IntersectionObserverEntry[]) {
		// PERF: run intersection callback takes too long
		for (const el of entries) {
			if (el.target === topEl()) {
				if (el.isIntersecting) {
					anchorRef = el.target;
					options.onPaginate?.("backwards");
				}
			} else if (el.target === bottomEl()) {
				if (el.isIntersecting) {
					anchorRef = el.target;
					options.onPaginate?.("forwards");
				}
			}
		}
	}

	createResizeObserver(wrapperEl, () => {
		// NOTE: fine for instantaneous resizes, janky when trying to smoothly resize
		if (isAtBottom() && options.autoscroll?.() || false) {
			console.log("autoscroll on resize");
			wrapperEl()!.scrollTo({ top: 999999, behavior: "instant" });
		}
	});

	function setRefs() {
		const newTopEl = wrapperEl()!.querySelector(
			options.topQuery,
		)! as HTMLElement;
		const newBottomEl = wrapperEl()!.querySelector(
			options.bottomQuery,
		)! as HTMLElement;
		setTopEl(newTopEl);
		setBottomEl(newBottomEl);
	}

	return {
		scrollPos,
		isAtBottom,
		scrollBy(pos: number, smooth = false) {
			wrapperEl()?.scrollBy({
				top: pos,
				behavior: smooth ? "smooth" : "instant",
			});
		},
		scrollTo(pos: number, smooth = false) {
			wrapperEl()?.scrollTo({
				top: pos,
				behavior: smooth ? "smooth" : "instant",
			});
		},
		List(props: { children: (item: T, idx: Accessor<number>) => JSX.Element }) {
			function reanchor() {
				console.log("do reanchor");
				const wrap = wrapperEl();
				const shouldAutoscroll = isAtBottom() &&
					(options.autoscroll?.() || false);
				if (!wrap || options.onRestore?.()) return setRefs();
				if (shouldAutoscroll) {
					console.log("autoscrolled");
					wrap.scrollTo({ top: 999999, behavior: "instant" });
				} else if (anchorRef && wrap.contains(anchorRef)) {
					// FIXME: don't force reflow; this casuses jank
					const currentRect = anchorRef.getBoundingClientRect();
					const diff = (currentRect.y - anchorRect.y) +
						(currentRect.height - anchorRect.height);
					console.log("reanchored", anchorRect, currentRect, diff);
					wrapperEl()?.scrollBy(0, diff);
				}
				setRefs();
			}

			createComputed(on(options.items, () => {
				console.log("begin reanchor");
				anchorRect = anchorRef?.getBoundingClientRect();
				queueMicrotask(reanchor);
			}));

			createEffect(on(topEl, (topEl) => {
				if (!topEl) return;
				setTopEl(topEl);
			}));

			createEffect(on(bottomEl, (bottomEl) => {
				if (!bottomEl) return;
				setBottomEl(bottomEl);
			}));

			function handleScroll() {
				const pos = wrapperEl()!.scrollTop;
				const bottom = wrapperEl()!.scrollHeight - wrapperEl()!.offsetHeight;
				setScrollPos(pos);
				setIsAtBottom(pos >= bottom);
				// TODO: maybe use css + trigger elements?
				if (pos >= bottom - 200) {
					options.onPaginate?.("forwards");
				} else if (pos < 200) {
					options.onPaginate?.("backwards");
				}
				// options.onScroll?.(pos);
			}

			// TODO: onScrollEnd might be useful
			// TODO: set passive: true on scroll event?
			return (
				<ul
					class="list"
					ref={setWrapperEl}
					onScroll={handleScroll}
					onContextMenu={options.onContextMenu}
				>
					<For each={options.items()}>
						{(item, idx) => props.children(item, idx)}
					</For>
				</ul>
			);
		},
	};
}
