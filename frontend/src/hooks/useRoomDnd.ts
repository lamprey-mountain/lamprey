import { debounce } from "@solid-primitives/scheduled";
import type { Room } from "sdk";
import { createMemo, createSignal } from "solid-js";
import type { RoomNavItem, RoomNavMappedItem } from "@/types/room-nav";

// TODO: verify llm generated code
// (i'm too lazy to manually implement dnd and this seems to work ¯\_(ツ)_/¯)

type AnyRoomNavItem = RoomNavItem;

type DndItem = { type: RoomNavItem["type"]; id: string };

type DndTarget = {
	id: string;
	position: "before" | "after" | "inside" | "create-folder";
};

type ItemLocation = {
	item: AnyRoomNavItem;
	container: AnyRoomNavItem[];
	index: number;
	parent: (RoomNavItem & { type: "folder" }) | null;
};

export interface RoomDndProps {
	getConfig: () => RoomNavItem[];
	setConfig: (newConfig: RoomNavItem[]) => void;
	getDefaultConfig: () => RoomNavItem[];
	rooms: () => Room[];
}

const itemId = (item: AnyRoomNavItem) =>
	item.type === "room" ? item.room_id : item.id;

const findItem = (items: RoomNavItem[], id: string): ItemLocation | null => {
	for (let i = 0; i < items.length; i++) {
		const item = items[i];
		if (itemId(item) === id) {
			return { item, container: items, index: i, parent: null };
		}

		if (item.type === "folder") {
			for (let j = 0; j < item.items.length; j++) {
				const child = item.items[j];
				if (itemId(child) === id) {
					return { item: child, container: item.items, index: j, parent: item };
				}
			}
		}
	}

	return null;
};

const removeEmptyFolders = (config: RoomNavItem[]) =>
	config.filter((item) => item.type !== "folder" || item.items.length > 0);

const simulateDrop = (
	source: RoomNavItem[],
	fromId: string,
	target: DndTarget | null,
) => {
	if (!target) return source;

	const config = structuredClone(source);
	const from = findItem(config, fromId);
	if (!from || fromId === target.id) return source;

	if (target.id !== "end" && !findItem(config, target.id)) return source;

	from.container.splice(from.index, 1);

	if (target.id === "end") {
		config.push(from.item);
		return removeEmptyFolders(config);
	}

	const to = findItem(config, target.id);
	// The target was inside the item we just removed.
	if (!to) return source;

	if (target.position === "create-folder") {
		if (
			from.item.type !== "room" ||
			to.item.type !== "room" ||
			to.parent !== null
		) {
			return source;
		}

		const folder: RoomNavItem = {
			type: "folder",
			id: crypto.randomUUID(),
			name: "New Folder",
			items: [to.item, from.item],
		};

		to.container.splice(to.index, 1, folder);
		return removeEmptyFolders(config);
	}

	if (target.position === "inside") {
		if (from.item.type !== "room" || to.item.type !== "folder") {
			return source;
		}

		to.item.items.push(from.item);
		return removeEmptyFolders(config);
	}

	const insertAfter = target.position === "after" ? 1 : 0;

	if (to.parent) {
		if (from.item.type === "room") {
			to.container.splice(to.index + insertAfter, 0, from.item);
		} else {
			// Folders and views cannot go inside folders; anchor them to the
			// parent folder instead.
			const parent = findItem(config, to.parent.id);
			if (!parent) return source;
			parent.container.splice(parent.index + insertAfter, 0, from.item);
		}
	} else {
		to.container.splice(to.index + insertAfter, 0, from.item);
	}

	return removeEmptyFolders(config);
};

export const useRoomDnd = (props: RoomDndProps) => {
	const [dragging, setDragging] = createSignal<DndItem | null>(null);
	const [target, setTarget] = createSignal<DndTarget | null>(null);

	const configured = createMemo(() => {
		const config = props.getConfig();
		return config.length > 0 ? config : props.getDefaultConfig();
	});

	// The rendered list contains rooms that aren't in the saved config yet
	// (e.g. newly joined rooms). Include them so drops use the same list as the UI.
	const effectiveConfig = createMemo(() => {
		const config = configured();
		const placed = new Set<string>();

		for (const item of config) {
			if (item.type === "room") {
				placed.add(item.room_id);
			} else if (item.type === "folder") {
				for (const child of item.items) placed.add(child.room_id);
			}
		}

		const unplaced: RoomNavItem[] = props
			.rooms()
			.filter((room) => !placed.has(room.id))
			.map((room) => ({ type: "room", room_id: room.id }));

		return unplaced.length > 0 ? [...unplaced, ...config] : config;
	});

	const previewedConfig = createMemo(() => {
		const fromId = dragging()?.id;
		if (!fromId) return effectiveConfig();
		return simulateDrop(effectiveConfig(), fromId, target());
	});

	const itemCache = new Map<string, RoomNavMappedItem>();

	const previewedItems = createMemo(() => {
		const roomMap = new Map(props.rooms().map((room) => [room.id, room]));
		const result: RoomNavMappedItem[] = [];

		const mapRoom = (roomId: string): RoomNavMappedItem | null => {
			const room = roomMap.get(roomId);
			if (!room) return null;

			const key = `room:${room.id}`;
			const cached = itemCache.get(key);
			if (cached?.type === "room" && cached.room === room) return cached;

			const mapped: RoomNavMappedItem = { type: "room", room };
			itemCache.set(key, mapped);
			return mapped;
		};

		const mapView = (item: Extract<RoomNavItem, { type: "view" }>) => {
			const key = `view:${item.id}`;
			const cached = itemCache.get(key);
			if (
				cached?.type === "view" &&
				cached.name === item.name &&
				cached.categories === item.categories &&
				cached.uncategorized_channels === item.uncategorized_channels
			) {
				return cached;
			}

			const mapped: RoomNavMappedItem = { ...item };
			itemCache.set(key, mapped);
			return mapped;
		};

		for (const item of previewedConfig()) {
			if (item.type === "room" || item.type === "view") {
				const mapped =
					item.type === "room" ? mapRoom(item.room_id) : mapView(item);
				if (mapped) result.push(mapped);
				continue;
			}

			const items = item.items
				.map((child) => roomMap.get(child.room_id))
				.filter((room): room is Room => !!room);

			if (items.length === 0) continue;

			const key = `folder:${item.id}`;
			const cached = itemCache.get(key);
			const sameItems =
				cached?.type === "folder" &&
				cached.items.length === items.length &&
				cached.items.every((room, index) => room === items[index]);

			if (cached?.type === "folder" && cached.name === item.name && sameItems) {
				result.push(cached);
				continue;
			}

			const mapped: RoomNavMappedItem = {
				type: "folder",
				id: item.id,
				name: item.name,
				items,
			};
			itemCache.set(key, mapped);
			result.push(mapped);
		}

		return result;
	});

	const intoCreateFolder = debounce(() => {
		setTarget((current) => {
			if (!current || current.id === "end") return current;
			return { id: current.id, position: "create-folder" };
		});
	}, 500);

	const handleDragStart = (e: DragEvent) => {
		const el = e.currentTarget as HTMLElement;
		const id = el.dataset.id;
		const type = el.dataset.type;

		if (id && (type === "room" || type === "folder" || type === "view")) {
			setDragging({ id, type });
		}

		e.stopPropagation();
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();

		const el = e.currentTarget as HTMLElement;
		const id = el.dataset.id;
		const type = el.dataset.type;
		const dragged = dragging();
		const folderId = el
			.closest("[data-folder-id]")
			?.getAttribute("data-folder-id");

		if (!id || !dragged || id === dragged.id) {
			intoCreateFolder.clear();
			return;
		}

		if (id === "end") {
			intoCreateFolder.clear();
			setTarget({ id, position: "before" });
			return;
		}

		const rect = el.getBoundingClientRect();
		const relY = e.clientY - rect.top;

		if (dragged.type === "room" && type === "folder") {
			const inside = relY > rect.height * 0.25 && relY < rect.height * 0.75;
			if (inside) {
				intoCreateFolder.clear();
				setTarget({ id, position: "inside" });
				return;
			}
		}

		setTarget({
			id,
			position: relY < rect.height / 2 ? "before" : "after",
		});

		if (dragged.type === "room" && type === "room" && !folderId) {
			intoCreateFolder();
		} else {
			intoCreateFolder.clear();
		}
	};

	const handleDragLeave = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();

		const el = e.currentTarget as HTMLElement;
		if (!el.contains(e.relatedTarget as Node | null)) {
			intoCreateFolder.clear();
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();

		const fromId = dragging()?.id;
		const dropTarget = target();

		intoCreateFolder.clear();
		setDragging(null);
		setTarget(null);

		if (!fromId || !dropTarget) return;

		props.setConfig(simulateDrop(effectiveConfig(), fromId, dropTarget));
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
