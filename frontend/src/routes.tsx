import { RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChatNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, Show } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { ChatHeader, ChatMain } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

const Nav2 = () => {
	const extraSidebar = false;
	return (
		<Show when={extraSidebar}>
			<div style="width:64px;display:flex;flex-direction:column;align-items:center;grid-area:nav2;overflow:auto;">
				{new Array(20).fill(0).map(() => (
					<div style="min-height:48px;width:48px;background:red;margin:8px 0;border-radius:4px">
					</div>
				))}
			</div>
		</Show>
	);
};

export const RouteRoom = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : t("loading")} />
			<Nav2 />
			<ChatNav />
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show when={flags.has("room_member_list")}>
					<RoomMembers room={room()!} />
				</Show>
			</Show>
		</>
	);
};

export const RouteRoomSettings = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	const title = () =>
		room() ? t("page.settings_room", room()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<Show when={room()}>
				<RoomSettings room={room()!} page={p.params.page} />
			</Show>
		</>
	);
};

export const RouteThreadSettings = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const title = () =>
		thread() ? t("page.settings_thread", thread()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<ChatNav />
			<Show when={thread()}>
				<ThreadSettings thread={thread()!} page={p.params.page} />
			</Show>
		</>
	);
};

export const RouteThread = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	return (
		<>
			<Show when={room() && thread()} fallback={<Title title={t("loading")} />}>
				<Title title={`${thread()!.name} - ${room()!.name}`} />
			</Show>
			<ChatNav />
			<Show when={room() && thread()}>
				<ChatHeader room={room()!} thread={thread()!} />
				<ChatMain room={room()!} thread={thread()!} />
				<Show when={flags.has("thread_member_list")}>
					<ThreadMembers thread={thread()!} />
				</Show>
			</Show>
		</>
	);
};
