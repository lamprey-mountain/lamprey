import { A } from "@solidjs/router";
import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { useApi } from "./api";
import { flags } from "./flags";
import { getThumbFromId } from "./media/util";
import { useCtx } from "./context";

type RoomNavConfig = Array<
	RoomNavItem | {
		type: "folder";
		name: string;
		items: RoomNavItem[];
	}
>;

type RoomNavItem = {
	type: "room";
	room_id: string;
} | {
	type: "view";
	name: string;
	uncategorized_channels: Array<ViewChannel>;
	categories: Array<ViewCategory>;
};

type ViewCategory =
	| {
		type: "custom";
		name: string;
		channels: Array<ViewChannel>;
	}
	| {
		type: "room";
		channel_id: string;
		room_id: string;
		nickname?: string;
	};

type ViewChannel = {
	channel_id: string;
	room_id?: string;
	nickname?: string;
};

export const RoomNav = () => {
	const api = useApi();
	const ctx = useCtx();
	const rooms = api.rooms.list();

	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(null);

	const getRoomOrder = () => {
		const config = ctx.userConfig().frontend.roomNav as RoomNavConfig;
		if (config && Array.isArray(config)) {
			return config
				.filter((item) => item.type === "room")
				.map((item) => item.room_id);
		}
		return [];
	};

	const reorderedRooms = () => {
		const roomOrder = getRoomOrder();
		const roomsList = rooms()?.items || [];

		if (roomOrder.length > 0) {
			return [...roomsList].sort((a, b) => {
				const aIndex = roomOrder.indexOf(a.id);
				const bIndex = roomOrder.indexOf(b.id);

				if (aIndex !== -1 && bIndex !== -1) {
					return aIndex - bIndex;
				}

				if (aIndex !== -1) return 1;
				if (bIndex !== -1) return -1;

				return 0;
			});
		}

		return roomsList;
	};

	const previewedRooms = createMemo(() => {
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;
		const rooms = reorderedRooms();

		if (!fromId || !toId || fromId === toId) {
			return rooms;
		}

		const order = rooms.map((r) => r.id);
		const fromIndex = order.indexOf(fromId);
		let toIndex = order.indexOf(toId);

		if (fromIndex === -1 || toIndex === -1) return rooms;

		if (after) {
			toIndex++;
		}

		if (fromIndex < toIndex) {
			toIndex--;
		}

		const [movedId] = order.splice(fromIndex, 1);
		order.splice(toIndex, 0, movedId);

		const roomMap = new Map(rooms.map((r) => [r.id, r]));
		return order.map((id) => roomMap.get(id)!);
	});

	const updateRoomOrder = (newOrder: string[]) => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				roomNav: newOrder.map(
					(room_id) => ({ type: "room", room_id }) as RoomNavItem,
				),
			},
		});
	};

	const handleDragStart = (e: DragEvent) => {
		const id = (e.currentTarget as HTMLElement).dataset.roomId;
		if (id) setDragging(id);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		const id = (e.currentTarget as HTMLElement).dataset.roomId;
		if (!id || id === dragging()) return;

		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;

		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) return;

		const order = reorderedRooms().map((r) => r.id);

		const fromIndex = order.indexOf(fromId);
		let toIndex = order.indexOf(toId);

		if (fromIndex === -1 || toIndex === -1) return;

		if (after) {
			toIndex++;
		}

		if (fromIndex < toIndex) {
			toIndex--;
		}

		const [movedRoom] = order.splice(fromIndex, 1);
		order.splice(toIndex, 0, movedRoom);

		updateRoomOrder(order);
	};

	onCleanup(() => {
		setDragging(null);
		setTarget(null);
	});

	return (
		<Show when={flags.has("two_tier_nav")}>
			<nav class="nav2">
				<ul>
					<li>
						<A href="/" end>
							home
						</A>
					</li>
					<For each={previewedRooms()}>
						{(room) => (
							<li
								draggable="true"
								class="menu-room"
								data-room-id={room.id}
								onDragStart={handleDragStart}
								onDragOver={handleDragOver}
								onDrop={handleDrop}
								onDragEnd={() => {
									setDragging(null);
									setTarget(null);
								}}
								classList={{
									dragging: dragging() === room.id,
									"drag-over": target()?.id === room.id && !target()?.after,
									"drag-over-after": target()?.id === room.id &&
										target()?.after,
								}}
							>
								<A draggable="false" href={`/room/${room.id}`} class="nav">
									<Show
										when={room.icon}
										fallback={<div class="avatar">{room.name}</div>}
									>
										<img
											src={getThumbFromId(room.icon!, 64)}
											class="avatar"
										/>
									</Show>
								</A>
							</li>
						)}
					</For>
				</ul>
			</nav>
		</Show>
	);
};
