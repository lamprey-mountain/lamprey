import { debounce } from "@solid-primitives/scheduled";
import type { Room } from "sdk";
import { createMemo, createSignal } from "solid-js";
import type { RoomNavItem, RoomNavMappedItem } from "@/types/room-nav";

type DndItem = { type: "room" | "folder"; id: string };

type DndTarget = {
	folderId?: string;
	otherRoomId: string;
	position: "before" | "after" | "create-folder";
};

export interface RoomDndProps {
	getConfig: () => RoomNavItem[];
	setConfig: (newConfig: RoomNavItem[]) => void;
	getDefaultConfig: () => RoomNavItem[];
	rooms: () => Room[];
}

// TODO: verify llm generated code
// (i'm too lazy to manually implement dnd and this seems to work ¯\_(ツ)_/¯)

export const useRoomDnd = (props: RoomDndProps) => {
	const [dragging, setDragging] = createSignal<DndItem | null>(null);
	const [target, setTarget] = createSignal<DndTarget | null>(null);

	const config = createMemo(() => {
		let config = props.getConfig();
		if (config.length === 0) {
			config = props.getDefaultConfig();
		}
		return config;
	});

	const findItem = (id: string, searchConfig: RoomNavItem[]) => {
		for (let i = 0; i < searchConfig.length; i++) {
			const item = searchConfig[i];
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

	const simulateDrop = (
		c: RoomNavItem[],
		fromId: string,
		t: DndTarget | null,
	) => {
		if (!t || !fromId) return c;

		const from = findItem(fromId, c);
		if (!from) return c;

		if (t.position === "create-folder") {
			const otherRoom = findItem(t.otherRoomId, c);
			if (!otherRoom || fromId === t.otherRoomId) return c;
			if (otherRoom.parent) return c; // dont allow folder creation inside folder

			const [movedItem] = from.parent
				? from.parent.items.splice(from.index, 1)
				: c.splice(from.index, 1);

			const otherRoomAfter = findItem(t.otherRoomId, c);
			if (otherRoomAfter) {
				if (otherRoomAfter.item.type === "room" && movedItem.type === "room") {
					const folderItem: RoomNavItem = {
						type: "folder",
						id: crypto.randomUUID(),
						name: "New Folder",
						items: [
							{ type: "room", room_id: otherRoomAfter.item.room_id },
							{ type: "room", room_id: movedItem.room_id },
						],
					};
					c.splice(otherRoomAfter.index, 1, folderItem);
				} else if (
					otherRoomAfter.item.type === "folder" &&
					movedItem.type === "room"
				) {
					otherRoomAfter.item.items.push(movedItem);
				} else {
					c.splice(from.index, 0, movedItem); // rollback
				}
			} else {
				c.push(movedItem); // fallback
			}
		} else {
			const { otherRoomId, folderId, position } = t;

			if (otherRoomId === "end") {
				const [movedItem] = from.parent
					? from.parent.items.splice(from.index, 1)
					: c.splice(from.index, 1);
				c.push(movedItem);
			} else {
				const otherRoomInit = findItem(otherRoomId, c);
				if (!otherRoomInit) return c;

				if (
					from.item.type === "folder" &&
					(folderId ||
						otherRoomInit.parent ||
						otherRoomInit.item.type === "folder")
				) {
					// Cannot drag folder into folder
					return c;
				}

				const [movedItem] = from.parent
					? from.parent.items.splice(from.index, 1)
					: c.splice(from.index, 1);

				const otherRoomAfter = findItem(otherRoomId, c);
				if (otherRoomAfter) {
					if (folderId && movedItem.type === "room") {
						const folderObj = findItem(folderId, c);
						if (folderObj && folderObj.item.type === "folder") {
							const targetIdx = folderObj.item.items.findIndex(
								(r) => r.room_id === otherRoomId,
							);
							if (targetIdx !== -1) {
								const insertIndex = targetIdx + (position === "after" ? 1 : 0);
								folderObj.item.items.splice(insertIndex, 0, movedItem as any);
							} else {
								folderObj.item.items.push(movedItem as any);
							}
						}
					} else {
						const insertIndex =
							otherRoomAfter.index + (position === "after" ? 1 : 0);
						if (otherRoomAfter.parent) {
							otherRoomAfter.parent.items.splice(
								insertIndex,
								0,
								movedItem as any,
							);
						} else {
							c.splice(insertIndex, 0, movedItem);
						}
					}
				} else {
					c.splice(from.index, 0, movedItem); // rollback
				}
			}
		}

		return c.filter(
			(item) => !(item.type === "folder" && item.items.length === 0),
		);
	};

	const previewedConfig = createMemo(() => {
		const c = JSON.parse(JSON.stringify(config())) as RoomNavItem[];
		const fromId = dragging()?.id;
		const t = target();

		if (!fromId || !t) return c;
		return simulateDrop(c, fromId, t);
	});

	const itemCache = new Map<string, RoomNavMappedItem>();

	const previewedItems = createMemo(() => {
		const configValue = previewedConfig();
		const roomsList = props.rooms();
		const roomMap = new Map(roomsList.map((r) => [r.id, r]));

		const mappedConfig: RoomNavMappedItem[] = [];
		const orderedIds = new Set<string>();

		for (const item of configValue) {
			if (item.type === "room") {
				const room = roomMap.get(item.room_id);
				if (room) {
					let mapped = itemCache.get(room.id);
					if (!mapped || mapped.type !== "room") {
						mapped = { type: "room", room };
						itemCache.set(room.id, mapped);
					}
					mappedConfig.push(mapped);
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
					// for folder, we can just create a new object, because folders might not be dragged directly, but if they are, they need stability too.
					let folderMapped = itemCache.get(item.id);
					if (
						!folderMapped ||
						folderMapped.type !== "folder" ||
						folderMapped.name !== item.name
					) {
						folderMapped = {
							type: "folder",
							id: item.id,
							name: item.name,
							items: folderItems,
						};
						itemCache.set(item.id, folderMapped);
					} else {
						// update items reference to trigger inner For if needed, but keep folder object stable?
						// actually, if we keep folder object stable but change its items, Solid's For might not update the inner list unless the folder itself is a signal, which it isn't.
						// so we MUST return a new folder object if the items change.
						// Wait, if we return a new folder object, the folder DOM node is recreated.
						// Let's just create a new folder object if items are different.
						const oldItems = folderMapped.items;
						let changed = oldItems.length !== folderItems.length;
						if (!changed) {
							for (let k = 0; k < oldItems.length; k++) {
								if (oldItems[k].id !== folderItems[k].id) {
									changed = true;
									break;
								}
							}
						}
						if (changed) {
							folderMapped = {
								type: "folder",
								id: item.id,
								name: item.name,
								items: folderItems,
							};
							itemCache.set(item.id, folderMapped);
						}
					}
					mappedConfig.push(folderMapped);
				}
			} else if (item.type === "view") {
				let viewMapped = itemCache.get(`view-${item.name}`);
				if (!viewMapped) {
					viewMapped = item;
					itemCache.set(`view-${item.name}`, viewMapped);
				}
				mappedConfig.push(viewMapped);
			}
		}

		const unordered = roomsList.filter((r) => !orderedIds.has(r.id));
		return [
			...unordered.map((room) => {
				let mapped = itemCache.get(room.id);
				if (!mapped || mapped.type !== "room") {
					mapped = { type: "room", room };
					itemCache.set(room.id, mapped);
				}
				return mapped;
			}),
			...mappedConfig,
		];
	});

	const intoCreateFolder = debounce(() => {
		setTarget((t) => {
			if (!t) return null;
			return {
				position: "create-folder",
				otherRoomId: t.otherRoomId,
			};
		});
	}, 500);

	const handleDragStart = (e: DragEvent) => {
		const el = e.currentTarget as HTMLElement;
		const id = el.dataset.id;
		const type = el.dataset.type as "room" | "folder";
		if (id && type) {
			setDragging({ id, type });
		}
		e.stopPropagation();
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const targetEl = e.currentTarget as HTMLElement;
		const id = targetEl.dataset.id;
		const toType = targetEl.dataset.type;
		const folderId =
			targetEl.closest("[data-folder-id]")?.getAttribute("data-folder-id") ||
			undefined;

		if (!id || !dragging() || id === dragging()?.id) {
			intoCreateFolder.clear();
			return;
		}

		const rect = targetEl.getBoundingClientRect();
		const relY = e.clientY - rect.top;
		const isAbove = relY < rect.height / 2;

		setTarget({
			folderId,
			otherRoomId: id,
			position: isAbove ? "before" : "after",
		});
		intoCreateFolder();
	};

	const handleDragLeave = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const relatedTarget = e.relatedTarget as HTMLElement;
		if (
			!e.currentTarget ||
			!(e.currentTarget as HTMLElement).contains(relatedTarget)
		) {
			intoCreateFolder.clear();
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const fromId = dragging()?.id;
		const t = target();

		intoCreateFolder.clear();
		setDragging(null);
		setTarget(null);

		if (!t || !fromId) return;

		const c = JSON.parse(JSON.stringify(config())) as RoomNavItem[];
		const finalConfig = simulateDrop(c, fromId, t);
		props.setConfig(finalConfig);
	};

	const handleDragEnd = () => {
		intoCreateFolder.clear();
		setDragging(null);
		setTarget(null);
	};

	const handle = (e: DragEvent) => {
		switch (e.type) {
			case "dragstart":
				handleDragStart(e);
				break;
			case "dragover":
				handleDragOver(e);
				break;
			case "dragleave":
				handleDragLeave(e);
				break;
			case "drop":
				handleDrop(e);
				break;
			case "dragend":
				handleDragEnd();
				break;
		}
	};

	return {
		dragging,
		target,
		handle,
		previewedItems,
	};
};
