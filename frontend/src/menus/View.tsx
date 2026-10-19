import { createMemo } from "solid-js";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { useModals } from "@/contexts/modal";
import { useReadTracking } from "@/contexts/read-tracking";
import { getViewChannels, updateRoomNavView } from "@/lib/room-nav";
import type { RoomNavItem, RoomNavView } from "@/types/room-nav";
import { Item, Menu, Separator } from "./Parts.tsx";

type ViewMenuProps = {
	view_id: string;
};

export function ViewMenu(props: ViewMenuProps) {
	const ctx = useCtx();
	const channels2 = useChannels();
	const [, modalctl] = useModals();
	const readTracking = useReadTracking();

	const getConfig = () =>
		(ctx.preferences().frontend.roomNav ?? []) as RoomNavItem[];

	const getView = createMemo<RoomNavView | null>(
		() =>
			getConfig().find(
				(item): item is RoomNavView =>
					item.type === "view" && item.id === props.view_id,
			) ?? null,
	);

	const updateView = (update: (view: RoomNavView) => RoomNavView) => {
		const prefs = ctx.preferences();
		const roomNav = updateRoomNavView(getConfig(), props.view_id, update);
		if (!roomNav) return;

		ctx.setPreferences({
			...prefs,
			frontend: {
				...prefs.frontend,
				roomNav,
			},
		});
	};

	const markAsRead = async () => {
		const view = getView();
		if (!view) return;

		for (const viewChannel of getViewChannels(view)) {
			const channel = channels2.cache.get(viewChannel.id);
			if (!channel?.last_message_id) continue;
			await readTracking.ack(
				channel.id,
				channel.last_message_id,
				true,
				false,
			);
		}
	};

	const renameView = () => {
		modalctl.prompt("new view name", (name) => {
			const viewName = name?.trim();
			if (!viewName) return;

			updateView((view) => ({ ...view, name: viewName }));
		});
	};

	const logToConsole = () => {
		console.log(JSON.parse(JSON.stringify(getView())));
	};

	return (
		<Menu>
			<Item onClick={markAsRead}>mark as read</Item>
			<Item onClick={renameView}>rename</Item>
			<Separator />
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
