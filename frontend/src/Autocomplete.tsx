import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import { useCtx } from "./context";
import { useApi } from "./api";
import { go } from "fuzzysort";
import { type User } from "sdk";

export const Autocomplete = () => {
	const ctx = useCtx();
	const api = useApi();

	const [allUsers, setAllUsers] = createSignal<User[]>([]);

	createEffect(on(ctx.autocomplete, (state) => {
		console.log(state);
		if (state?.type === "mention") {
			const channelId = state.channelId;
			const channel = api.channels.cache.get(channelId);
			const roomId = channel?.room_id;

			const threadMembers = api.thread_members.list(() => channelId)();
			const roomMembers = roomId
				? api.room_members.list(() => roomId)()
				: undefined;

			const userIds = new Set<string>();
			threadMembers?.items.forEach((m) => userIds.add(m.user_id));
			roomMembers?.items.forEach((m) => userIds.add(m.user_id));

			const users = [...userIds].map((id) => api.users.cache.get(id)).filter(
				Boolean,
			) as User[];
			setAllUsers(users);
			console.log("all users", users);
		}
	}));

	const [filtered, setFiltered] = createSignal<Fuzzysort.KeyResult<User>[]>([]);
	const [hoveredIndex, setHoveredIndex] = createSignal(0);
	const hovered = () => filtered()[hoveredIndex()]?.obj;

	createEffect(() => {
		const state = ctx.autocomplete();
		if (state?.type === "mention") {
			const results = go(state.query, allUsers(), {
				key: "name",
				limit: 10,
				all: true,
			});
			setFiltered(results as any);
			if (hoveredIndex() >= results.length) {
				setHoveredIndex(0);
			}
		}
	});

	const select = (user: User) => {
		const state = ctx.autocomplete();
		if (state?.type === "mention") {
			state.onSelect(user.id, user.name);
			ctx.setAutocomplete(null);
		}
	};

	const onKeyDown = (e: KeyboardEvent) => {
		if (!ctx.autocomplete()) return;

		if (e.key === "ArrowUp") {
			e.preventDefault();
			e.stopPropagation();
			setHoveredIndex((i) => (i - 1 + filtered().length) % filtered().length);
		} else if (e.key === "ArrowDown") {
			e.preventDefault();
			e.stopPropagation();
			setHoveredIndex((i) => (i + 1) % filtered().length);
		} else if (e.key === "Enter" || e.key === "Tab") {
			e.preventDefault();
			e.stopPropagation();
			const user = hovered();
			if (user) {
				select(user);
			}
		} else if (e.key === "Escape") {
			e.preventDefault();
			e.stopPropagation();
			ctx.setAutocomplete(null);
		}
	};

	createEffect(() => {
		if (ctx.autocomplete()) {
			document.addEventListener("keydown", onKeyDown, { capture: true });
			onCleanup(() => {
				document.removeEventListener("keydown", onKeyDown, { capture: true });
			});
		}
	});

	createEffect(() => {
		console.log("autocomplete", ctx.autocomplete(), filtered());
	});

	return (
		<Show when={ctx.autocomplete() && filtered().length > 0}>
			<div class="autocomplete">
				<For each={filtered()}>
					{(result, i) => (
						<div
							class="item"
							classList={{ hovered: i() === hoveredIndex() }}
							onMouseEnter={() => setHoveredIndex(i())}
							onMouseDown={(e) => {
								e.preventDefault();
								select(result.obj);
							}}
						>
							{result.obj.name}
						</div>
					)}
				</For>
			</div>
		</Show>
	);
};
