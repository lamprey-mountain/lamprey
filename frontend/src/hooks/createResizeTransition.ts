import { onCleanup } from "solid-js";

export type ResizeTransitionProps = {
	height?: boolean;
	width?: boolean;
};

// PERF: consider using css `scale`?

export const createResizeTransition = (props: ResizeTransitionProps = {}) => {
	let contentRef: HTMLElement;
	let oldHeight: number | undefined;
	let oldWidth: number | undefined;
	let anim: Animation | undefined;

	const obs = new ResizeObserver((entry) => {
		if (anim) return;

		for (const e of entry) {
			const height = e.borderBoxSize[0].blockSize;
			const width = e.borderBoxSize[0].inlineSize;
			if (oldHeight !== undefined && oldWidth !== undefined) {
				anim = contentRef?.animate(
					[
						{
							height: (props.height ?? true) ? `${oldHeight}px` : undefined,
							width: (props.width ?? true) ? `${oldWidth}px` : undefined,
						},
						{
							height: (props.height ?? true) ? `${height}px` : undefined,
							width: (props.width ?? true) ? `${width}px` : undefined,
						},
					],
					{
						duration: 200,
						easing: "ease",
					},
				);
				anim.finished.then(() => {
					anim = undefined;
				});
			}
			oldHeight = height;
			oldWidth = width;
		}
	});

	onCleanup(() => {
		obs.disconnect();
		anim?.cancel();
	});

	return {
		content(el: HTMLElement) {
			contentRef = el;
		},
		container(el: HTMLElement) {
			obs.observe(el);
		},
	};
};
