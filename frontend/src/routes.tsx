import { A, Navigate, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ThreadNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, For, Match, Show, Switch } from "solid-js";
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
import { PinnedMessages } from "./menu/PinnedMessages.tsx";
import { Resizable } from "./Resizable.tsx";
import { UserProfile } from "./UserProfile.tsx";
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
	const showPinned = () => ctx.thread_pinned_view.get(props.thread.id) ?? false;
	const showVoiceChat = () =>
		props.thread.type === "Voice" &&
		ctx.voice_chat_sidebar_open.get(props.thread.id);

	return (
		<Switch>
			<Match when={showVoiceChat()}>
				<Resizable storageKey="voice-chat-sidebar-width" initialWidth={320}>
					<div class="voice-chat-sidebar">
						<ChatMain thread={props.thread} />
					</div>
				</Resizable>
			</Match>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults thread={props.thread} search={search()!} />
				</Resizable>
			</Match>
			<Match when={showPinned()}>
				<Resizable storageKey="pinned-sidebar-width" initialWidth={320}>
					<PinnedMessages thread={props.thread} />
				</Resizable>
			</Match>
			<Match when={showMembers()}>
				<ThreadMembers thread={props.thread} />
			</Match>
		</Switch>
	);
};

export const RouteThread = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	createEffect(() => {
		const { thread_id, message_id } = p.params;
		if (thread_id && message_id) {
			ctx.thread_anchor.set(thread_id, {
				type: "context",
				limit: 50,
				message_id: message_id,
			});
			ctx.thread_highlight.set(thread_id, message_id);
		} else if (thread_id) {
			const current_anchor = ctx.thread_anchor.get(thread_id);
			if (current_anchor?.type === "context") {
				ctx.thread_anchor.delete(thread_id);
			}
		}
	});

	// fetch threads to populate sidebar
	api.threads.list(() => thread()?.room_id!);

	const title = () => {
		const th = thread();
		if (!th) return t("loading");
		if (th.type === "Dm") {
			const user_id = api.users.cache.get("@self")!.id;
			return th.recipients.find((i) => i.id !== user_id)?.name ??
				"dm";
		}

		return room() && th.room_id ? `${th.name} - ${room()!.name}` : th.name;
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

export const RouteUser = (p: RouteSectionProps) => {
	const api = useApi();
	const user = api.users.fetch(() => p.params.user_id);

	return (
		<>
			<Title title={user()?.name ?? "loading..."} />
			<RoomNav />
			<ThreadNav />
			<header class="chat-header">
				<b>{user()?.name}</b>
			</header>
			<Show when={user()}>
				<UserProfile user={user()!} />
			</Show>
			<VoiceTray />
		</>
	);
};
