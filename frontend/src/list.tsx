import { JSX } from "solid-js/jsx-runtime";
import { For, on, onMount } from "solid-js";
import {
	Accessor,
	createComputed,
	createEffect,
	createSignal,
	onCleanup,
} from "solid-js";
// import { throttle } from "@solid-primitives/scheduled";

export type TimelineStatus = "loading" | "update" | "ready";

export type SliceInfo = {
	start: number;
	end: number;
};

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
}) {
	const [wrapperEl, setWrapperEl] = createSignal<HTMLElement>();
	const [topEl, setTopEl] = createSignal<HTMLElement>();
	const [bottomEl, setBottomEl] = createSignal<HTMLElement>();
	const [isAtBottom, setIsAtBottom] = createSignal(false); // FIXME: should only be true if at slice end
	const [scrollPos, setScrollPos] = createSignal(0);
	let topRef: HTMLElement | undefined;
	let bottomRef: HTMLElement | undefined;
	let anchorRef: Element;
	let anchorRect: DOMRect;

	const margin = 0;
	const intersections = new IntersectionObserver((entries) => {
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
	}, {
		rootMargin: `${margin}px 0px ${margin}px 0px`,
	});

	const resizes = new ResizeObserver((_entries) => {
		// NOTE: fine for instantaneous resizes, janky when trying to smoothly resize
		if (isAtBottom() && options.autoscroll?.() || false) {
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

	onCleanup(() => {
		intersections.disconnect();
		resizes.disconnect();
	});

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
				const wrap = wrapperEl();
				const shouldAutoscroll = isAtBottom() &&
					(options.autoscroll?.() || false);
				if (!wrap || !anchorRef) return setRefs();
				if (shouldAutoscroll) {
					wrap.scrollTo({ top: 999999, behavior: "instant" });
				} else {
					// FIXME: tons of reflow and jank
					const currentRect = anchorRef.getBoundingClientRect();
					const diff = (currentRect.y - anchorRect.y) +
						(currentRect.height - anchorRect.height);
					wrapperEl()?.scrollBy(0, diff);
				}
				setRefs();
			}

			createComputed(on(options.items, () => {
				anchorRect = anchorRef?.getBoundingClientRect();
			}));

			createEffect(on(options.items, () => {
				queueMicrotask(reanchor);
				// reanchor()
			}));

			createEffect(on(topEl, (topEl) => {
				if (!topEl) return;
				if (topRef) intersections.unobserve(topRef);
				topRef = topEl;
				intersections.observe(topEl);
			}));

			createEffect(on(bottomEl, (bottomEl) => {
				if (!bottomEl) return;
				if (bottomRef) intersections.unobserve(bottomRef);
				bottomRef = bottomEl;
				intersections.observe(bottomEl);
			}));

			onMount(() => {
				resizes.observe(wrapperEl()!);
			});

			function handleScroll() {
				const pos = wrapperEl()!.scrollTop;
				setScrollPos(pos);
				setIsAtBottom(
					pos >= (wrapperEl()!.scrollHeight - wrapperEl()!.offsetHeight),
				);
				// options.onScroll?.(pos);
			}

			// TODO: onScrollEnd might be useful
			// TODO: set passive: true on scroll event
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
