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
	createContext,
	createEffect,
	createMemo,
	createSignal,
	createUniqueId,
	type JSX,
	type JSXElement,
	onCleanup,
	type ParentProps,
	Show,
	useContext,
} from "solid-js";
import { Portal } from "solid-js/web";
import { FloatingArrow, solidArrow } from "@/atoms/Tooltip";

type TooltipAnimState = {
	shouldAnim: boolean;
	timeout: NodeJS.Timeout;
};

export type TooltipCreate = {
	tip: JSX.Element | (() => JSX.Element);
	interactive?: boolean;
	placement?: Placement;
	group?: string;
	arrow?: boolean;
	// other properties are removed/no longer supported
};

export type TooltipsState = {
	targetRef: Accessor<HTMLElement | undefined>;
	content: Accessor<JSX.Element | undefined>;
	visible: Accessor<boolean>;
	show: (el: HTMLElement, content: JSX.Element, config?: TooltipCreate) => void;
	hide: () => void;

	/** returns an id that can be used with `data-tooltip-id` */
	create: (create: TooltipCreate) => string;

	/** Map tracking animation suppression state for groups */
	suppressed: Map<string, TooltipAnimState>;
};

const TooltipContext = createContext<TooltipsState>();

export const TooltipProvider = (props: ParentProps) => {
	const [targetRef, setTargetRef] = createSignal<HTMLElement>();
	const [content, setContent] = createSignal<JSX.Element>();
	const [activeConfig, setActiveConfig] = createSignal<TooltipCreate>();
	const [visible, setVisible] = createSignal(false);
	const [animate, setAnimate] = createSignal(true);
	const [tipRef, setTipRef] = createSignal<HTMLElement>();
	const [arrowRef, setArrowRef] = createSignal<SVGElement>();

	const configs = new Map<string, TooltipCreate>();
	const suppressed = new Map<string, TooltipAnimState>();
	let popupHideTimeout: NodeJS.Timeout | undefined;
	let isHovered = false;

	const show = (
		el: HTMLElement,
		content: JSX.Element,
		config?: TooltipCreate,
	) => {
		clearTimeout(popupHideTimeout);
		isHovered = true;

		if (config?.group) {
			const s = suppressed.get(config.group);
			if (!s) {
				suppressed.set(config.group, {
					shouldAnim: true,
					timeout: 0 as unknown as NodeJS.Timeout,
				});
			} else {
				setAnimate(s.shouldAnim);
				s.shouldAnim = false;
				clearTimeout(s.timeout);
			}
		} else {
			setAnimate(true);
		}

		setTargetRef(el);
		setContent(content);
		setActiveConfig(config);
		setVisible(true);
	};

	const hide = () => {
		const config = activeConfig();
		if (config?.group) {
			const s = suppressed.get(config.group)!;
			s.timeout = setTimeout(() => {
				s.shouldAnim = true;
			}, 500);
		}

		setVisible(false);
		setTargetRef(undefined);
		setContent(undefined);
		setActiveConfig(undefined);
	};

	const considerHiding = () => {
		isHovered = false;
		const config = activeConfig();

		if (!config?.interactive) {
			hide();
			return;
		}

		// postpone poput hiding to prevent issues with interactive tooltips
		popupHideTimeout = setTimeout(() => {
			if (!isHovered) hide();
		}, 0);
	};

	const create = (config: TooltipCreate) => {
		const id = createUniqueId();
		configs.set(id, config);
		return id;
	};

	const state: TooltipsState = {
		targetRef: targetRef,
		content,
		visible,
		show,
		hide,
		create,
		suppressed,
	};

	const pos = useFloating(targetRef, tipRef, {
		whileElementsMounted: autoUpdate,
		strategy: "fixed",
		get placement() {
			return activeConfig()?.placement ?? "top";
		},
		middleware: [
			offset(8),
			flip(),
			shift({ padding: 8 }),
			solidArrow({
				element: () => arrowRef() as unknown as HTMLElement,
				padding: 4,
			}),
		],
	});

	createEffect(() => {
		const a = pos.middlewareData.arrow;
		const el = arrowRef();
		if (a && el) {
			el.style.translate = `${Math.round(a.x ?? 0)}px ${Math.round(a.y ?? 0)}px`;
			el.dataset.placement = pos.placement;
		}
	});

	const handleMouseOver = (e: MouseEvent) => {
		const target = (e.target as HTMLElement).closest(
			"[data-tooltip], [data-tooltip-id]",
		) as HTMLElement | null;
		if (!target) return;

		const tooltipId = target.getAttribute("data-tooltip-id");
		if (tooltipId && configs.has(tooltipId)) {
			const config = configs.get(tooltipId)!;
			const content =
				typeof config.tip === "function" ? config.tip() : config.tip;
			show(target, content, config);
			return;
		}

		const tooltipText = target.getAttribute("data-tooltip");
		if (tooltipText) {
			show(target, tooltipText);
		}
	};

	const handleMouseOut = (e: MouseEvent) => {
		const target = (e.target as HTMLElement).closest(
			"[data-tooltip], [data-tooltip-id]",
		) as HTMLElement | null;
		if (!target) return;

		if (target === targetRef()) {
			considerHiding();
		}
	};

	document.addEventListener("mouseover", handleMouseOver);
	document.addEventListener("mouseout", handleMouseOut);
	onCleanup(() => {
		document.removeEventListener("mouseover", handleMouseOver);
		document.removeEventListener("mouseout", handleMouseOut);
	});

	return (
		<TooltipContext.Provider value={state}>
			{props.children}
			<Show when={visible()}>
				<Portal mount={document.getElementById("overlay")!}>
					<div
						ref={setTipRef}
						onMouseEnter={() => {
							isHovered = true;
							clearTimeout(popupHideTimeout);
						}}
						onMouseLeave={considerHiding}
						onFocusOut={() => {
							if (!isHovered) hide();
						}}
						style={{
							position: pos.strategy,
							top: "0",
							left: "0",
							translate: `${Math.round(pos.x ?? 0)}px ${Math.round(pos.y ?? 0)}px`,
						}}
						class="tooltip"
						classList={{
							animate: animate(),
							interactive: activeConfig()?.interactive ?? false,
						}}
					>
						<div class="base"></div>
						<div class="inner">{content()}</div>
						<Show when={activeConfig()?.arrow ?? true}>
							<FloatingArrow ref={setArrowRef} />
						</Show>
					</div>
				</Portal>
			</Show>
		</TooltipContext.Provider>
	);
};

export const useTooltip = () => {
	const ctx = useContext(TooltipContext);
	if (!ctx) throw new Error("useTooltip must be used inside a TooltipProvider");
	return ctx;
};
