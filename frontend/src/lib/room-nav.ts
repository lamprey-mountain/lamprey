import type { RoomNavItem, RoomNavView } from "@/types/room-nav";

export const VIEW_UNCATEGORIZED_KEY = "__view_uncategorized__";

export const viewCategoryKey = (viewId: string, index: number) =>
	`view-category:${viewId}:${index}`;

export const viewCategoryIndex = (key: string | null) => {
	if (!key?.startsWith("view-category:")) return null;
	const index = Number(key.slice(key.lastIndexOf(":") + 1));
	return Number.isInteger(index) ? index : null;
};

export const updateRoomNavView = (
	config: RoomNavItem[] | undefined,
	viewId: string,
	update: (view: RoomNavView) => RoomNavView,
) => {
	if (!config) return null;

	let changed = false;
	const next = config.map((item) => {
		if (item.type !== "view" || item.id !== viewId) return item;
		changed = true;
		return update(item);
	});

	return changed ? next : null;
};

export const getViewChannels = (view: RoomNavView) => {
	const result = [...(view.uncategorized_channels ?? [])];

	for (const category of view.categories ?? []) {
		if ("channels" in category) result.push(...category.channels);
		else result.push(category);
	}

	return result;
};
