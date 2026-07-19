import {
	arrow,
	autoUpdate,
	flip,
	type Middleware,
	offset,
	type Padding,
	type Placement,
	shift,
} from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import {
	type Accessor,
	createEffect,
	createSignal,
	type JSX,
	type JSXElement,
	onCleanup,
	onMount,
	type Ref,
	Show,
	type ValidComponent,
} from "solid-js";
import { Portal, render } from "solid-js/web";

// WARNING: this is potentially very laggy
// TODO: defer tooltip
type TooltipProps = {
	tipText?: string;
	attrs?: Record<string, string>;
	interactive?: boolean;
	placement?: Placement;
	animGroup?: string;
	doesntRetain?: string;
	mount?: HTMLElement;

	// https://floating-ui.com/docs/detectoverflow#altboundary
	altBoundary?: boolean;

	arrow?: boolean;
};

type TooltipAnimState = {
	shouldAnim: boolean;
	timeout: NodeJS.Timeout;
};

const tooltipAnimSuppress = new Map<string, TooltipAnimState>();

// TODO: only use one tooltip + event listener instead of per element
// or, debug performance issues in geneal
export function tooltip(
	props: TooltipProps,
	tip: ValidComponent,
	wrap: HTMLElement,
) {
	const [contentEl, setContentEl] = createSignal<HTMLElement>();
	const [tipEl, setTipEl] = createSignal<HTMLDivElement>();
	// const [title, setTitle] = createSignal(getTitle());
	const [visible, setVisible] = createSignal(false);
	const [animate, setAnimate] = createSignal(true);
	let popupRemoveTimeout: NodeJS.Timeout;
	let isHovered = false;
	const overlayEl = document.getElementById("overlay")!;
	const padding = () => 8;

	if (props.animGroup) {
		const s = tooltipAnimSuppress.get(props.animGroup);
		if (!s) {
			tooltipAnimSuppress.set(props.animGroup, {
				shouldAnim: true,
				timeout: 0 as unknown as NodeJS.Timeout,
			});
		}
	}

	function getTitle() {
		return props.tipText ?? (typeof tip === "string" ? tip : "");
	}

	function showTip() {
		clearTimeout(popupRemoveTimeout);
		isHovered = true;
		if (visible()) return;
		if (props.animGroup) {
			const s = tooltipAnimSuppress.get(props.animGroup)!;
			// console.log(s);
			setAnimate(s.shouldAnim);
			s.shouldAnim = false;
			clearTimeout(s.timeout);
		}
		setVisible(true);
		wrap.title = "";
	}

	function hideTip() {
		// TODO: exit animations? might be too much
		wrap.title = getTitle();
		setVisible(false);
		if (props.animGroup) {
			const s = tooltipAnimSuppress.get(props.animGroup)!;
			s.timeout = setTimeout(() => {
				s.shouldAnim = true;
			}, 500);
		}
		isHovered = false;
	}

	function considerHidingTip() {
		// FIXME: nested popups/tooltips can cause issues with isHovered
		// maybe have global set of what is hovered and what is a parent of what
		isHovered = false;
		if (!props.interactive) return hideTip();
		if (
			props.doesntRetain &&
			document.activeElement?.matches(props.doesntRetain)
		)
			return hideTip();
		if (tipEl()?.contains(document.activeElement)) return;
		popupRemoveTimeout = setTimeout(hideTip, 0);
	}

	function showTipIfInteractive() {
		if (props.interactive) showTip();
	}

	function handleFocusOff() {
		if (!isHovered) hideTip();
	}

	let arrowEl!: HTMLElement;

	function middleware(props: TooltipProps) {
		const m = [shift({ padding: padding() })];

		// HACK: make volume slider work properly
		if (props.placement === "top-start") {
			m.push(offset({ mainAxis: -8 }));
		} else {
			m.push(offset({ mainAxis: 8 }));
		}

		m.push(flip());

		if (props.arrow ?? true) {
			m.push(solidArrow({ element: () => arrowEl, padding: 4 }));
		}
		return m;
	}

	const pos = useFloating(contentEl, tipEl, {
		whileElementsMounted: autoUpdate,
		strategy: "fixed",
		placement: props.placement,
		middleware: middleware(props),
	});

	createEffect(() => {
		const a = pos.middlewareData.arrow;
		const el = arrowEl;
		if (a && el) {
			el.style.translate = `${Math.round(a.x ?? 0)}px ${Math.round(a.y ?? 0)}px`;
			el.dataset.placement = pos.placement;
		}
	});

	createEffect(() => {
		wrap.addEventListener("mouseenter", showTip);
		wrap.addEventListener("mouseleave", considerHidingTip);
		onCleanup(() => {
			wrap.removeEventListener("mouseenter", showTip);
			wrap.removeEventListener("mouseleave", considerHidingTip);
		});
	});

	onMount(() => {
		setContentEl(wrap);
	});

	// TODO: use onPointerEnter/Leave instead of mouse events?
	return (
		<>
			{wrap}
			<Show when={visible()}>
				<Portal mount={props.mount ?? overlayEl}>
					<div
						onMouseEnter={showTipIfInteractive}
						onMouseLeave={considerHidingTip}
						onFocusOut={handleFocusOff}
						ref={setTipEl}
						style={{
							position: pos.strategy,
							translate: `${Math.round(pos.x ?? 0)}px ${Math.round(pos.y ?? 0)}px`,
							visibility: visible() ? "visible" : "hidden",
							"--padding": `${padding()}px`,
						}}
						class="tooltip"
						classList={{
							animate: animate(),
							interactive: props.interactive,
						}}
					>
						<div class="base"></div>
						<div class="inner">{tip as any}</div>
						<Show when={props.arrow ?? true}>
							<FloatingArrow ref={arrowEl!} />
						</Show>
					</div>
				</Portal>
			</Show>
		</>
	);
}

type CreateTooltipProps = Omit<TooltipProps, "mount"> & {
	tip: () => JSXElement;
	mount?: () => HTMLElement | undefined;
};

// TODO: only use one tooltip + event listener instead of per element
// or, debug performance issues in general
export function createTooltip(props: CreateTooltipProps) {
	const [contentEl, setContentEl] = createSignal<JSX.Element>();
	const [tipEl, setTipEl] = createSignal<JSX.Element>();
	// const [title, setTitle] = createSignal(getTitle());
	const [visible, setVisible] = createSignal(false);
	const [animate, setAnimate] = createSignal(true);
	let popupRemoveTimeout: NodeJS.Timeout;
	let isHovered = false;
	const overlayEl = document.getElementById("overlay")!;
	const padding = () => 8;

	if (props.animGroup) {
		const s = tooltipAnimSuppress.get(props.animGroup);
		if (!s) {
			tooltipAnimSuppress.set(props.animGroup, {
				shouldAnim: true,
				timeout: 0 as unknown as NodeJS.Timeout,
			});
		}
	}

	function getTitle() {
		const tip = tipEl();
		return props.tipText ?? (typeof tip === "string" ? tip : "");
	}

	function showTip() {
		clearTimeout(popupRemoveTimeout);
		isHovered = true;
		if (visible()) return;
		if (props.animGroup) {
			const s = tooltipAnimSuppress.get(props.animGroup)!;
			// console.log(s);
			setAnimate(s.shouldAnim);
			s.shouldAnim = false;
			clearTimeout(s.timeout);
		}
		setVisible(true);
		const wrap = contentEl();
		if (wrap instanceof HTMLElement) wrap.title = "";
	}

	function hideTip() {
		// TODO: exit animations? might be too much
		const wrap = contentEl();
		if (wrap instanceof HTMLElement) wrap.title = getTitle();
		setVisible(false);
		if (props.animGroup) {
			const s = tooltipAnimSuppress.get(props.animGroup)!;
			s.timeout = setTimeout(() => {
				s.shouldAnim = true;
			}, 500);
		}
		isHovered = false;
	}

	function considerHidingTip() {
		// FIXME: nested popups/tooltips can cause issues with isHovered
		// maybe have global set of what is hovered and what is a parent of what
		isHovered = false;
		if (!props.interactive) return hideTip();
		if (
			props.doesntRetain &&
			document.activeElement?.matches(props.doesntRetain)
		)
			return hideTip();
		const tip = tipEl();
		if (tip instanceof Node && tip.contains(document.activeElement)) return;
		popupRemoveTimeout = setTimeout(hideTip, 0);
	}

	function showTipIfInteractive() {
		if (props.interactive) showTip();
	}

	function handleFocusOff() {
		if (!isHovered) hideTip();
	}

	let arrowEl!: HTMLElement;

	function middleware(props: Omit<TooltipProps, "mount">) {
		const m = [shift({ padding: padding() })];

		// HACK: make volume slider work properly
		if (props.placement === "top-start") {
			m.push(offset({ mainAxis: -8 }));
		} else {
			m.push(offset({ mainAxis: 8 }));
		}

		m.push(flip());

		if (props.arrow ?? true) {
			m.push(solidArrow({ element: () => arrowEl, padding: 4 }));
		}
		return m;
	}

	const pos = useFloating(
		contentEl as () => HTMLElement,
		tipEl as () => HTMLElement,
		{
			whileElementsMounted: autoUpdate,
			strategy: "fixed",
			placement: props.placement,
			middleware: middleware(props),
		},
	);

	createEffect(() => {
		const a = pos.middlewareData.arrow;
		const el = arrowEl;
		if (a && el) {
			el.style.translate = `${Math.round(a.x ?? 0)}px ${Math.round(a.y ?? 0)}px`;
			el.dataset.placement = pos.placement;
		}
	});

	// TODO: use onPointerEnter/Leave instead of mouse events?
	// TODO: make typescript happy
	return {
		update: pos.update,
		content: (el: HTMLElement) => {
			onMount(() => {
				setContentEl(el);
				el.addEventListener("mouseenter", showTip);
				el.addEventListener("mouseleave", considerHidingTip);
			});

			onCleanup(() => {
				el.removeEventListener("mouseenter", showTip);
				el.removeEventListener("mouseleave", considerHidingTip);
				setVisible(false);
			});

			render(() => {
				return (
					<Show when={visible()}>
						<Portal mount={props.mount?.() ?? overlayEl}>
							<div
								onMouseEnter={showTipIfInteractive}
								onMouseLeave={considerHidingTip}
								onFocusOut={handleFocusOff}
								ref={setTipEl}
								style={{
									position: pos.strategy,
									translate: `${Math.round(pos.x ?? 0)}px ${Math.round(pos.y ?? 0)}px`,
									visibility: visible() ? "visible" : "hidden",
									"--padding": `${padding()}px`,
								}}
								class="tooltip"
								classList={{
									animate: animate(),
									interactive: props.interactive,
								}}
							>
								<div class="base"></div>
								<div class="inner">{props.tip()}</div>
								<Show when={props.arrow ?? true}>
									<FloatingArrow ref={arrowEl!} />
								</Show>
							</div>
						</Portal>
					</Show>
				);
			}, document.body);
		},
		showTip,
		considerHidingTip,
		setContentEl,
	};
}

// const handleMouseOver = (e: MouseEvent) => {
// 	// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
// 	// if (!tipEl) return;
// 	// const tipText = tipEl.dataset.tooltip;
// 	// setTip(tipText as string);
// 	// tooltip.setContentEl(tipEl)
// 	// tooltip.showTip();
// };

// const handleMouseOut = (e: MouseEvent) => {
// 	// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
// 	// if (tipEl) return;
// 	// tooltip.considerHidingTip()
// };

const FloatingArrow = (props: { ref: Ref<SVGSVGElement> }) => {
	return (
		<svg
			class="arrow"
			aria-hidden="true"
			width="14"
			height="14"
			viewBox="0 0 14 14"
			ref={props.ref}
		>
			<path d="M0,-1 H14 L7,7 Z" class="arrow-fill" stroke="none" />
			<path
				d="M0,-1 L7,7 L14,-1"
				class="arrow-stroke"
				fill="none"
				stroke-width="1"
				vector-effect="non-scaling-stroke"
			/>
		</svg>
	);
};

// from https://github.com/lxsmnsyc/solid-floating-ui/issues/5#issuecomment-1869444380
const solidArrow = ({
	element,
	padding,
}: {
	element: Accessor<HTMLElement>;
	padding: Padding | undefined;
}): Middleware => ({
	name: "arrow",
	fn(...args) {
		return arrow({
			element: element(),
			padding: padding,
		}).fn(...args);
	},
});
