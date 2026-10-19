import { createMemo, Show } from "solid-js";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { useModals } from "@/contexts/modal";
import { useReadTracking } from "@/contexts/read-tracking";
import { useNavigate } from "@/contexts/router";
import {
	getViewChannels,
	updateRoomNavView,
	updateViewChannel,
} from "@/lib/room-nav";
import type { RoomNavItem, RoomNavView } from "@/types/room-nav";
import { ChannelNotificationMenu } from "./Channel.tsx";
import { Item, Menu, Separator } from "./Parts.tsx";

type ViewChannelMenuProps = {
	view_id: string;
	channel_id: string;
};

// the context menu for channels referenced by a view
export function ViewChannelMenu(props: ViewChannelMenuProps) {
	const ctx = useCtx();
	const channels2 = useChannels();
	const readTracking = useReadTracking();
	const nav = useNavigate();
	const [, modalCtl] = useModals();

	const getConfig = () =>
		(ctx.preferences().frontend.roomNav ?? []) as RoomNavItem[];

	const getView = createMemo<RoomNavView | null>(
		() =>
			getConfig().find(
				(item): item is RoomNavView =>
					item.type === "view" && item.id === props.view_id,
			) ?? null,
	);

	const viewChannel = createMemo(() => {
		const view = getView();
		if (!view) return null;
		return (
			getViewChannels(view).find(
				(channel) => channel.id === props.channel_id,
			) ?? null
		);
	});

	const channel = channels2.use(() => props.channel_id);

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

	const copyId = () => navigator.clipboard.writeText(props.channel_id);

	const markRead = () => {
		const c = channels2.cache.get(props.channel_id);
		if (!c?.last_message_id) return;
		readTracking.ack(props.channel_id, c.last_message_id, true, false);
	};

	const goToRoom = () => {
		const roomId = viewChannel()?.room_id ?? channel()?.room_id;
		if (!roomId) return;
		nav(`/room/${roomId}`);
	};

	const changeNickname = () => {
		modalCtl.open({
			type: "view_channel_nickname",
			initial: viewChannel()?.nickname ?? "",
			placeholder: channel()?.name ?? "",
			cont: (nickname) => {
				updateView((view) =>
					updateViewChannel(view, props.channel_id, (viewChannel) => ({
						...viewChannel,
						nickname: nickname.trim() || null,
					})),
				);
			},
		});
	};

	const removeFromFavorites = () => {
		updateView((view) => updateViewChannel(view, props.channel_id, () => null));
	};

	const logToConsole = () => console.log(channel());

	return (
		<Menu>
			<Item onClick={markRead}>mark as read</Item>
			<Show when={channel()}>
				{(c) => <ChannelNotificationMenu channel={c()} />}
			</Show>
			<Separator />
			<Show when={viewChannel()?.room_id ?? channel()?.room_id}>
				<Item onClick={goToRoom}>go to room</Item>
			</Show>
			<Item onClick={changeNickname}>change nickname</Item>
			<Item onClick={removeFromFavorites} color="danger">
				remove from favorites
			</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
