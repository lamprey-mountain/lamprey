// TODO: remove

import {
	type Accessor,
	createEffect,
	createMemo,
	createSignal,
	on,
} from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { onMount } from "solid-js";

const ESTIMATED_H = 80;

export function createList2<
	T extends { id: string; class?: string; nonce?: string },
>(options: {
	items: Accessor<Array<T>>;
	autoscroll?: Accessor<boolean>;
	onPaginate?: (dir: "forwards" | "backwards") => void;
	onRestore?: () => boolean;
	isLoading?: Accessor<boolean>;
	estimateSize?: () => number;
}) {
	const [wrapperEl, setWrapperEl] = createSignal<HTMLDivElement | null>(null);

	const virtualizer = createVirtualizer({
		get count() {
			console.log("count", options.items().length);
			return options.items().length;
		},
		getScrollElement: () => wrapperEl(),
		estimateSize: () => options.estimateSize?.() ?? ESTIMATED_H,
		overscan: 5,
		debug: true,
	});

	const scrollPos = createMemo(() => virtualizer.scrollOffset);
	const isAtBottom = createMemo(() => {
		const el = wrapperEl();
		if (!el) return true;
		const bottom = el.scrollHeight - el.clientHeight;
		return virtualizer.scrollOffset >= bottom - 64;
	});

	// Pagination
	createEffect(
		on(
			() => virtualizer.scrollOffset,
			(pos) => {
				if (options.isLoading?.()) return;

				const el = wrapperEl();
				if (!el) return;

				const bottom = el.scrollHeight - el.clientHeight;

				if (pos < 200) options.onPaginate?.("backwards");
				if (bottom - pos < 200) options.onPaginate?.("forwards");
			},
			{ defer: true },
		),
	);

	// Restore scroll position
	createEffect(() => {
		if (options.items().length > 0 && options.onRestore) {
			// Small delay to allow items to render/measure
			setTimeout(() => {
				options.onRestore?.();
			}, 0);
		}
	});

	return {
		scrollPos,
		isAtBottom,
		scrollBy(pos: number, smooth = false) {
			virtualizer.scrollBy(pos, { behavior: smooth ? "smooth" : "auto" });
		},
		scrollTo(pos: number, smooth = false) {
			virtualizer.scrollToOffset(pos, { behavior: smooth ? "smooth" : "auto" });
		},
		scrollToBottom(smooth = false) {
			virtualizer.scrollToOffset(virtualizer.getTotalSize(), {
				behavior: smooth ? "smooth" : "auto",
			});
		},
		getOffset(id: string) {
			const idx = options.items().findIndex((x) => x.id === id);
			if (idx === -1) return null;
			return virtualizer.getOffsetForIndex(idx);
		},
		getViewportHeight() {
			return wrapperEl()?.clientHeight ?? 0;
		},
		List(listProps: {
			children: (item: T, idx: Accessor<number>) => JSX.Element;
		}) {
			onMount(() => virtualizer._didMount());
			onMount(() => virtualizer._willUpdate());

			return (
				<div
					class="_list"
					ref={setWrapperEl}
					style="position: relative; overflow-y: scroll; overflow-anchor: none; height: 100%; width: 100%; display:block"
				>
					<div
						style={`height: ${virtualizer.getTotalSize()}px; width: 100%; position: relative;`}
					>
						{virtualizer.getVirtualItems().map((virtualRow) => {
							const item = () => options.items()[virtualRow.index];
							let el;

							createEffect(() => {
								console.log("measure", el);
								item();
								virtualizer.measureElement(el);
							});

							return (
								<div
									ref={el}
									data-index={virtualRow.index}
									style={{
										position: "absolute",
										top: 0,
										left: 0,
										width: "100%",
										transform: `translateY(${virtualRow.start}px)`,
									}}
								>
									{listProps.children(item(), () => virtualRow.index)}
								</div>
							);
						})}
					</div>
				</div>
			);
		},
	};
}
