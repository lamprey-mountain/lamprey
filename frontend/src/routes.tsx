import { A, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChatNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, For, Show } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { ChatHeader, ChatMain } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

export const Nav2 = () => {
	const api = useApi();
	const rooms = api.rooms.list();
	return (
		<Show when={flags.has("two_tier_nav")}>
			<nav class="nav2">
				<ul>
					<li>
						<A href="/" end>home</A>
					</li>
					<For each={rooms()?.items}>
						{(room) => (
							<li draggable="true">
								<A draggable="false" href={`/room/${room.id}`}>{room.name}</A>
							</li>
						)}
					</For>
				</ul>
			</nav>
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
			<ChatNav room_id={p.params.room_id} />
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
			<ChatNav room_id={thread()?.room_id} />
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
			<Nav2 />
			<ChatNav room_id={thread()?.room_id} />
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

export const RouteHome = () => {
	const { t } = useCtx();
	return (
		<>
			<Title title={t("page.home")} />
			<Nav2 />
			<ChatNav />
			<Home />
		</>
	);
};

export const RouteVoice = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	return (
		<>
			<Show when={room() && thread()} fallback={<Title title={t("loading")} />}>
				<Title title={`${thread()!.name} - ${room()!.name}`} />
			</Show>
			<Nav2 />
			<ChatNav room_id={thread()?.room_id} />
			<Show when={room() && thread()}>
				<ChatHeader room={room()!} thread={thread()!} />
				<Voice room={room()!} thread={thread()!} />
				<Show when={flags.has("thread_member_list")}>
					<ThreadMembers thread={thread()!} />
				</Show>
			</Show>
		</>
	);
};

export const RouteFeed = () => {
	return (
		<>
			<Title title="feed" />
			<Nav2 />
			<Feed />
		</>
	);
};
