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
import { ChannelIcon } from "@/avatar/ChannelIcon.tsx";
import { createKeybinds } from "@/lib/keybinds";

export type ChannelPickerOption = {
	channel: Channel;
	label: string;
};

const ChevronDown = () => (
	<svg
		aria-hidden="true"
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

	const filtered = createMemo(() => {
		const filter = getFilter();
		const items = getItems();
		if (!filter) return items.map((i) => ({ obj: i }));

		return go(filter, items, {
			key: "label",
			all: true,
		}) as unknown as Array<Fuzzysort.KeyResult<T>>;
	});

	createEffect(() => {
		const list = filtered();
		const currentHovered = getHovered();
		if (!list.some((i) => i.obj === currentHovered)) {
			setHovered(() => list[0]?.obj ?? null);
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
			const hovered = getHovered();
			const idx = hovered ? list.findIndex((i) => i.obj === hovered) : -1;
			setHovered(() => list[(idx + 1) % list.length]?.obj ?? null);
		},
		prev() {
			const list = filtered();
			if (list.length === 0) return;
			const hovered = getHovered();
			const idx = hovered ? list.findIndex((i) => i.obj === hovered) : 0;
			setHovered(
				() => list[(list.length + idx - 1) % list.length]?.obj ?? null,
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
	required?: boolean;
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
		const required = props.required ?? true;
		const opts = channels
			.filter((c) => !filter || filter(c))
			.map((c) => ({
				channel: c,
				label: c.name,
			}));
		if (!required) {
			opts.unshift({
				channel: null as unknown as Channel,
				label: "no channel",
			});
		}
		return opts;
	});

	createEffect(() => {
		selector.setItems(options());
	});

	createEffect(() => {
		const pSelected = props.selected;
		if (pSelected !== undefined) setSelected(() => pSelected);
	});

	const [value, setValue] = createSignal<string>("");

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

	function handleWheel(e: WheelEvent) {
		if (shown()) return;
		e.preventDefault();
		const opts = options();
		const currentSelected = selected();
		if (e.deltaY < 0) {
			const idx = opts.findIndex((o) => o.channel === currentSelected);
			const next = (opts.length + idx - 1) % opts.length;
			select(opts[next]?.channel ?? null);
		} else if (e.deltaY > 0) {
			const idx = opts.findIndex((o) => o.channel === currentSelected);
			const next = (idx + 1) % opts.length;
			select(opts[next]?.channel ?? null);
		}
	}

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

	createEffect(() => {
		if (!shown()) return;
		const hovered = selector.getHovered();
		if (!hovered) return;
		const itemId = `channel-option-${hovered.channel?.id}`;
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
							{(sc) => (
								<ChannelIcon
									channel={sc()}
									style="width: 20px; height: 20px; flex: none;"
								/>
							)}
						</Show>
						<input
							type="text"
							class="dropdown"
							classList={{
								"no-channel": selected() === null,
							}}
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
								setValue(opt?.label ?? "");
								props.onBlur?.(e);
							}}
							onInput={(e) => {
								const val = e.currentTarget.value;
								setValue(val);
								selector.setFilter(val);
								if (val) setShown(true);
								props.onInput?.(val);
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
									? `channel-option-${hoveredChannel()?.id}`
									: undefined
							}
							style={{ width: "100%" }}
						/>
					</div>
					{/* Fix: Changed div to button for a11y */}
					<button
						type="button"
						class="dropdown-chevron-wrapper"
						onClick={() => setShown(!shown())}
						aria-label="Toggle dropdown"
					>
						<ChevronDown />
					</button>
					<Portal
						mount={
							props.mount ?? document.getElementById("overlay") ?? document.body
						}
					>
						<Show when={shown()}>
							<menu
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
								<ul role="listbox">
									<For
										each={selector.filtered()}
										fallback={<li class="no-results">no channels</li>}
									>
										{(entry) => {
											const itemId = `channel-option-${entry.obj.channel?.id ?? "none"}`;
											const isHovered = () =>
												(entry.obj.channel?.id ?? null) ===
												(hoveredChannel()?.id ?? null);
											const isSelected = () =>
												(entry.obj.channel?.id ?? null) ===
												(selectedChannel()?.id ?? null);

											return (
												<li
													id={itemId}
													role="option"
													tabindex="-1"
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
														<Show
															when={entry.obj.channel}
															fallback={
																<span class="no-channel">no channel</span>
															}
														>
															<ChannelIcon
																channel={entry.obj.channel}
																style="width: 20px; height: 20px;"
															/>
															<span>{entry.obj.label}</span>
														</Show>
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
	required?: boolean;
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
		required: props.required,
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
					const input = inputEl();
					if (input) input.value = "";
				}
			} else {
				setShown(true);
			}
		},
		Backspace: () => {
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
		const input = inputEl();
		if (input) input.value = "";
	}

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
			onKeyDown={(e) => {
				if (e.key === "Enter") inputEl()?.focus();
			}}
			style={props.style}
			role="presentation"
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
								type="button"
								class="button"
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
						selector.setFilter(e.currentTarget.value);
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
							? `channel-option-${selector.getHovered()?.channel.id}`
							: undefined
					}
				/>
			</div>
			{/* Fix: Static interaction div -> button */}
			<button
				type="button"
				class="dropdown-chevron-wrapper"
				onClick={(e) => {
					e.stopPropagation();
					setShown(!shown());
				}}
				aria-label="Toggle listbox"
			>
				<ChevronDown />
			</button>
			<Portal mount={props.mount ?? document.body}>
				<Show when={shown()}>
					<menu
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
						<ul role="listbox">
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
											tabindex="-1"
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
											style={{ display: "flex" }}
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
