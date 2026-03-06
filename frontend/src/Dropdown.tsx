import {
	createEffect,
	createSignal,
	createUniqueId,
	For,
	type JSX,
	Show,
	untrack,
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

const ChevronDown = () => (
	<svg
		width="16"
		height="16"
		viewBox="0 0 16 16"
		fill="none"
		xmlns="http://www.w3.org/2000/svg"
		class="dropdown-chevron"
	>
		<path
			d="M4 6L8 10L12 6"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
	</svg>
);

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
			const filtered = getFiltered();
			if (filtered.length === 0) return;
			const idx = filtered.findIndex((i) => i.obj === getHovered()!);
			setHovered(filtered[(idx + 1) % filtered.length].obj);
		},
		prev() {
			const filtered = getFiltered();
			if (filtered.length === 0) return;
			const idx = filtered.findIndex((i) => i.obj === getHovered()!);
			setHovered(filtered[(filtered.length + idx - 1) % filtered.length].obj);
		},
	};
}

export function createDropdown<T>(
	props: {
		selected?: T;
		required?: boolean;
		onSelect?: (item: T | null) => void;
		onInput?: (value: string) => void;
		onKeyDown?: (e: KeyboardEvent) => void;
		onBlur?: (e: FocusEvent) => void;
		ignoreMissingLabel?: boolean;
		options: () => Array<DropdownItem<T>>;
		mount?: Element | DocumentFragment | null;
		placeholder?: string;
	},
) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLDivElement>();
	const [containerEl, setContainerEl] = createSignal<HTMLDivElement>();
	const [selected, setSelected] = createSignal<T | null>(
		props.selected ?? null,
	);
	const position = useFloating(containerEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset({ mainAxis: -1 }), flip()],
		placement: "bottom",
	});

	const selector = createSelect<T>();

	createEffect(() => {
		selector.setItems(props.options);
	});

	createEffect(() => {
		if (props.selected !== undefined) setSelected(() => props.selected!);
	});

	const select = (item: T | null) => {
		setSelected(() => item);
		setShown(false);
		props.onSelect?.(item);
	};

	const binds = createKeybinds({
		"ArrowUp": (e) => {
			if (!shown()) {
				e.preventDefault();
				const options = props.options();
				const idx = options.findIndex((i) => i.item === selected());
				const next = (options.length + idx - 1) % options.length;
				select(options[next]?.item);
			}
		},
		"ArrowDown": (e) => {
			if (!shown()) {
				e.preventDefault();
				const options = props.options();
				const idx = options.findIndex((i) => i.item === selected());
				const next = (idx + 1) % options.length;
				select(options[next]?.item);
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
			const hovered = selector.getHovered();
			if (shown() && hovered) {
				e.preventDefault();
				select(hovered.item);
			}
		},
	});

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		const options = props.options();
		if (e.deltaY < 0) {
			if (shown()) {
				selector.prev();
			} else {
				const idx = options.findIndex((i) => i.item === selected());
				const next = (options.length + idx - 1) % options.length;
				select(options[next]?.item);
			}
		} else if (e.deltaY > 0) {
			if (shown()) {
				selector.next();
			} else {
				const idx = options.findIndex((i) => i.item === selected());
				const next = (idx + 1) % options.length;
				select(options[next]?.item);
			}
		}
	}

	const [value, setValue] = createSignal<string>("");
	createEffect((prev) => {
		const s = selected();
		if (s !== prev) {
			if (document.activeElement === inputEl()) return s;
			const opt = untrack(() => props.options()).find((i) => i.item === s);
			if (opt) {
				setValue(opt.label);
			} else if (!props.ignoreMissingLabel) {
				setValue("");
			}
		}
		return s;
	});

	const listboxId = createUniqueId();

	return {
		setSelected(t: T) {
			setSelected(() => t);
		},
		setValue(s: string) {
			setValue(s);
		},
		open() {
			setShown(true);
			selector.setFilter("");
		},
		focus() {
			inputEl()?.focus();
		},
		View(props2: { style?: string | JSX.CSSProperties; class?: string }) {
			return (
				<div
					ref={setContainerEl}
					class={`dropdown-container ${props2.class ?? ""}`}
					style={props2.style}
				>
					<input
						type="text"
						class="dropdown"
						ref={setInputEl}
						placeholder={props.placeholder ?? "select an item..."}
						value={value()}
						onClick={() => {
							setShown(true);
							selector.setFilter("");
						}}
						onBlur={(e) => {
							queueMicrotask(() => setShown(false));
							if (!props.ignoreMissingLabel) {
								const opt = props.options().find((i) => i.item === selected());
								if (opt) {
									setValue(opt.label);
								} else {
									setValue("");
								}
							}
							props.onBlur?.(e);
						}}
						onInput={(e) => {
							const { value } = e.target;
							setValue(value);
							selector.setFilter(value);
							if (value) setShown(true);
							props.onInput?.(value);
						}}
						onKeyDown={(e) => {
							binds(e);
							props.onKeyDown?.(e);
						}}
						onWheel={handleWheel}
						role="combobox"
						aria-autocomplete="list"
						aria-haspopup="listbox"
						aria-controls={shown() ? listboxId : undefined}
						aria-expanded={shown()}
						style={{ width: "100%" }}
					/>
					<div
						class="dropdown-chevron-wrapper"
						onClick={() => setShown(!shown())}
					>
						<ChevronDown />
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
									width: `${containerEl()?.offsetWidth || 0}px`,
								}}
							>
								<ul>
									<For each={selector.getFiltered()} fallback={"no options"}>
										{(entry) => (
											<li
												onMouseOver={() => selector.setHovered(entry.obj)}
												onMouseDown={(e) => {
													e.preventDefault();
													select(entry.obj.item);
												}}
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
				</div>
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
						queueMicrotask(() => setShown(false));
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
			<div
				class="dropdown-chevron-wrapper"
				onClick={(e) => {
					e.stopPropagation();
					setShown(!shown());
				}}
			>
				<ChevronDown />
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
										}}
									>
										<Show when={props.selected.includes(entry.obj.item)}>
											<Checkmark
												seed={entry.obj.label}
												style={{
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

export function Dropdown<T>(
	props: VoidProps<{
		selected?: T;
		required?: boolean;
		onSelect?: (item: T | null) => void;
		options: Array<DropdownItem<T>>;
		style?: string;
		mount?: Element | DocumentFragment | null;
		ignoreMissingLabel?: boolean;
		placeholder?: string;
	}>,
) {
	const dropdown = createDropdown<T>({
		get selected() {
			return props.selected;
		},
		required: props.required,
		onSelect: props.onSelect,
		options: () => props.options,
		mount: props.mount,
		ignoreMissingLabel: props.ignoreMissingLabel,
		placeholder: props.placeholder,
	});

	return <dropdown.View style={props.style} />;
}
