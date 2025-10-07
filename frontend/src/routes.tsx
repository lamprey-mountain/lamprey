import { A, Navigate, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ThreadNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, For, Show } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { ChatHeader, ChatMain, SearchResults } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice, VoiceTray } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { getThumbFromId } from "./media/util.tsx";
import { RouteInviteInner } from "./Invite.tsx";
import { Forum } from "./Forum.tsx";
import { Category } from "./Category.tsx";
import { SERVER_ROOM_ID, type Thread } from "sdk";
export { RouteAuthorize } from "./Oauth.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => (document.title = props.title ?? ""));
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
						<A href="/" end>
							home
						</A>
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
	const ctx = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : t("loading")} />
			<RoomNav />
			<ThreadNav room_id={p.params.room_id} />
			<header>
				<b>home</b>
			</header>
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show
					when={flags.has("room_member_list") &&
						ctx.userConfig().frontend.showMembers !== false}
				>
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

const ThreadSidebar = (props: { thread: Thread }) => {
	const ctx = useCtx();
	const search = () => ctx.thread_search.get(props.thread.id);
	const showMembers = () =>
		props.thread.type !== "Voice" &&
		flags.has("thread_member_list") &&
		ctx.userConfig().frontend.showMembers !== false;

	return (
		<Show
			when={search()}
			fallback={
				<Show when={showMembers()}>
					<ThreadMembers thread={props.thread} />
				</Show>
			}
		>
			<SearchResults thread={props.thread} search={search()!} />
		</Show>
	);
};

export const RouteThread = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const ctx = useCtx();
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
					when={thread()!.type === "Chat" ||
						thread()!.type === "Dm" ||
						thread()!.type === "Gdm"}
				>
					<ChatMain thread={thread()!} />
				</Show>
				<Show when={thread()!.type === "Voice"}>
					<Voice thread={thread()!} />
				</Show>
				<Show when={thread()!.type === "Forum"}>
					<Forum thread={thread()!} />
				</Show>
				<Show when={thread()!.type === "Category"}>
					<Category thread={thread()!} />
				</Show>
				<ThreadSidebar thread={thread()!} />
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
			<VoiceTray />
		</>
	);
};

export const RouteFeed = () => {
	return (
		<>
			<Title title="feed" />
			<RoomNav />
			<Feed />
			<VoiceTray />
		</>
	);
};

export const RouteInvite = (p: RouteSectionProps) => {
	return (
		<>
			<Show when={p.params.code}>
				<RoomNav />
				<ThreadNav room_id={p.params.room_id} />
				<RouteInviteInner code={p.params.code!} />
				<VoiceTray />
			</Show>
		</>
	);
};
