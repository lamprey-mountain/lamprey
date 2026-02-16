import { A } from "@solidjs/router";
import {
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "./api";
import { flags } from "./flags";
import { getThumbFromId } from "./media/util";
import { useCtx } from "./context";
import type { Room } from "sdk";
import icHome from "./assets/home.png";
import icFolder1 from "./assets/folder-1.png";
import { RoomIcon } from "./User";

export type RoomNavItem =
	| {
		type: "room";
		room_id: string;
	}
	| {
		type: "folder";
		id: string;
		name: string;
		items: { type: "room"; room_id: string }[];
	}
	| {
		type: "view";
		name: string;
		// Omitting view-specific properties for now
	};

/*
TODO: room nav views

type RoomNavToplevelItem = RoomNavItem & {
	type: "folder",
	name: string,
	items: RoomNavItem[],
}

type RoomNavItem = {
	type: "room",
	room_id: string,
} | {
	type: "view",
	name: "string",
	uncategorized_channels: Array<ViewChannel>;
	categories: Array<ViewCategory>;
}

// either a local category for this view or a category from a room
type ViewCategory =
	| {
		name: string;
		channels: Array<Channel>;
	}
	| {
		id: string;
		room_id: string;
		nickname?: string;
	};

type ViewChannel = {
	id: string;
	room_id?: string;
	nickname?: string;
};
*/

type RoomNavConfig = Array<RoomNavItem>;

export const RoomNav = () => {
	const api = useApi();
	const ctx = useCtx();
	const rooms = api.rooms.list();

	const getRoomMentionCount = (roomId: string) => {
		let totalMentions = 0;
		for (const channel of api.channels.cache.values()) {
			if (channel.room_id === roomId && channel.mention_count) {
				totalMentions += channel.mention_count;
			}
		}
		return totalMentions;
	};

	const getRoomUnread = (roomId: string) => {
		for (const channel of api.channels.cache.values()) {
			if (
				channel.room_id === roomId && channel.is_unread &&
				channel.type !== "Voice"
			) {
				return true;
			}
		}
		return false;
	};

	const getFolderUnread = (folder: { items: Room[] }) => {
		return folder.items.some((room) => getRoomUnread(room.id));
	};

	const getFolderMentionCount = (folder: { items: Room[] }) => {
		return folder.items.reduce(
			(acc, room) => acc + getRoomMentionCount(room.id),
			0,
		);
	};

	const [dragging, setDragging] = createSignal<
		{
			id: string;
			type: "room" | "folder";
		} | null
	>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(
		null,
	);
	const [collapsedFolders, setCollapsedFolders] = createSignal(
		new Set<string>(),
	);
	const [folderPreview, setFolderPreview] = createSignal<string | null>(null);
	let folderTimer: number | undefined;

	const getConfig = (): RoomNavConfig => {
		const config = ctx.userConfig().frontend.roomNav as RoomNavConfig;
		if (config && Array.isArray(config)) {
			return JSON.parse(JSON.stringify(config)); // Deep copy
		}
		return [];
	};

	const reorderedItems = createMemo(() => {
		let config = getConfig();
		const roomsList = rooms()?.items || [];
		const roomMap = new Map(roomsList.map((r) => [r.id, r]));

		const orderedIds = new Set<string>();

		// Migration for old configs
		for (const item of config) {
			if (item.type === "folder" && !item.id) {
				item.id = crypto.randomUUID();
			}
		}

		const mappedConfig: (
			| Room
			| { type: "folder"; id: string; name: string; items: Room[] }
			| { type: "view"; name: string }
		)[] = [];

		for (const item of config) {
			if (item.type === "room") {
				const room = roomMap.get(item.room_id);
				if (room) {
					mappedConfig.push(room);
					orderedIds.add(room.id);
				}
			} else if (item.type === "folder") {
				const folderItems = item.items
					.map((i) => {
						const room = roomMap.get(i.room_id);
						if (room) {
							orderedIds.add(room.id);
							return room;
						}
						return null;
					})
					.filter((r): r is Room => !!r);

				if (folderItems.length > 0) {
					mappedConfig.push({
						type: "folder",
						id: item.id,
						name: item.name,
						items: folderItems,
					});
				}
			} else if (item.type === "view") {
				mappedConfig.push(item);
			}
		}

		const unordered = roomsList.filter((r) => !orderedIds.has(r.id));
		return [...unordered, ...mappedConfig];
	});

	const previewedItems = createMemo(() => {
		const fromId = dragging()?.id;
		const toId = target()?.id;
		const after = target()?.after;
		const items = reorderedItems();
		const creatingFolder = folderPreview();

		if (!fromId) return items;

		const newItems = items.map((item) => {
			if (item.type === "folder") {
				return { ...item, items: [...item.items] };
			}
			return { ...item };
		});

		const findItem = (id: string, list: any[]) => {
			for (let i = 0; i < list.length; i++) {
				const item = list[i];
				const itemId = item.type === "folder"
					? item.id
					: item.type === "view"
					? `view-${item.name}`
					: item.id;
				if (itemId === id) {
					return { item, index: i, parent: null, parentList: list };
				}
				if (item.type === "folder") {
					for (let j = 0; j < item.items.length; j++) {
						if (item.items[j].id === id) {
							return {
								item: item.items[j],
								index: j,
								parent: item,
								parentList: item.items,
							};
						}
					}
				}
			}
			return null;
		};

		if (
			creatingFolder && toId === creatingFolder && fromId !== creatingFolder
		) {
			const fromResult = findItem(fromId, newItems);
			const toResult = findItem(toId, newItems);
			if (!fromResult || !toResult || fromResult.parent || toResult.parent) {
				return items;
			}

			const [fromItem] = fromResult.parentList.splice(fromResult.index, 1);
			const toIndex = newItems.findIndex((i: any) => i.id === toId);
			if (toIndex === -1) return items;

			newItems[toIndex] = {
				type: "folder",
				id: crypto.randomUUID(),
				name: "New Folder",
				items: [newItems[toIndex], fromItem],
			};
			return newItems;
		}

		if (!toId || fromId === toId) return items;

		const from = findItem(fromId, newItems);
		const to = findItem(toId, newItems);

		if (!from || !to) return items;

		if (
			from.item.type === "folder" && (to.parent || to.item.type === "folder")
		) {
			return items;
		}

		const [movedItem] = from.parentList.splice(from.index, 1);

		if (to.item.type === "folder" && movedItem.id) {
			to.item.items.push(movedItem);
		} else if (to.parent) {
			let insertIndex = to.index + (after ? 1 : 0);
			to.parent.items.splice(insertIndex, 0, movedItem);
		} else {
			let insertIndex = to.index + (after ? 1 : 0);
			if (!from.parent && from.index < to.index) {
				insertIndex--;
			}
			newItems.splice(insertIndex, 0, movedItem);
		}

		return newItems.filter(
			(item: any) => !(item.type === "folder" && item.items.length === 0),
		);
	});

	const updateRoomOrder = (newConfig: RoomNavConfig) => {
		for (const item of newConfig) {
			if (item.type === "folder" && !item.id) {
				item.id = crypto.randomUUID();
			}
		}
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				roomNav: newConfig,
			},
		});
	};

	const handleDragStart = (e: DragEvent, type: "room" | "folder") => {
		const id = (e.currentTarget as HTMLElement).dataset.id;
		if (id) setDragging({ id, type });
		e.stopPropagation();
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const targetEl = e.currentTarget as HTMLElement;
		const id = targetEl.dataset.id;
		const toType = targetEl.dataset.type;

		if (!id || !dragging() || id === dragging()?.id) {
			clearTimeout(folderTimer);
			setFolderPreview(null);
			return;
		}

		const rect = targetEl.getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}

		const fromType = dragging()?.type;

		if (fromType === "room" && toType === "room") {
			if (folderPreview() !== id) {
				clearTimeout(folderTimer);
				folderTimer = window.setTimeout(() => setFolderPreview(id), 1000);
			}
		} else {
			clearTimeout(folderTimer);
			setFolderPreview(null);
		}
	};

	const handleDragLeave = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		clearTimeout(folderTimer);
		setFolderPreview(null);
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const fromId = dragging()?.id;
		const toId = target()?.id;
		const after = target()?.after;
		const creatingFolder = folderPreview();

		clearTimeout(folderTimer);
		setFolderPreview(null);
		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) return;

		let config = getConfig();
		if (config.length === 0) {
			config = (rooms()?.items || []).map((r) => ({
				type: "room",
				room_id: r.id,
			}));
		}

		if (creatingFolder && fromId !== creatingFolder) {
			const newConfig: RoomNavConfig = [];
			let folderCreated = false;
			for (const item of config) {
				if (item.type === "room" && item.room_id === fromId) continue;
				if (item.type === "room" && item.room_id === creatingFolder) {
					newConfig.push({
						type: "folder",
						id: crypto.randomUUID(),
						name: "New Folder",
						items: [
							{ type: "room", room_id: creatingFolder },
							{ type: "room", room_id: fromId },
						],
					});
					folderCreated = true;
				} else {
					newConfig.push(item);
				}
			}
			if (folderCreated) {
				updateRoomOrder(newConfig);
				return;
			}
		}

		const findItem = (id: string) => {
			for (let i = 0; i < config.length; i++) {
				const item = config[i];
				if (item.type === "room" && item.room_id === id) {
					return { item, index: i, parent: null };
				}
				if (item.type === "folder" && item.id === id) {
					return { item, index: i, parent: null };
				}
				if (item.type === "folder") {
					for (let j = 0; j < item.items.length; j++) {
						if (item.items[j].room_id === id) {
							return { item: item.items[j], index: j, parent: item };
						}
					}
				}
			}
			return null;
		};

		const from = findItem(fromId);
		const to = findItem(toId);

		if (!from || !to) return;

		if (
			from.item.type === "folder" && (to.parent || to.item.type === "folder")
		) {
			return;
		}

		const [movedItem] = from.parent
			? from.parent.items.splice(from.index, 1)
			: config.splice(from.index, 1);

		if (to.item.type === "folder" && movedItem.type === "room") {
			to.item.items.push(movedItem);
		} else if (to.parent) {
			const insertIndex = to.index + (after ? 1 : 0);
			to.parent.items.splice(
				insertIndex,
				0,
				movedItem as { type: "room"; room_id: string },
			);
		} else {
			let insertIndex = to.index + (after ? 1 : 0);
			if (!from.parent && from.index < to.index) {
				insertIndex--;
			}
			config.splice(insertIndex, 0, movedItem);
		}

		const finalConfig = config.filter(
			(item) => !(item.type === "folder" && item.items.length === 0),
		);

		updateRoomOrder(finalConfig);
	};

	onCleanup(() => {
		clearTimeout(folderTimer);
		setDragging(null);
		setTarget(null);
	});

	const toggleFolder = (id: string) => {
		setCollapsedFolders((prev) => {
			const next = new Set(prev);
			if (next.has(id)) next.delete(id);
			else next.add(id);
			return next;
		});
	};

	const RoomItem = (props: { room: Room }) => {
		const mentionCount = () => getRoomMentionCount(props.room.id);

		return (
			<li
				draggable="true"
				class="menu-room room-item"
				data-id={props.room.id}
				data-room-id={props.room.id}
				data-type="room"
				onDragStart={(e) => handleDragStart(e, "room")}
				onDragOver={handleDragOver}
				onDragLeave={handleDragLeave}
				onDrop={handleDrop}
				onDragEnd={() => {
					setDragging(null);
					setTarget(null);
					clearTimeout(folderTimer);
					setFolderPreview(null);
				}}
				classList={{
					dragging: dragging()?.id === props.room.id,
					"drag-over": target()?.id === props.room.id && !target()?.after,
					"drag-over-after": target()?.id === props.room.id && target()?.after,
					"folder-preview": folderPreview() === props.room.id,
					"no-icon": !props.room.icon,
					unread: getRoomUnread(props.room.id),
				}}
			>
				<A draggable="false" href={`/room/${props.room.id}`} class="nav">
					<Show
						when={props.room.icon}
						fallback={<div class="avatar">{props.room.name}</div>}
					>
						<RoomIcon room={props.room} mentionCount={mentionCount()} />
					</Show>
				</A>
			</li>
		);
	};

	return (
		<Show when={flags.has("two_tier_nav")}>
			<nav id="room-nav">
				<ul>
					<li class="home-item">
						<A href="/" end>
							<img src={icHome} class="icon" />
						</A>
					</li>
					<For each={previewedItems()}>
						{(item) => (
							<Switch>
								<Match when={item.type === "folder" && item} keyed>
									{(folder) => (
										<div
											class="room-folder"
											data-id={folder.id}
											data-type="folder"
											draggable="true"
											onDragStart={(e) => handleDragStart(e, "folder")}
											onDragOver={handleDragOver}
											onDrop={handleDrop}
											onDragLeave={handleDragLeave}
											classList={{
												dragging: dragging()?.id === folder.id,
												"drag-over": target()?.id === folder.id,
												"preview": folderPreview() &&
													folder.items.some((room) =>
														room.id === folderPreview()
													),
												collapsed: collapsedFolders().has(folder.id),
												unread: getFolderUnread(folder),
											}}
										>
											<div
												class="folder-header"
												onClick={() => toggleFolder(folder.id)}
												onContextMenu={(e) => {
													e.preventDefault();
													queueMicrotask(() => {
														ctx.setMenu({
															x: e.clientX,
															y: e.clientY,
															type: "folder",
															folder_id: folder.id,
														});
													});
												}}
											>
												<img src={icFolder1} class="icon" />
											</div>
											<Show when={!collapsedFolders().has(folder.id)}>
												<ul>
													<For each={folder.items}>
														{(room) => <RoomItem room={room} />}
													</For>
												</ul>
											</Show>
										</div>
									)}
								</Match>
								<Match when={item.type === "view"} keyed>
									{(view) => (
										<li
											class="menu-room"
											data-id={`view-${view.name}`}
											data-type="view"
										>
											<A href="#" class="nav">
												<div class="avatar">{view.name.substring(0, 2)}</div>
											</A>
										</li>
									)}
								</Match>
								<Match when={"id" in item} keyed>
									<RoomItem room={item as Room} />
								</Match>
							</Switch>
						)}
					</For>
				</ul>
			</nav>
		</Show>
	);
};
