import { autoUpdate, flip, offset, shift, size } from "@floating-ui/dom";
import { go } from "fuzzysort";
import type { Channel } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createMemo,
	createSignal,
	createUniqueId,
	For,
	type JSX,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { ChannelIcon } from "../avatar/ChannelIcon.tsx";
import { createKeybinds } from "../keybinds";

export type ChannelPickerOption = {
	channel: Channel;
	label: string;
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
	const [getItems, setItems] = createSignal<Array<T>>([]);
	const [getFilter, setFilter] = createSignal("");
	const [getHovered, setHovered] = createSignal<T | null>(null);

	// Use Memo for filtering logic (derived state)
	const filtered = createMemo(() => {
		const filter = getFilter();
		const items = getItems();
		if (!filter) return items.map((i) => ({ obj: i }));

		return go(filter, items, {
			key: "label",
			all: true,
		}) as unknown as Array<Fuzzysort.KeyResult<T>>;
	});

	// Synchronize hovered state when list changes
	createEffect(() => {
		const list = filtered();
		const currentHovered = getHovered();
		if (!list.some((i) => i.obj === currentHovered)) {
			setHovered((list[0]?.obj ?? null) as any);
		}
	});

	return {
		filtered,
		getHovered,
		setItems,
		setFilter,
		setHovered,
		next() {
			const list = filtered();
			if (list.length === 0) return;
			const idx = list.findIndex((i) => i.obj === getHovered()!);
			setHovered((list[(idx + 1) % list.length]?.obj ?? null) as any);
		},
		prev() {
			const list = filtered();
			if (list.length === 0) return;
			const idx = list.findIndex((i) => i.obj === getHovered()!);
			setHovered(
				(list[(list.length + idx - 1) % list.length]?.obj ?? null) as any,
			);
		},
	} as const;
}

export function createChannelPicker(props: {
	selected?: Channel | null;
	channels: () => Channel[];
	onSelect?: (channel: Channel | null) => void;
	onInput?: (value: string) => void;
	onKeyDown?: (e: KeyboardEvent) => void;
	onBlur?: (e: FocusEvent) => void;
	mount?: Element | DocumentFragment | null;
	placeholder?: string;
	filter?: (channel: Channel) => boolean;
	style?: JSX.CSSProperties;
	class?: string;
}) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLUListElement>();
	const [containerEl, setContainerEl] = createSignal<HTMLDivElement>();
	const [selected, setSelected] = createSignal<Channel | null>(
		props.selected ?? null,
	);
	const listboxId = createUniqueId();

	const PADDING = 16;
	const position = useFloating(containerEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [
			offset({ mainAxis: -1 }),
			flip({ padding: PADDING }),
			shift({ padding: PADDING }),
			size({
				padding: PADDING,
				apply({ availableHeight, elements }) {
					Object.assign(elements.floating.style, {
						maxHeight: `${Math.max(0, availableHeight)}px`,
					});
				},
			}),
		],
		placement: "bottom",
	});

	const selector = createSelect<ChannelPickerOption>();

	const options = createMemo(() => {
		const channels = props.channels();
		const filter = props.filter;
		return channels
			.filter((c) => !filter || filter(c))
			.map((c) => ({
				channel: c,
				label: c.name,
			}));
	});

	createEffect(() => {
		selector.setItems(options());
	});

	createEffect(() => {
		if (props.selected !== undefined) setSelected(() => props.selected!);
	});

	const select = (channel: Channel | null) => {
		setSelected(() => channel);
		setShown(false);
		const opt = options().find((o) => o.channel === channel);
		if (opt) {
			setValue(opt.label);
		}
		inputEl()?.blur();
		props.onSelect?.(channel);
	};

	const binds = createKeybinds({
		ArrowUp: (e) => {
			if (!shown()) {
				e.preventDefault();
				const opts = options();
				const idx = opts.findIndex((o) => o.channel === selected());
				const next = (opts.length + idx - 1) % opts.length;
				select(opts[next]?.channel ?? null);
			} else {
				e.preventDefault();
				selector.prev();
			}
		},
		ArrowDown: (e) => {
			if (!shown()) {
				e.preventDefault();
				const opts = options();
				const idx = opts.findIndex((o) => o.channel === selected());
				const next = (idx + 1) % opts.length;
				select(opts[next]?.channel ?? null);
			} else {
				e.preventDefault();
				selector.next();
			}
		},
		Escape: (e) => {
			if (shown()) {
				e.preventDefault();
				setShown(false);
			}
		},
		Enter: (e) => {
			const hovered = selector.getHovered();
			if (shown() && hovered) {
				e.preventDefault();
				select(hovered.channel);
			}
		},
	});

	// Only prevent default on wheel when dropdown is closed
	function handleWheel(e: WheelEvent) {
		if (shown()) return; // Let native scrolling work when open
		e.preventDefault();
		const opts = options();
		if (e.deltaY < 0) {
			const idx = opts.findIndex((o) => o.channel === selected());
			const next = (opts.length + idx - 1) % opts.length;
			select(opts[next]?.channel ?? null);
		} else if (e.deltaY > 0) {
			const idx = opts.findIndex((o) => o.channel === selected());
			const next = (idx + 1) % opts.length;
			select(opts[next]?.channel ?? null);
		}
	}

	const [value, setValue] = createSignal<string>("");
	createEffect(() => {
		const s = selected();
		if (document.activeElement === inputEl()) return;
		const opt = options().find((o) => o.channel === s);
		if (opt) {
			setValue(opt.label);
		} else {
			setValue("");
		}
	});

	// Scroll hovered item into view
	createEffect(() => {
		if (!shown()) return;
		const hovered = selector.getHovered();
		if (!hovered) return;
		const itemId = `channel-option-${hovered.channel.id}`;
		const el = document.getElementById(itemId);
		if (el) {
			el.scrollIntoView({ block: "nearest" });
		}
	});

	return {
		setSelected(channel: Channel) {
			setSelected(() => channel);
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
			const selectedChannel = () => selected();
			const hoveredChannel = () => selector.getHovered()?.channel;

			return (
				<div
					ref={setContainerEl}
					class={`dropdown-container ${props2.class ?? ""}`}
					style={props2.style}
				>
					<div
						class="channel-picker-input"
						style={{
							display: "flex",
							"align-items": "center",
							gap: "8px",
						}}
					>
						<Show when={selectedChannel()}>
							<ChannelIcon
								channel={selectedChannel()!}
								style="width: 20px; height: 20px; flex: none;"
							/>
						</Show>
						<input
							type="text"
							class="dropdown"
							ref={setInputEl}
							placeholder={props.placeholder ?? "select a channel..."}
							value={value()}
							onClick={() => {
								setShown(true);
								selector.setFilter("");
							}}
							onBlur={(e) => {
								queueMicrotask(() => setShown(false));
								const opt = options().find((o) => o.channel === selected());
								if (opt) {
									setValue(opt.label);
								} else {
									setValue("");
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
							aria-activedescendant={
								hoveredChannel()
									? `channel-option-${hoveredChannel()!.id}`
									: undefined
							}
							style={{ width: "100%" }}
						/>
					</div>
					<div
						class="dropdown-chevron-wrapper"
						onClick={() => setShown(!shown())}
					>
						<ChevronDown />
					</div>
					<Portal mount={props.mount ?? document.getElementById("overlay")!}>
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
									<For
										each={selector.filtered()}
										fallback={<li class="no-results">no channels</li>}
									>
										{(entry) => {
											const itemId = `channel-option-${entry.obj.channel.id}`;
											const isHovered = () =>
												entry.obj.channel.id === hoveredChannel()?.id;
											const isSelected = () =>
												entry.obj.channel.id === selectedChannel()?.id;

											return (
												<li
													id={itemId}
													role="option"
													onMouseOver={() => selector.setHovered(entry.obj)}
													onMouseDown={(e) => {
														e.preventDefault();
														select(entry.obj.channel);
													}}
													classList={{
														hovered: isHovered(),
														selected: isSelected(),
													}}
													aria-selected={isSelected()}
												>
													<div
														style={{
															display: "flex",
															"align-items": "center",
															gap: "8px",
														}}
													>
														<ChannelIcon
															channel={entry.obj.channel}
															style="width: 20px; height: 20px;"
														/>
														<span>{entry.obj.label}</span>
													</div>
												</li>
											);
										}}
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

export function ChannelPicker(props: {
	selected?: Channel | null;
	channels: () => Channel[];
	onSelect?: (channel: Channel | null) => void;
	style?: string;
	mount?: Element | DocumentFragment | null;
	placeholder?: string;
	filter?: (channel: Channel) => boolean;
}) {
	const picker = createChannelPicker({
		get selected() {
			return props.selected;
		},
		channels: props.channels,
		onSelect: props.onSelect,
		mount: props.mount,
		placeholder: props.placeholder,
		filter: props.filter,
	});

	return <picker.View style={props.style} />;
}

export function MultiChannelPicker(props: {
	selected: Channel[];
	channels: () => Channel[];
	onSelect: (channel: Channel) => void;
	onRemove: (channel: Channel) => void;
	style?: JSX.CSSProperties;
	placeholder?: string;
	mount?: Element | DocumentFragment | null;
	filter?: (channel: Channel) => boolean;
}) {
	const [shown, setShown] = createSignal(false);
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLUListElement>();
	const [containerEl, setContainerEl] = createSignal<HTMLDivElement>();
	const listboxId = createUniqueId();

	const PADDING = 16;
	const position = useFloating(containerEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [
			offset({ mainAxis: -1 }),
			flip({ padding: PADDING }),
			shift({ padding: PADDING }),
			size({
				padding: PADDING,
				apply({ availableHeight, elements }) {
					Object.assign(elements.floating.style, {
						maxHeight: `${Math.max(0, availableHeight)}px`,
					});
				},
			}),
		],
		placement: "bottom",
	});

	const selector = createSelect<ChannelPickerOption>();

	const options = createMemo(() => {
		const channels = props.channels();
		const filter = props.filter;
		return channels
			.filter((c) => !filter || filter(c))
			.map((c) => ({
				channel: c,
				label: c.name,
			}));
	});

	createEffect(() => {
		selector.setItems(options());
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
		Escape: (e) => {
			if (shown()) {
				e.preventDefault();
				setShown(false);
			}
		},
		Enter: (e) => {
			e.preventDefault();
			if (shown()) {
				const hovered = selector.getHovered();
				if (hovered) {
					props.onSelect(hovered.channel);
					selector.setFilter("");
					if (inputEl()) inputEl()!.value = "";
				}
			} else {
				setShown(true);
			}
		},
		Backspace: (e) => {
			if (selector.filtered().length === 0 && props.selected.length > 0) {
				props.onRemove(props.selected[props.selected.length - 1]);
			}
		},
	});

	function select(channel: Channel) {
		if (props.selected.includes(channel)) {
			props.onRemove(channel);
		} else {
			props.onSelect(channel);
		}
		selector.setFilter("");
		if (inputEl()) inputEl()!.value = "";
	}

	// Scroll hovered item into view
	createEffect(() => {
		if (!shown()) return;
		const hovered = selector.getHovered();
		if (!hovered) return;
		const itemId = `channel-option-${hovered.channel.id}`;
		const el = document.getElementById(itemId);
		if (el) {
			el.scrollIntoView({ block: "nearest" });
		}
	});

	return (
		<div
			ref={setContainerEl}
			class="dropdown multi-dropdown"
			onClick={() => inputEl()?.focus()}
			style={props.style}
		>
			<div class="multi-dropdown-selected">
				<For each={props.selected}>
					{(channel) => (
						<span class="chip">
							<ChannelIcon
								channel={channel}
								style="width: 16px; height: 16px; flex: none;"
							/>
							{channel.name}
							<button
								onClick={(e) => {
									e.stopPropagation();
									props.onRemove(channel);
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
					aria-activedescendant={
						selector.getHovered()
							? `channel-option-${selector.getHovered()!.channel.id}`
							: undefined
					}
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
							width: `${containerEl()?.offsetWidth || 0}px`,
						}}
					>
						<ul>
							<For
								each={selector.filtered()}
								fallback={<li class="no-results">no channels</li>}
							>
								{(entry) => {
									const itemId = `channel-option-${entry.obj.channel.id}`;
									const isHovered = () =>
										entry.obj.channel.id === selector.getHovered()?.channel.id;
									const isSelected = () =>
										props.selected.some((c) => c.id === entry.obj.channel.id);

									return (
										<li
											id={itemId}
											role="option"
											onMouseOver={() => selector.setHovered(entry.obj)}
											onMouseDown={(e) => {
												e.preventDefault();
												e.stopPropagation();
												select(entry.obj.channel);
											}}
											classList={{
												hovered: isHovered(),
												selected2: isSelected(),
											}}
											aria-selected={isSelected()}
											style={{
												display: "flex",
											}}
										>
											<div
												style={{
													display: "flex",
													"align-items": "center",
													gap: "8px",
												}}
											>
												<ChannelIcon
													channel={entry.obj.channel}
													style="width: 20px; height: 20px;"
												/>
												<span>{entry.obj.label}</span>
											</div>
										</li>
									);
								}}
							</For>
						</ul>
					</menu>
				</Show>
			</Portal>
		</div>
	);
}
