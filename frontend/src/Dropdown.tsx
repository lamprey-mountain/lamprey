import {
	createEffect,
	createSignal,
	createUniqueId,
	For,
	type JSX,
	Show,
	type VoidProps,
} from "solid-js";
import { Portal } from "solid-js/web";
import { go } from "fuzzysort";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { createKeybinds } from "./keybinds";
import { Checkmark } from "./icons";

export type DropdownItem<T> = {
	item: T;
	label: string;
	view?: JSX.Element;
};

function createSelect<T>() {
	const [getItems, setItems] = createSignal<Array<DropdownItem<T>>>([]);
	const [getFiltered, setFiltered] = createSignal<
		Array<Fuzzysort.KeyResult<DropdownItem<T>>>
	>([]);
	const [getFilter, setFilter] = createSignal("");
	const [getHovered, setHovered] = createSignal<DropdownItem<T>>();

	createEffect(() => {
		setFiltered(
			go(getFilter(), getItems(), {
				key: "label",
				all: true,
			}) as unknown as Array<Fuzzysort.KeyResult<DropdownItem<T>>>,
		);
		if (!getFiltered().some((i) => i.obj === getHovered())) {
			setHovered(getFiltered()?.[0]?.obj);
		}
	});

	return {
		getFiltered,
		getHovered,
		setItems,
		setFilter,
		setHovered,
		next() {
			const idx = getFiltered().findIndex((i) => i.obj === getHovered()!);
			setHovered(getFiltered()[(idx + 1) % getFiltered().length].obj);
		},
		prev() {
			const idx = getFiltered().findIndex((i) => i.obj === getHovered()!);
			setHovered(
				getFiltered()[(getFiltered().length + idx - 1) % getFiltered().length]
					.obj,
			);
		},
	};
}

export function createDropdown<T>(
	props: {
		selected?: T;
		required?: boolean;
		onSelect?: (item: T | null) => void;
		options: () => Array<DropdownItem<T>>;
		mount?: Element | DocumentFragment | null;
	},
) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLDivElement>();
	const [selected, setSelected] = createSignal<T | null>(
		props.selected ?? props.options()[0]?.item ?? null,
	);
	const position = useFloating(inputEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset({ mainAxis: -1 }), flip()],
		placement: "bottom",
	});

	const selector = createSelect<T>();

	createEffect(() => {
		selector.setItems(props.options);
	});

	createEffect(() => {
		if (props.selected) setSelected(() => props.selected!);
	});

	const binds = createKeybinds({
		"ArrowUp": () => {
			if (!shown()) {
				const idx = props.options().findIndex((i) => i.item === selected());
				const next = (props.options.length + idx - 1) % props.options.length;
				select(props.options()[next]?.item);
			}
		},
		"ArrowDown": () => {
			if (!shown()) {
				const idx = props.options().findIndex((i) => i.item === selected());
				const next = (idx + 1) % props.options.length;
				select(props.options()[next]?.item);
			}
		},
		"ArrowUp, Shift-Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.prev();
			}
		},
		"ArrowDown, Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.next();
			}
		},
		"Escape": (e) => {
			if (shown()) {
				e.preventDefault();
				setShown(false);
			}
		},
		"Enter": (e) => {
			e.preventDefault();
			if (shown()) {
				select(selector.getHovered()?.item ?? null);
			} else {
				setShown(true);
				setTimeout(() => {
					debugger;
				}, 100);
			}
		},
	});

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		if (e.deltaY < 0) {
			if (shown()) {
				selector.prev();
			} else {
				const idx = props.options().findIndex((i) => i.item === selected());
				const next = (props.options.length + idx - 1) % props.options.length;
				select(props.options()[next]?.item);
			}
		} else if (e.deltaY > 0) {
			if (shown()) {
				selector.next();
			} else {
				const idx = props.options().findIndex((i) => i.item === selected());
				const next = (idx + 1) % props.options.length;
				select(props.options()[next]?.item);
			}
		}
	}

	function select(item: T | null) {
		setSelected(() => item);
		setShown(false);
		props.onSelect?.(item);
	}

	const [value, setValue] = createSignal<string | undefined>(undefined, {
		equals: false,
	});
	createEffect(() => {
		setValue(props.options().find((i) => i.item === selected())?.label);
	});

	const listboxId = createUniqueId();

	// TODO: maybe use click instead of mousedown?
	// TODO: automatically show dropdown items on hover?

	return {
		setSelected(t: T) {
			setSelected(() => t);
		},
		View() {
			return (
				<>
					<input
						class="dropdown"
						ref={setInputEl}
						placeholder="select an item..."
						value={value()}
						onMouseDown={() => setShown(!shown())}
						onBlur={() => {
							setShown(false);
							setValue(
								props.options().find((i) => i.item === selected())?.label,
							);
						}}
						onInput={(e) => {
							const { value } = e.target;
							selector.setFilter(e.target.value);
							if (value) setShown(true);
						}}
						onKeyDown={binds}
						onWheel={handleWheel}
						role="combobox"
						aria-autocomplete="list"
						aria-haspopup="listbox"
						aria-controls={shown() ? listboxId : undefined}
						aria-expanded={shown()}
						aria-keyshortcuts={shown()
							? "ArrowUp ArrowDown Tab Shift+Tab Escape Enter"
							: "Enter"}
					/>
					<Portal mount={props.mount ?? document.body}>
						<Show when={shown()}>
							<menu
								role="listbox"
								ref={setDropdownEl}
								id={listboxId}
								class="dropdown-items floating"
								style={{
									"z-index": 999999,
									position: position.strategy,
									translate: `${position.x}px ${position.y}px`,
									width: `${inputEl()?.offsetWidth || 0}px`,
								}}
							>
								<ul>
									<For each={selector.getFiltered()} fallback={"no options"}>
										{(entry) => (
											<li
												onMouseOver={() => selector.setHovered(entry.obj)}
												onMouseDown={() => select(entry.obj.item)}
												classList={{
													hovered:
														entry.obj.item === selector.getHovered()?.item,
													selected: entry.obj.item === selected(),
												}}
												aria-selected={entry.obj.item === selected()}
											>
												{entry.obj.view ?? entry.obj.label}
											</li>
										)}
									</For>
								</ul>
							</menu>
						</Show>
					</Portal>
				</>
			);
		},
	};
}

export function MultiDropdown<T>(
	props: VoidProps<{
		selected: T[];
		onSelect: (item: T) => void;
		onRemove: (item: T) => void;
		options: Array<DropdownItem<T>>;
		style?: string;
		placeholder?: string;
		mount?: Element | DocumentFragment | null;
	}>,
) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLDivElement>();
	const [containerEl, setContainerEl] = createSignal<HTMLDivElement>();

	const position = useFloating(containerEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset({ mainAxis: -1 }), flip()],
		placement: "bottom",
	});

	const selector = createSelect<T>();

	createEffect(() => {
		selector.setItems(props.options);
	});

	const binds = createKeybinds({
		"ArrowUp, Shift-Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.prev();
			}
		},
		"ArrowDown, Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.next();
			}
		},
		"Escape": (e) => {
			if (shown()) {
				e.preventDefault();
				setShown(false);
			}
		},
		"Enter": (e) => {
			e.preventDefault();
			if (shown()) {
				const hovered = selector.getHovered();
				if (hovered) {
					props.onSelect(hovered.item);
					selector.setFilter("");
					if (inputEl()) inputEl()!.value = "";
				}
			} else {
				setShown(true);
			}
		},
		"Backspace": (e) => {
			if (selector.getFilter() === "" && props.selected.length > 0) {
				props.onRemove(props.selected[props.selected.length - 1]);
			}
		},
	});

	function select(item: T) {
		if (props.selected.includes(item)) {
			props.onRemove(item);
		} else {
			props.onSelect(item);
		}
		selector.setFilter("");
		if (inputEl()) inputEl()!.value = "";
		// keep it open for more selections?
		// setShown(false);
	}

	const listboxId = createUniqueId();

	return (
		<div
			ref={setContainerEl}
			class="dropdown multi-dropdown"
			onClick={() => inputEl()?.focus()}
			style={props.style}
		>
			<div class="multi-dropdown-selected">
				<For each={props.selected}>
					{(item) => (
						<span class="chip">
							{props.options.find((o) => o.item === item)?.label ?? "???"}
							<button
								onClick={(e) => {
									e.stopPropagation();
									props.onRemove(item);
								}}
							>
								×
							</button>
						</span>
					)}
				</For>
				<input
					ref={setInputEl}
					placeholder={props.selected.length === 0 ? props.placeholder : ""}
					onFocus={() => setShown(true)}
					onBlur={() => {
						setTimeout(() => setShown(false), 200);
					}}
					onInput={(e) => {
						selector.setFilter(e.target.value);
						setShown(true);
					}}
					onKeyDown={binds}
					role="combobox"
					aria-autocomplete="list"
					aria-haspopup="listbox"
					aria-controls={shown() ? listboxId : undefined}
					aria-expanded={shown()}
				/>
			</div>
			<Portal mount={props.mount ?? document.body}>
				<Show when={shown()}>
					<menu
						role="listbox"
						ref={setDropdownEl}
						id={listboxId}
						class="dropdown-items floating"
						style={{
							"z-index": 999999,
							position: position.strategy,
							translate: `${position.x}px ${position.y}px`,
							width: `${containerEl()?.parentElement?.offsetWidth || 0}px`,
						}}
					>
						<ul>
							<For each={selector.getFiltered()} fallback={"no options"}>
								{(entry) => (
									<li
										onMouseOver={() => selector.setHovered(entry.obj)}
										onMouseDown={(e) => {
											e.preventDefault();
											e.stopPropagation();
											select(entry.obj.item);
										}}
										classList={{
											hovered: entry.obj.item === selector.getHovered()?.item,
											selected2: props.selected.includes(entry.obj.item),
										}}
										aria-selected={props.selected.includes(entry.obj.item)}
										style={{
											display: "flex",
											// "align-items": "center",
											// "justify-content": "space-between",
										}}
									>
										<Show when={props.selected.includes(entry.obj.item)}>
											<Checkmark
												seed={entry.obj.label}
												style={{
													// HACK: colored icons
													filter:
														"invert(0.5) sepia(1) saturate(3) hue-rotate(220deg)",
												}}
											/>
										</Show>
										{entry.obj.view ?? entry.obj.label}
									</li>
								)}
							</For>
						</ul>
					</menu>
				</Show>
			</Portal>
		</div>
	);
}

// TODO: placeholder
export function Dropdown<T>(
	props: VoidProps<{
		selected?: T;
		required?: boolean;
		onSelect?: (item: T | null) => void;
		options: Array<DropdownItem<T>>;
		style?: string;
		mount?: Element | DocumentFragment | null;
	}>,
) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLDivElement>();
	const [selected, setSelected] = createSignal<T | null>(
		props.selected ?? props.options[0]?.item ?? null,
	);
	const position = useFloating(inputEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset({ mainAxis: -1 }), flip()],
		placement: "bottom",
	});

	const selector = createSelect<T>();

	createEffect(() => {
		selector.setItems(props.options);
	});

	createEffect(() => {
		if (props.selected) setSelected(() => props.selected!);
	});

	const binds = createKeybinds({
		"ArrowUp": () => {
			if (!shown()) {
				const idx = props.options.findIndex((i) => i.item === selected());
				const next = (props.options.length + idx - 1) % props.options.length;
				select(props.options[next]?.item);
			}
		},
		"ArrowDown": () => {
			if (!shown()) {
				const idx = props.options.findIndex((i) => i.item === selected());
				const next = (idx + 1) % props.options.length;
				select(props.options[next]?.item);
			}
		},
		"ArrowUp, Shift-Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.prev();
			}
		},
		"ArrowDown, Tab": (e) => {
			if (shown()) {
				e.preventDefault();
				selector.next();
			}
		},
		"Escape": (e) => {
			if (shown()) {
				e.preventDefault();
				setShown(false);
			}
		},
		"Enter": (e) => {
			e.preventDefault();
			if (shown()) {
				select(selector.getHovered()?.item ?? null);
			} else {
				setShown(true);
				setTimeout(() => {
					debugger;
				}, 100);
			}
		},
	});

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		if (e.deltaY < 0) {
			if (shown()) {
				selector.prev();
			} else {
				const idx = props.options.findIndex((i) => i.item === selected());
				const next = (props.options.length + idx - 1) % props.options.length;
				select(props.options[next]?.item);
			}
		} else if (e.deltaY > 0) {
			if (shown()) {
				selector.next();
			} else {
				const idx = props.options.findIndex((i) => i.item === selected());
				const next = (idx + 1) % props.options.length;
				select(props.options[next]?.item);
			}
		}
	}

	function select(item: T | null) {
		setSelected(() => item);
		setShown(false);
		props.onSelect?.(item);
	}

	const [value, setValue] = createSignal<string | undefined>(undefined, {
		equals: false,
	});
	createEffect(() => {
		setValue(props.options.find((i) => i.item === selected())?.label);
	});

	const listboxId = createUniqueId();

	// TODO: maybe use click instead of mousedown?
	// TODO: automatically show dropdown items on hover?
	// TODO: show chevron arrow

	return (
		<>
			<input
				class="dropdown"
				ref={setInputEl}
				placeholder="select an item..."
				value={value()}
				onMouseDown={() => setShown(!shown())}
				onBlur={() => {
					setShown(false);
					setValue(props.options.find((i) => i.item === selected())?.label);
				}}
				onInput={(e) => {
					const { value } = e.target;
					selector.setFilter(e.target.value);
					if (value) setShown(true);
				}}
				onKeyDown={binds}
				onWheel={handleWheel}
				role="combobox"
				aria-autocomplete="list"
				aria-haspopup="listbox"
				aria-controls={shown() ? listboxId : undefined}
				aria-expanded={shown()}
				aria-keyshortcuts={shown()
					? "ArrowUp ArrowDown Tab Shift+Tab Escape Enter"
					: "Enter"}
				style={props.style}
			/>
			<Portal mount={props.mount ?? document.body}>
				<Show when={shown()}>
					<menu
						role="listbox"
						ref={setDropdownEl}
						id={listboxId}
						class="dropdown-items floating"
						style={{
							"z-index": 999999,
							position: position.strategy,
							translate: `${position.x}px ${position.y}px`,
							width: `${inputEl()?.offsetWidth || 0}px`,
						}}
					>
						<ul>
							<For each={selector.getFiltered()} fallback={"no options"}>
								{(entry) => (
									<li
										onMouseOver={() => selector.setHovered(entry.obj)}
										onMouseDown={() => select(entry.obj.item)}
										classList={{
											hovered: entry.obj.item === selector.getHovered()?.item,
											selected: entry.obj.item === selected(),
										}}
										aria-selected={entry.obj.item === selected()}
									>
										{entry.obj.view ?? entry.obj.label}
									</li>
								)}
							</For>
						</ul>
					</menu>
				</Show>
			</Portal>
		</>
	);
}
