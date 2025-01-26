import {
	createSignal,
	createUniqueId,
	JSX,
	ParentProps,
	useContext,
} from "solid-js";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip } from "@floating-ui/dom";
import { chatctx, useCtx } from "../context.ts";

export function Menu(props: ParentProps<{ submenu?: boolean }>) {
  const ctx = useCtx();
	return (
		<menu
			onMouseDown={(e) => !props.submenu && e.stopPropagation()}
			onMouseLeave={() => ctx.dispatch({ do: "menu.preview", id: null })}
		>
			<ul>
				{props.children}
			</ul>
		</menu>
	);
}

export function Submenu(
	props: ParentProps<
		{ content: JSX.Element; onClick?: (e: MouseEvent) => void }
	>,
) {
  const ctx = useCtx();
	const [itemEl, setItemEl] = createSignal<Element | undefined>();
	const [subEl, setSubEl] = createSignal<HTMLElement | undefined>();
	const [hovered, setHovered] = createSignal(false);

	// FIXME: seens to have an error on unmount
	const dims = useFloating(itemEl, subEl, {
		whileElementsMounted: autoUpdate,
		middleware: [flip()],
		placement: "right-start",
	});

	const menuId = createUniqueId();
	let timeout: number;

	function handleMouseEnter() {
		if (!ctx.data.cursor.preview) ctx.dispatch({ do: "menu.preview", id: menuId });
		let s = 1;
		const attempt = () => {
			const a = -ctx.data.cursor.vel * (1 / s);
			if (a <= 0.3) {
			  ctx.dispatch({ do: "menu.preview", id: menuId });
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

	return (
		<li
			ref={setItemEl}
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<button
				onClick={(e) => {
					e.stopPropagation();
					props.onClick?.(e);
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
					visibility: hovered() || ctx.data.cursor.preview === menuId ? "visible" : "hidden",
				}}
				onMouseEnter={() => setHovered(true)}
				onMouseLeave={() => setHovered(false)}
			>
				<Menu submenu>
					{props.children}
				</Menu>
			</div>
		</li>
	);
}

export function Item(
	props: ParentProps<{ onClick?: (e: MouseEvent) => void }>,
) {
	const ctx = useContext(chatctx)!;

	let timeout: number;
	function handleMouseEnter() {
		if (!ctx.data.cursor.preview) ctx.dispatch({ do: "menu.preview", id: null });
		const s = 1;
		const attempt = () => {
			const a = -ctx.data.cursor.vel * (1 / s);
			if (a <= 0) {
			  ctx.dispatch({ do: "menu.preview", id: null });
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
					e.stopPropagation();
					props.onClick?.(e);
					if (!props.onClick) ctx.dispatch({ do: "modal.alert", text: "todo" });
					ctx.dispatch({ do: "menu", menu: null });
				}}
				onMouseEnter={handleMouseEnter}
				onMouseLeave={handleMouseLeave}
			>
				{props.children}
			</button>
		</li>
	);
}

export function Separator() {
	return (
		<li>
			<hr />
		</li>
	);
}
