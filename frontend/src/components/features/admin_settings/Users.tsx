import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { throttle } from "@solid-primitives/scheduled";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { useApi, useUsers } from "@/api";
import { Search } from "@/atoms/Search";
import { Time } from "@/atoms/Time.tsx";
import { Avatar } from "@/components/shared/User";
import { useMenu } from "@/contexts/mod.tsx";
import { getDate } from "@/utils/general";

export function Users() {
	const { setMenu } = useMenu();
	const api = useApi();
	const [query, setQuery] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<any[]>([]);

	const throttledSearch = throttle(async (q: string) => {
		if (q.length > 0) {
			const results = await api.users.search(q);
			if (results && results.results) {
				setSearchResults(
					results.results
						.map((id: string) => api.users.cache.get(id))
						.filter(Boolean),
				);
			} else {
				setSearchResults([]);
			}
		} else {
			setSearchResults([]);
		}
	}, 500);

	createEffect(() => {
		throttledSearch(query());
	});

	const users = createMemo(() => [...api.users.cache.values()]);

	const fetchMore = () => {
		// Users are loaded from cache, no pagination needed
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) fetchMore();
			}
		},
	);

	return (
		<div class="room-settings-members">
			<h2>Users</h2>
			<Search
				placeholder="Search users..."
				onInput={setQuery}
				ref={(el) => queueMicrotask(() => el.focus())}
			/>
			<header>
				<div class="name">name</div>
				<div class="joined">registered</div>
			</header>
			<Show
				when={
					query().length > 0 ? searchResults().length > 0 : users().length > 0
				}
			>
				<ul>
					<For each={query().length > 0 ? searchResults() : users()}>
						{(user) => (
							<li>
								<div class="profile">
									<Avatar user={user} />
									<div>
										<h3 class="name">{user.name}</h3>
										<div class="dim">{user.id}</div>
									</div>
								</div>
								<div class="joined">
									<Show when={user.registered_at}>
										<Time date={getDate(user.registered_at!)} />
									</Show>
								</div>
								<div style="flex:1"></div>
								<button
									type="button"
									class="button"
									onClick={(e) => {
										e.stopPropagation();
										setMenu({
											type: "user",
											user_id: user.id,
											x: e.clientX,
											y: e.clientY,
											admin: true,
										});
									}}
								>
									options
								</button>
							</li>
						)}
					</For>
				</ul>
				<Show when={query().length === 0}>
					<div ref={setBottom}></div>
				</Show>
			</Show>
		</div>
	);
}
