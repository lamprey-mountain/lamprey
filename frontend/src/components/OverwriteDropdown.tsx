import {
	createEffect,
	createMemo,
	createSignal,
	createUniqueId,
	For,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { useApi } from "../api.tsx";
import { createKeybinds } from "../keybinds.tsx";
import type { Role, RoomMemberSearchResponse, User } from "sdk";
import { throttle } from "@solid-primitives/scheduled";

type OverwriteOption = {
	id: string;
	name: string;
	type: "Role" | "User";
	icon?: string;
};

export function OverwriteDropdown(props: {
	room_id: string;
	onSelect: (id: string, type: "Role" | "User") => void;
	excludeIds?: string[];
}) {
	const api = useApi();
	const [shown, setShown] = createSignal(false);
	const [query, setQuery] = createSignal("");
	const [inputEl, setInputEl] = createSignal<HTMLInputElement>();
	const [dropdownEl, setDropdownEl] = createSignal<HTMLDivElement>();
	const [hoveredIndex, setHoveredIndex] = createSignal(0);

	const roles = api.roles.list(() => props.room_id);
	const [memberResults, setMemberResults] = createSignal<
		RoomMemberSearchResponse
	>({ room_members: [], users: [] });

	const throttledSearch = throttle(
		async (q: string) => {
			if (q.length > 0) {
				const results = await api.room_members.search(props.room_id, q);
				setMemberResults(results);
			} else {
				setMemberResults({ room_members: [], users: [] });
			}
		},
		300,
	);

	createEffect(() => {
		throttledSearch(query());
	});

	const options = createMemo(() => {
		const q = query().toLowerCase();
		const exclude = new Set(props.excludeIds || []);

		const roleOptions: OverwriteOption[] = (roles()?.items || [])
			.filter((r) =>
				r.id !== props.room_id && // exclude @everyone
				!exclude.has(r.id) &&
				r.name.toLowerCase().includes(q)
			)
			.map((r) => ({ id: r.id, name: r.name, type: "Role" }));

		const userOptions: OverwriteOption[] = memberResults().users
			.filter((u) => !exclude.has(u.id))
			.map((u) => ({ id: u.id, name: u.name, type: "User" }));

		return [...roleOptions, ...userOptions];
	});

	createEffect(() => {
		if (hoveredIndex() >= options().length) {
			setHoveredIndex(Math.max(0, options().length - 1));
		}
	});

	const position = useFloating(inputEl, dropdownEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset({ mainAxis: -1 }), flip()],
		placement: "bottom-start",
	});

	const select = (opt: OverwriteOption) => {
		props.onSelect(opt.id, opt.type);
		setShown(false);
		setQuery("");
	};

	const binds = createKeybinds({
		"ArrowUp": (e) => {
			e.preventDefault();
			setHoveredIndex((i) => (i > 0 ? i - 1 : options().length - 1));
		},
		"ArrowDown": (e) => {
			e.preventDefault();
			setHoveredIndex((i) => (i < options().length - 1 ? i + 1 : 0));
		},
		"Enter": (e) => {
			e.preventDefault();
			const opt = options()[hoveredIndex()];
			if (opt) select(opt);
		},
		"Escape": () => setShown(false),
	});

	const listboxId = createUniqueId();

	return (
		<div class="overwrite-dropdown">
			<input
				ref={setInputEl}
				type="text"
				class="dropdown"
				placeholder="Add role or member..."
				value={query()}
				onInput={(e) => {
					setQuery(e.currentTarget.value);
					setShown(true);
				}}
				onFocus={() => setShown(true)}
				onBlur={() => setTimeout(() => setShown(false), 200)}
				onKeyDown={binds}
				role="combobox"
				aria-autocomplete="list"
				aria-expanded={shown()}
				aria-controls={listboxId}
			/>
			<Portal>
				<Show when={shown() && options().length > 0}>
					<div
						ref={setDropdownEl}
						id={listboxId}
						role="listbox"
						class="dropdown-items floating"
						style={{
							position: position.strategy,
							top: `${position.y ?? 0}px`,
							left: `${position.x ?? 0}px`,
							width: `${inputEl()?.offsetWidth}px`,
							"z-index": 10000,
						}}
					>
						<ul>
							<For each={options()}>
								{(opt, i) => (
									<li
										role="option"
										aria-selected={i() === hoveredIndex()}
										onMouseEnter={() => setHoveredIndex(i())}
										onMouseDown={(e) => {
											e.preventDefault();
											select(opt);
										}}
										classList={{
											hovered: i() === hoveredIndex(),
										}}
									>
										<Show when={opt.type === "User"}>
											<span class="prefix">@</span>
										</Show>
										{opt.name}
									</li>
								)}
							</For>
						</ul>
					</div>
				</Show>
			</Portal>
		</div>
	);
}
