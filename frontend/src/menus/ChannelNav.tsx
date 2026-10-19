import type { api } from "sdk";
import { Show } from "solid-js";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useModals } from "@/contexts/modal.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { getViewChannels, updateRoomNavView } from "@/lib/room-nav";
import type { RoomNavItem, RoomNavView } from "@/types/room-nav";
import { Item, Menu } from "./Parts.tsx";

export type ChannelNavMenuProps = {
	room_id?: string;
	view_id?: string;
};

// when right clicking in channel nav but not on a channel (empty space)
export function ChannelNavMenu(props: ChannelNavMenuProps) {
	const ctx = useCtx();
	const channels2 = useChannels();
	const [, modalctl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room_id,
		() => undefined,
	);

	const updateView = (update: (view: RoomNavView) => RoomNavView) => {
		const viewId = props.view_id;
		if (!viewId) return;

		const prefs = ctx.preferences();
		const config = (prefs.frontend.roomNav ?? []) as RoomNavItem[];
		const roomNav = updateRoomNavView(config, viewId, update);
		if (!roomNav) return;

		ctx.setPreferences({
			...prefs,
			frontend: {
				...prefs.frontend,
				roomNav,
			},
		});
	};

	const addChannel = () => {
		modalctl.prompt("channel id", (id) => {
			const channelId = id?.trim();
			if (!channelId) return;

			const channel = channels2.cache.get(channelId);
			const channelRef: api["ViewChannel"] = { id: channelId };
			const roomId = channel?.room_id ?? props.room_id;
			if (roomId) channelRef.room_id = roomId;

			updateView((view) => {
				if (getViewChannels(view).some((channel) => channel.id === channelId)) {
					return view;
				}

				return {
					...view,
					uncategorized_channels: [
						...(view.uncategorized_channels ?? []),
						channelRef,
					],
				};
			});
		});
	};

	const createCategory = () => {
		modalctl.prompt("new category name", (name) => {
			const categoryName = name?.trim();
			if (!categoryName) return;

			updateView((view) => ({
				...view,
				categories: [
					...(view.categories ?? []),
					{ name: categoryName, channels: [] },
				],
			}));
		});
	};

	return (
		<Menu>
			<Show
				when={props.view_id}
				fallback={
					<>
						<Show when={hasPermission("ChannelManage")}>
							<Item
								onClick={() => {
									if (!props.room_id) return;
									modalctl.open({
										type: "channel_create",
										room_id: props.room_id,
									});
								}}
							>
								create channel
							</Item>
						</Show>
						<Show when={hasPermission("InviteCreate")}>
							<Item
								onClick={() => {
									if (!props.room_id) return;
									modalctl.open({
										type: "invite_create",
										room_id: props.room_id,
									});
								}}
							>
								create invite
							</Item>
						</Show>
					</>
				}
			>
				<Item onClick={addChannel}>add channel</Item>
				<Item onClick={createCategory}>create category</Item>
			</Show>
		</Menu>
	);
}
