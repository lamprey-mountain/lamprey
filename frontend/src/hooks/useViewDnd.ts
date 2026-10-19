import type { api } from "sdk";
import { createSignal } from "solid-js";
import type { DndPlacement } from "@/hooks/useChannelDnd";
import {
	VIEW_UNCATEGORIZED_KEY,
	viewCategoryIndex,
} from "@/lib/room-nav";
import type { RoomNavView } from "@/types/room-nav";

type ViewChannel = api["ViewChannel"];

type ViewDndTarget =
	| {
			kind: "channel";
			categoryId: string | null;
			channelId: string;
			place: "before" | "after";
	  }
	| { kind: "category"; categoryId: string }
	| { kind: "uncategorized" };

type ViewDndState = {
	channelId: string;
	target?: ViewDndTarget;
};

type ChannelArray = ViewChannel[] | undefined;

const channelsForCategory = (
	view: RoomNavView,
	categoryId: string | null,
): ChannelArray => {
	if (categoryId === null) return view.uncategorized_channels;

	const index = viewCategoryIndex(categoryId);
	if (index === null) return undefined;

	const category = view.categories?.[index];
	return category && "channels" in category ? category.channels : undefined;
};

const removeChannel = (view: RoomNavView, channelId: string) => {
	const fromUncategorized = view.uncategorized_channels?.findIndex(
		(channel) => channel.id === channelId,
	);
	if (fromUncategorized !== undefined && fromUncategorized !== -1) {
		return view.uncategorized_channels?.splice(fromUncategorized, 1)[0];
	}

	for (const category of view.categories ?? []) {
		if (!("channels" in category)) continue;
		const index = category.channels.findIndex(
			(channel) => channel.id === channelId,
		);
		if (index !== -1) return category.channels.splice(index, 1)[0];
	}

	return undefined;
};

export interface ViewDndProps {
	view: () => RoomNavView | null;
	updateView: (update: (view: RoomNavView) => RoomNavView) => void;
}

export const useViewDnd = (props: ViewDndProps) => {
	const [state, setState] = createSignal<ViewDndState | null>(null);

	const handleStart = (e: DragEvent) => {
		if (!props.view()) return;

		const el = e.currentTarget as HTMLElement;
		const channelId = el.dataset.channelId;
		if (!channelId) return;

		e.stopPropagation();
		e.dataTransfer?.setData("text/plain", channelId);
		if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
		setState({ channelId });
	};

	const overChannel = (
		e: DragEvent,
		categoryId: string | null,
		channelId: string,
	) => {
		const current = state();
		if (!current || current.channelId === channelId) return;

		e.preventDefault();
		e.stopPropagation();

		const el = e.currentTarget as HTMLElement;
		const rect = el.getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;

		setState({
			...current,
			target: {
				kind: "channel",
				categoryId,
				channelId,
				place: after ? "after" : "before",
			},
		});
	};

	const overCategory = (e: DragEvent, categoryId: string) => {
		const current = state();
		if (!current) return;

		e.preventDefault();
		e.stopPropagation();
		setState({ ...current, target: { kind: "category", categoryId } });
	};

	const overUncategorized = (e: DragEvent) => {
		const current = state();
		if (!current) return;

		e.preventDefault();
		e.stopPropagation();
		setState({ ...current, target: { kind: "uncategorized" } });
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();

		const current = state();
		const view = props.view();
		setState(null);

		if (!current?.target || !view) return;

		const next = JSON.parse(JSON.stringify(view)) as RoomNavView;
		const moved = removeChannel(next, current.channelId);
		if (!moved) return;

		const target = current.target;

		if (target.kind === "category") {
			const channels = channelsForCategory(next, target.categoryId);
			if (!channels) return;
			channels.push(moved);
		} else if (target.kind === "uncategorized") {
			const channels = next.uncategorized_channels ?? [];
			channels.push(moved);
			next.uncategorized_channels = channels;
		} else {
			const channels = channelsForCategory(next, target.categoryId);
			if (!channels) return;

			let index = channels.findIndex(
				(channel) => channel.id === target.channelId,
			);
			if (index === -1) index = channels.length;
			else if (target.place === "after") index += 1;

			channels.splice(index, 0, moved);
		}

		props.updateView(() => next);
	};

	const handleEnd = () => {
		setState(null);
	};

	const placement = (id: string): DndPlacement | undefined => {
		const target = state()?.target;
		if (!target) return undefined;

		if (target.kind === "channel" && target.channelId === id) {
			return target.place;
		}
		if (target.kind === "category" && target.categoryId === id) {
			return "inside-category";
		}
		if (target.kind === "uncategorized" && id === VIEW_UNCATEGORIZED_KEY) {
			return "inside-category";
		}

		return undefined;
	};

	return {
		handleStart,
		overChannel,
		overCategory,
		overUncategorized,
		handleDrop,
		handleEnd,
		placement,
	};
};
