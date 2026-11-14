import { type Accessor, createSignal } from "solid-js";

export type RovingTabindexOptions = {
	/** ids of the items in render order */
	ids: Accessor<string[]>;
	/** id that should be tabbable until an item gets focused */
	initialId?: Accessor<string | undefined>;
	orientation?: "vertical" | "horizontal";
};

/**
 * Roving tabindex for a list of items: only one item is in the tab order, arrow
 * keys (and home/end) move focus between items, and the tab stop follows focus.
 *
 * Items must be focusable elements carrying `data-roving-id`, and their
 * `tabIndex` should come from `tabIndex(id)`.
 */
export const useRovingTabindex = (props: RovingTabindexOptions) => {
	let containerEl: HTMLElement | undefined;
	const [activeId, setActiveId] = createSignal<string>();

	const getItems = () =>
		Array.from(
			containerEl?.querySelectorAll<HTMLElement>("[data-roving-id]") ?? [],
		).filter((el) => el.offsetParent !== null);

	const itemFromEvent = (target: EventTarget | null) => {
		if (!(target instanceof Element)) return;
		const item = target.closest<HTMLElement>("[data-roving-id]");
		if (!item || !containerEl?.contains(item)) return;
		return item;
	};

	const focusAt = (index: number) => {
		const items = getItems();
		if (!items.length) return;
		const item = items[((index % items.length) + items.length) % items.length];
		setActiveId(item.dataset.rovingId);
		item.focus();
	};

	const move = (direction: 1 | -1) => {
		const items = getItems();
		const current = items.findIndex(
			(item) => item.dataset.rovingId === activeId(),
		);
		if (current === -1) {
			focusAt(direction === 1 ? 0 : -1);
			return;
		}
		focusAt(current + direction);
	};

	const onKeyDown = (e: KeyboardEvent) => {
		if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
		if (!itemFromEvent(e.target)) return;

		const vertical = (props.orientation ?? "vertical") === "vertical";
		const next = vertical ? "ArrowDown" : "ArrowRight";
		const prev = vertical ? "ArrowUp" : "ArrowLeft";

		if (e.key === next || e.key === prev) {
			e.preventDefault();
			move(e.key === next ? 1 : -1);
		} else if (e.key === "Home" || e.key === "End") {
			e.preventDefault();
			focusAt(e.key === "Home" ? 0 : -1);
		}
	};

	/** the item that is (or should be) in the tab order */
	const currentId = () => {
		const ids = props.ids();
		const active = activeId();
		if (active !== undefined && ids.includes(active)) return active;
		const initial = props.initialId?.();
		if (initial !== undefined && ids.includes(initial)) return initial;
		return ids[0];
	};

	const tabIndex = (id: string) => (id === currentId() ? 0 : -1);

	const containerProps = {
		ref: (el: HTMLElement) => {
			containerEl = el;
		},
		onKeyDown,
		onFocusIn: (e: FocusEvent) => {
			const item = itemFromEvent(e.target);
			if (item) setActiveId(item.dataset.rovingId);
		},
		onPointerDown: (e: PointerEvent) => {
			const item = itemFromEvent(e.target);
			if (item) setActiveId(item.dataset.rovingId);
		},
	};

	return { containerProps, tabIndex };
};
