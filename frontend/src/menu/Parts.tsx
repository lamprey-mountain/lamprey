import {
	children,
	createEffect,
	createSignal,
	createUniqueId,
	type JSX,
	type ParentProps,
	useContext,
} from "solid-js";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip } from "@floating-ui/dom";
import { chatctx, useCtx } from "../context.ts";
import { useModals } from "../contexts/modal";
import { useMenu } from "../contexts/menu.tsx";

function isSeparator(child: any): boolean {
	return (child as HTMLElement)?.classList.contains("menu-separator");
}

export function Menu(props: ParentProps<{ submenu?: boolean }>) {
	const ctx = useCtx();
	const { setPreview } = useMenu();
	const resolved = children(() => props.children);

	const filtered = () => {
		const flat = resolved.toArray();
		if (flat.length === 0) return flat;

		const result: any[] = [];
		let prevWasSeparator = false;

		for (const child of flat) {
			if (!child) continue;
			const isSep = isSeparator(child);
			if (isSep && (result.length === 0 || prevWasSeparator)) continue;
			result.push(child);
			prevWasSeparator = isSep;
		}

		if (result.length > 0 && isSeparator(result[result.length - 1])) {
			result.pop();
		}

		return result;
	};

	return (
		<menu
			onMouseDown={(e) => !props.submenu && e.stopPropagation()}
			onMouseLeave={() => setPreview(null)}
			role="menu"
		>
			<ul>{filtered()}</ul>
		</menu>
	);
}

export function Submenu(
	props: ParentProps<
		{
			content: JSX.Element;
			onClick?: (e: MouseEvent) => void;
			onOpen?: () => void;
		}
	>,
) {
	const ctx = useCtx();
	const { preview, setPreview } = useMenu();
	const [itemEl, setItemEl] = createSignal<Element | undefined>();
	const [subEl, setSubEl] = createSignal<HTMLElement | undefined>();
	const [hovered, setHovered] = createSignal(false);

	const dims = useFloating(itemEl, subEl, {
		whileElementsMounted: autoUpdate,
		middleware: [flip()],
		placement: "right-start",
	});

	const menuId = createUniqueId();
	let timeout: ReturnType<typeof setTimeout>;

	function handleMouseEnter() {
		if (!preview()) {
			setPreview(menuId);
		}
		let s = 1;
		const attempt = () => {
			const a = -ctx.data.cursor.vel * (1 / s);
			if (a <= 0.3) {
				setPreview(menuId);
			} else {
				s += .01;
				timeout = setTimeout(attempt, a);
			}
		};
		attempt();
	}

	function handleMouseLeave() {
		clearTimeout(timeout);
	}

	const visible = () => hovered() || preview() === menuId;

	createEffect(() => {
		if (visible()) {
			props.onOpen?.();
		}
	});

	return (
		<li
			ref={setItemEl}
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
			aria-haspopup="menu"
			aria-expanded={visible()}
			aria-controls={menuId}
		>
			<button
				onClick={(e) => {
					e.stopPropagation();
					props.onClick?.(e);
					// HACK: close menu
					document.getElementById("root")!.click();
				}}
			>
				{props.content}
			</button>
			<div
				ref={setSubEl}
				class="submenu"
				style={{
					position: dims.strategy,
					left: `${dims.x}px`,
					top: `${dims.y}px`,
					visibility: visible() ? "visible" : "hidden",
				}}
				onMouseEnter={() => setHovered(true)}
				onMouseLeave={() => setHovered(false)}
				id={menuId}
			>
				<Menu submenu>
					{props.children}
				</Menu>
			</div>
		</li>
	);
}

type ItemColor = "danger";

export function Item(
	props: ParentProps<
		{
			onClick?: (e: MouseEvent) => void;
			disabled?: boolean;
			color?: ItemColor;
			classList?: Record<string, boolean>;
		}
	>,
) {
	const ctx = useContext(chatctx)!;
	const { preview, setPreview } = useMenu();

	let timeout: ReturnType<typeof setTimeout>;
	function handleMouseEnter() {
		if (!preview()) {
			setPreview(null);
		}
		const s = 1;
		const attempt = () => {
			const a = -ctx.data.cursor.vel * (1 / s);
			if (a <= 0) {
				setPreview(null);
			} else {
				timeout = setTimeout(attempt, a);
			}
		};
		attempt();
	}

	function handleMouseLeave() {
		clearTimeout(timeout);
	}

	return (
		<li>
			<button
				onClick={(e) => {
					props.onClick?.(e);
					if (!props.onClick) {
						const [, modalCtl] = useModals();
						modalCtl.alert("todo");
					}
				}}
				onMouseEnter={handleMouseEnter}
				onMouseLeave={handleMouseLeave}
				disabled={props.disabled ?? false}
				classList={{
					...props.classList,
					["color-" + props.color]: !!props.color,
				}}
			>
				{props.children}
			</button>
		</li>
	);
}

export function Separator() {
	return (
		<li class="menu-separator">
			<hr />
		</li>
	);
}
