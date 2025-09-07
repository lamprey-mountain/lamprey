import { A, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ThreadNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, For, Show } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { ChatHeader, ChatMain } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice, VoiceTray } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { getThumbFromId } from "./media/util.tsx";
import { RouteInviteInner } from "./Invite.tsx";
import { AdminSettings } from "./AdminSettings.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

export const RoomNav = () => {
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
							<li draggable="true" class="menu-room" data-room-id={room.id}>
								<A draggable="false" href={`/room/${room.id}`} class="nav">
									<Show
										when={room.icon}
										fallback={<div class="avatar">{room.name}</div>}
									>
										<img
											src={getThumbFromId(room.icon!, 64)}
											class="avatar"
										/>
									</Show>
								</A>
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
			<RoomNav />
			<ThreadNav room_id={p.params.room_id} />
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show when={flags.has("room_member_list")}>
					<RoomMembers room={room()!} />
				</Show>
			</Show>
			<VoiceTray />
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

export const RouteAdminSettings = (p: RouteSectionProps) => {
	return (
		<>
			<Title title={"admin settings"} />
			<AdminSettings page={p.params.page} />
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

	// fetch threads to populate sidebar
	api.threads.list(() => thread()?.room_id!);

	const title = () => {
		if (!thread()) return t("loading");
		return room() && thread()?.room_id
			? `${thread()!.name} - ${room()!.name}`
			: thread()!.name;
	};

	return (
		<>
			<Title title={title()} />
			<RoomNav />
			<ThreadNav room_id={thread()?.room_id ?? undefined} />
			<Show when={thread()}>
				<Show when={thread()!.type !== "Voice"}>
					<ChatHeader thread={thread()!} />
				</Show>
				<Show
					when={thread()!.type === "Chat" || thread()!.type === "Dm" ||
						thread()!.type === "Gdm"}
				>
					<ChatMain thread={thread()!} />
				</Show>
				<Show when={thread()!.type === "Voice"}>
					<Voice thread={thread()!} />
				</Show>
				<Show
					when={thread()!.type !== "Voice" && flags.has("thread_member_list")}
				>
					<ThreadMembers thread={thread()!} />
				</Show>
				<VoiceTray />
			</Show>
		</>
	);
};

export const RouteHome = () => {
	const { t } = useCtx();
	return (
		<>
			<Title title={t("page.home")} />
			<RoomNav />
			<ThreadNav />
			<Home />
		</>
	);
};

export const RouteFeed = () => {
	return (
		<>
			<Title title="feed" />
			<RoomNav />
			<Feed />
		</>
	);
};

export const RouteInvite = (p: RouteSectionProps) => {
	return (
		<>
			<Show when={p.params.code}>
				<RouteInviteInner code={p.params.code!} />
			</Show>
		</>
	);
};
