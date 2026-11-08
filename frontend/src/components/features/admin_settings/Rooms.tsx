import { throttle } from "@solid-primitives/scheduled";
import { getTimestampFromUUID, type Room } from "sdk";
import { createEffect, createSignal, Show } from "solid-js";
import { useRooms } from "@/api";
import { Time } from "@/atoms/Time.tsx";
import { useMenu } from "@/contexts/mod.tsx";
import { useNavigate } from "@/contexts/router";
import { getThumbFromId } from "@/media/util.tsx";
import { ResourceList } from "./ResourceList";

export function Rooms() {
	const { setMenu } = useMenu();
	const nav = useNavigate();
	const rooms2 = useRooms();

	const [query, setQuery] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<Room[] | null>(null);
	const [searching, setSearching] = createSignal(false);

	const search = throttle(async (q: string) => {
		if (q.length === 0) {
			setSearchResults(null);
			return;
		}

		setSearching(true);
		try {
			const results = await rooms2.search(q);
			setSearchResults(results.rooms ?? []);
		} catch {
			setSearchResults([]);
		} finally {
			setSearching(false);
		}
	}, 500);

	createEffect(() => {
		search(query());
	});

	const list = rooms2.useListAll();

	const loadPage = () => {
		rooms2.loadListAll().catch(() => {});
	};

	const pagedRooms = () =>
		list.ids
			.map((id) => rooms2.cache.get(id))
			.filter((room) => room !== undefined);

	const items = () =>
		query().length > 0 ? (searchResults() ?? []) : pagedRooms();

	const hasMore = () => query().length === 0 && list.has_more;

	const openRoom = (room: Room) => nav(`/room/${room.id}`);

	return (
		<ResourceList
			title="Rooms"
			class="room-settings-members"
			query={query}
			onSearch={setQuery}
			placeholder="Search rooms..."
			header={
				<>
					<div class="name">name</div>
					<div class="joined">created</div>
					<div style="flex:1">members</div>
				</>
			}
			items={items}
			itemKey={(room) => room.id}
			hasMore={hasMore}
			onLoadMore={loadPage}
			loading={() => searching() || list.isLoading}
			empty={query().length > 0 ? "no rooms match this search" : "no rooms"}
			renderItem={(room) => (
				<>
					<button
						type="button"
						class="profile"
						style="text-align:start;background:none;border:none;padding:0;cursor:pointer"
						onClick={() => openRoom(room)}
					>
						<Show
							when={room.icon}
							fallback={<div class="avatar">{room.name}</div>}
						>
							<img src={getThumbFromId(room.icon!, 64)} class="avatar" />
						</Show>
						<div>
							<h3 class="name">{room.name}</h3>
							<div class="dim">{room.id}</div>
						</div>
					</button>
					<div class="joined">
						<Time date={getTimestampFromUUID(room.id)} />
					</div>
					<div class="admin-badges">
						<Show when={room.deleted_at}>
							<span class="admin-badge danger">deleted</span>
						</Show>
						<Show when={room.public}>
							<span class="admin-badge">public</span>
						</Show>
						<Show when={room.quarantined}>
							<span class="admin-badge danger">quarantined</span>
						</Show>
						<span class="dim">
							{room.online_count} online / {room.member_count} members /{" "}
							{room.channel_count} channels
						</span>
					</div>
					<div style="flex:1"></div>
					<button
						type="button"
						class="button"
						onClick={(e) => {
							e.stopPropagation();
							setMenu({
								type: "room",
								room_id: room.id,
								x: e.clientX,
								y: e.clientY,
								admin: true,
							});
						}}
					>
						options
					</button>
				</>
			)}
		/>
	);
}
