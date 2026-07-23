import { A, useParams } from "@solidjs/router";
import type { Room } from "sdk";
import {
	createMemo,
	createSelector,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useChannels, useRooms } from "@/api";
import { useCtx } from "@/app/context";
import icFolder1 from "@/assets/folder-1.png";
import icHome from "@/assets/home.png";
import { Icon } from "@/atoms/Icon";
import { useMenu } from "@/contexts/mod";
import { useRoomDnd } from "@/hooks/useRoomDnd";
import {
	type RoomNavFocusItem,
	useRoomNavKeybinds,
} from "@/hooks/useRoomNavKeybinds";
import { flags } from "@/lib/flags";
import type { RoomNavConfig } from "@/types/room-nav";
import { RoomIcon } from "./User";

export const RoomNav = () => {
	const rooms2 = useRooms();
	const channels2 = useChannels();
	const ctx = useCtx();
	const { setMenu } = useMenu();
	const rooms = createMemo(() => [...rooms2.cache.values()]);

	const getRoomMentionCount = (roomId: string) => {
		let totalMentions = 0;
		for (const channel of channels2.cache.values()) {
			if (channel.room_id === roomId && channel.mention_count) {
				totalMentions += channel.mention_count;
			}
		}
		return totalMentions;
	};

	const getRoomUnread = (roomId: string) => {
		for (const channel of channels2.cache.values()) {
			if (
				channel.room_id === roomId &&
				channel.is_unread &&
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

	// TODO: render folder mention count
	const _getFolderMentionCount = (folder: { items: Room[] }) => {
		return folder.items.reduce(
			(acc, room) => acc + getRoomMentionCount(room.id),
			0,
		);
	};

	const getConfig = (): RoomNavConfig => {
		const config = ctx.preferences().frontend.roomNav as RoomNavConfig;
		if (config && Array.isArray(config)) {
			return JSON.parse(JSON.stringify(config)); // Deep copy
		}
		return [];
	};

	const updateRoomOrder = (newConfig: RoomNavConfig) => {
		for (const item of newConfig) {
			if (item.type === "folder" && !item.id) {
				item.id = crypto.randomUUID();
			}
		}
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				roomNav: newConfig,
			},
		});
	};

	const dnd = useRoomDnd({
		getConfig,
		setConfig: updateRoomOrder,
		getDefaultConfig: () =>
			rooms().map((r) => ({ type: "room", room_id: r.id })),
		rooms,
	});

	const [collapsedFolders, setCollapsedFolders] = createSignal(
		new Set<string>(),
	);

	const toggleFolder = (id: string) => {
		setCollapsedFolders((prev) => {
			const next = new Set(prev);
			if (next.has(id)) next.delete(id);
			else next.add(id);
			return next;
		});
	};

	const params = useParams();

	const navItems = createMemo(() => {
		const items: RoomNavFocusItem[] = [];
		items.push({ id: "home", type: "home", folderId: null });

		for (const item of dnd.previewedItems()) {
			if (item.type === "folder" && item.id) {
				items.push({ id: item.id, type: "folder", folderId: null });
				if (!collapsedFolders().has(item.id)) {
					for (const room of item.items) {
						items.push({ id: room.id, type: "room", folderId: item.id });
					}
				}
			} else if (item.type === "view" && item.name) {
				items.push({ id: item.id, type: "view", folderId: null });
			} else if (item.type === "room" && item.room) {
				items.push({ id: item.room.id, type: "room", folderId: null });
			}
		}

		return items;
	});

	const keybinds = useRoomNavKeybinds({
		items: navItems,
		selectedId: () => params.room_id ?? "home",
		onToggleFolder: toggleFolder,
	});

	const isFocused = (id: string) => {
		const focused = keybinds.focusedId();
		if (focused !== null) return focused === id;
		const selected = params.room_id ?? "home";
		return selected === id;
	};

	const isSelected = createSelector(() => params.room_id ?? "home");

	const RoomItem = (props: { room: Room; folderId?: string }) => {
		const mentionCount = () => getRoomMentionCount(props.room.id);

		return (
			<li
				draggable="true"
				class="item menu-room room-item"
				data-id={props.room.id}
				data-room-id={props.room.id}
				data-folder-id={props.folderId}
				data-type="room"
				data-nav-id={props.room.id}
				tabIndex={isFocused(props.room.id) ? 0 : -1}
				onDragStart={dnd.handle}
				onDragOver={dnd.handle}
				onDragLeave={dnd.handle}
				onDrop={dnd.handle}
				onDragEnd={dnd.handle}
				classList={{
					dragging: dnd.dragging()?.id === props.room.id,
					unread: getRoomUnread(props.room.id),
					selected: isSelected(props.room.id),
				}}
			>
				<div class="tile">
					<A
						draggable="false"
						href={`/room/${props.room.id}`}
						class="nav"
						tabIndex={-1}
					>
						<RoomIcon room={props.room} mentionCount={mentionCount()} />
					</A>
				</div>
			</li>
		);
	};

	return (
		<Show when={flags.has("two_tier_nav")}>
			<nav id="room-nav" ref={keybinds.container} tabindex="-1">
				<ul class="room-list">
					<li
						class="item room-item"
						classList={{ selected: isSelected("home") }}
						data-nav-id="home"
						tabIndex={isFocused("home") ? 0 : -1}
					>
						<div class="tile with-background">
							<A href="/" end tabIndex={-1}>
								<Icon src={icHome} alt="home" />
							</A>
						</div>
					</li>
					<For each={dnd.previewedItems()}>
						{(item) => (
							<Switch>
								<Match when={item.type === "folder" && item} keyed>
									{(folder) => (
										<div
											class="folder"
											data-id={folder.id}
											data-type="folder"
											draggable="true"
											onDragStart={dnd.handle}
											onDragOver={dnd.handle}
											onDrop={dnd.handle}
											onDragLeave={dnd.handle}
											onDragEnd={dnd.handle}
											classList={{
												dragging: dnd.dragging()?.id === folder.id,
												target: dnd.target()?.folderId === folder.id,
												collapsed: collapsedFolders().has(folder.id),
												unread: !!getFolderUnread(folder),
											}}
										>
											<div
												class="item folder-item"
												classList={{
													unread: !!getFolderUnread(folder),
												}}
												data-nav-id={folder.id}
												tabIndex={isFocused(folder.id) ? 0 : -1}
											>
												<div
													class="tile with-background"
													onClick={() => toggleFolder(folder.id)}
													onContextMenu={(e) => {
														e.preventDefault();
														queueMicrotask(() => {
															setMenu({
																x: e.clientX,
																y: e.clientY,
																type: "folder",
																folder_id: folder.id,
															});
														});
													}}
												>
													<Icon src={icFolder1} alt="folder" />
												</div>
											</div>
											<Show when={!collapsedFolders().has(folder.id)}>
												<ul class="folder-items">
													<For each={folder.items}>
														{(room) => (
															<RoomItem room={room} folderId={folder.id} />
														)}
													</For>
												</ul>
											</Show>
										</div>
									)}
								</Match>
								<Match when={item.type === "view"}>
									{(view: { name: string }) => (
										<li
											class="item menu-room"
											data-id={`view-${view.name}`}
											data-type="view"
											data-nav-id={`view-${view.name}`}
											tabIndex={isFocused(`view-${view.name}`) ? 0 : -1}
											// isSelected
										>
											<A href="#" class="nav" tabIndex={-1}>
												<div class="avatar">{view.name?.substring?.(0, 2)}</div>
											</A>
										</li>
									)}
								</Match>
								<Match when={item.type === "room" && item} keyed>
									{(roomItem) => <RoomItem room={roomItem.room} />}
								</Match>
							</Switch>
						)}
					</For>
					<li
						class="drop-target-end"
						data-id="end"
						data-type="root"
						onDragOver={dnd.handle}
						onDrop={dnd.handle}
						style={{ height: "40px", "margin-top": "auto" }}
					/>
				</ul>
			</nav>
		</Show>
	);
};
