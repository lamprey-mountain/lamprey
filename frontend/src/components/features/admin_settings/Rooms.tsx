import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { throttle } from "@solid-primitives/scheduled";
import { getTimestampFromUUID } from "sdk";
import { createEffect, createSignal, For, Show } from "solid-js";
import { useRooms } from "@/api";
import { Time } from "@/atoms/Time.tsx";
import { getThumbFromId } from "@/media/util.tsx";

export function Rooms() {
	const rooms2 = useRooms();
	const [query, setQuery] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<any[]>([]);

	const throttledSearch = throttle(async (q: string) => {
		if (q.length > 0) {
			const results = await rooms2.search(q);
			if (results && results.results && results.rooms) {
				const roomMap = new Map(results.rooms.map((r: any) => [r.id, r]));
				setSearchResults(
					results.results.map((id: string) => roomMap.get(id)).filter(Boolean),
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

	const rooms = rooms2.useListAll();

	const fetchMore = () => {
		if (rooms.has_more) {
			rooms2.fetchListAll(rooms.cursor ?? undefined);
		}
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
			<h2>Rooms</h2>
			<input
				type="text"
				placeholder="Search rooms..."
				onInput={(e) => setQuery(e.currentTarget.value)}
			/>
			<header>
				<div class="name">name</div>
				<div class="joined">created</div>
			</header>
			<Show
				when={
					query().length > 0 ? searchResults().length > 0 : rooms.ids.length > 0
				}
			>
				<ul>
					<For each={query().length > 0 ? searchResults() : rooms.ids}>
						{(item) => {
							const room = query().length > 0 ? item : rooms2.cache.get(item);
							if (!room) return null;
							return (
								<li>
									<div class="profile">
										<Show
											when={room.icon}
											fallback={<div class="avatar">{room.name}</div>}
										>
											<img
												src={getThumbFromId(room.icon!, 64)}
												class="avatar"
											/>
										</Show>
										<div>
											<h3 class="name">{room.name}</h3>
											<div class="dim">{room.id}</div>
										</div>
									</div>
									<div class="joined">
										<Time date={getTimestampFromUUID(room.id)} />
									</div>
									<div style="flex:1"></div>
									<button type="button" class="button">
										options
									</button>
								</li>
							);
						}}
					</For>
				</ul>
				<Show when={query().length === 0}>
					<div ref={setBottom}></div>
				</Show>
			</Show>
		</div>
	);
}
