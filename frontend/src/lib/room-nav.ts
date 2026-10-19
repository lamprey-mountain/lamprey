import type { api } from "sdk";
import type { RoomNavItem, RoomNavView } from "@/types/room-nav";

type ViewChannel = api["ViewChannel"];
type ViewCategory = api["ViewCategory"];

export const VIEW_UNCATEGORIZED_KEY = "__view_uncategorized__";

/** the last channel the user viewed in a room, persisted in localStorage */
export const getLastViewedChannel = (roomId: string): string | null =>
	localStorage.getItem(`last_channel_${roomId}`);

export const setLastViewedChannel = (
	roomId: string,
	channelId: string,
): void => {
	localStorage.setItem(`last_channel_${roomId}`, channelId);
};

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

export const updateViewChannel = (
	view: RoomNavView,
	channelId: string,
	update: (channel: ViewChannel) => ViewChannel | null,
): RoomNavView => {
	const apply = (channel: ViewChannel) =>
		channel.id === channelId ? update(channel) : channel;

	const categories: ViewCategory[] = [];
	for (const category of view.categories ?? []) {
		if (!("channels" in category)) {
			const next = category.id === channelId ? update(category) : category;
			if (next) categories.push(next);
			continue;
		}

		categories.push({
			...category,
			channels: category.channels
				.map(apply)
				.filter((channel): channel is ViewChannel => channel !== null),
		});
	}

	return {
		...view,
		uncategorized_channels: (view.uncategorized_channels ?? [])
			.map(apply)
			.filter((channel): channel is ViewChannel => channel !== null),
		categories,
	};
};
