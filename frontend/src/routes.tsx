import { A, Navigate, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChannelNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import {
	createEffect,
	createResource,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ChannelSettings } from "./ChannelSettings.tsx";
import { ChatHeader, ChatMain, SearchResults } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice, VoiceTray } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { getThumbFromId } from "./media/util.tsx";
import { RouteInviteInner } from "./Invite.tsx";
import { Forum } from "./Forum.tsx";
import { Category } from "./Category.tsx";
import { type Channel, SERVER_ROOM_ID } from "sdk";
import { PinnedMessages } from "./menu/PinnedMessages.tsx";
import { Resizable } from "./Resizable.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { Inbox } from "./Inbox.tsx";
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
			<Resizable
				storageKey="channel-nav-width"
				side="left"
				initialWidth={256}
				minWidth={180}
				maxWidth={500}
			>
				<ChannelNav room_id={p.params.room_id} />
			</Resizable>
			<header>
				<b>home</b>
			</header>
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show
					when={flags.has("room_member_list") &&
						ctx.userConfig().frontend.showMembers !== false}
				>
					<Resizable
						storageKey="room-members-width"
						initialWidth={198}
						minWidth={180}
						maxWidth={500}
					>
						<RoomMembers room={room()!} />
					</Resizable>
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

export const RouteChannelSettings = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const channel = api.channels.fetch(() => p.params.channel_id);
	const title = () =>
		channel() ? t("page.settings_channel", channel()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<Show when={channel()}>
				<ChannelSettings channel={channel()!} page={p.params.page} />
			</Show>
		</>
	);
};

const ChannelSidebar = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const search = () => ctx.channel_search.get(props.channel.id);
	const showMembers = () =>
		props.channel.type !== "Voice" &&
		flags.has("channel_member_list") &&
		ctx.userConfig().frontend.showMembers !== false;
	const showPinned = () =>
		ctx.channel_pinned_view.get(props.channel.id) ?? false;
	const showVoiceChat = () =>
		props.channel.type === "Voice" &&
		ctx.voice_chat_sidebar_open.get(props.channel.id);

	return (
		<Switch>
			<Match when={showVoiceChat()}>
				<Resizable storageKey="voice-chat-sidebar-width" initialWidth={320}>
					<div class="voice-chat-sidebar">
						<ChatMain channel={props.channel} />
					</div>
				</Resizable>
			</Match>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults channel={props.channel} search={search()!} />
				</Resizable>
			</Match>
			<Match when={showPinned()}>
				<Resizable storageKey="pinned-sidebar-width" initialWidth={320}>
					<PinnedMessages channel={props.channel} />
				</Resizable>
			</Match>
			<Match when={showMembers()}>
				<Resizable
					storageKey="thread-members-width"
					initialWidth={198}
					minWidth={180}
					maxWidth={500}
				>
					<ThreadMembers thread={props.channel} />
				</Resizable>
			</Match>
		</Switch>
	);
};

export const RouteChannel = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const channel = api.channels.fetch(() => p.params.channel_id);
	const room = api.rooms.fetch(() => channel()?.room_id!);

	createEffect(() => {
		const { channel_id, message_id } = p.params;
		if (channel_id && message_id) {
			ctx.channel_anchor.set(channel_id, {
				type: "context",
				limit: 50,
				message_id: message_id,
			});
			ctx.channel_highlight.set(channel_id, message_id);
		} else if (channel_id) {
			const current_anchor = ctx.channel_anchor.get(channel_id);
			if (current_anchor?.type === "context") {
				ctx.channel_anchor.delete(channel_id);
			}
		}
	});

	// fetch channels to populate sidebar
	api.channels.list(() => channel()?.room_id!);

	const title = () => {
		const ch = channel();
		if (!ch) return t("loading");
		if (ch.type === "Dm") {
			const user_id = api.users.cache.get("@self")!.id;
			return ch.recipients.find((i) => i.id !== user_id)?.name ??
				"dm";
		}

		return room() && ch.room_id ? `${ch.name} - ${room()!.name}` : ch.name;
	};

	return (
		<>
			<Title title={title()} />
			<RoomNav />
			<Resizable
				storageKey="channel-nav-width"
				side="left"
				initialWidth={256}
				minWidth={180}
				maxWidth={500}
			>
				<ChannelNav room_id={channel()?.room_id ?? undefined} />
			</Resizable>
			<Show when={channel()}>
				<Show when={channel()!.type !== "Voice"}>
					<ChatHeader channel={channel()!} />
				</Show>
				<Show
					when={channel()!.type === "Text" ||
						channel()!.type === "Dm" ||
						channel()!.type === "Gdm" ||
						channel()!.type === "ThreadPublic" ||
						channel()!.type === "ThreadPrivate"}
				>
					<ChatMain channel={channel()!} />
				</Show>
				<Show when={channel()!.type === "Voice"}>
					<Voice channel={channel()!} />
				</Show>
				<Show when={channel()!.type === "Forum"}>
					<Forum channel={channel()!} />
				</Show>
				<Show when={channel()!.type === "Category"}>
					<Category channel={channel()!} />
				</Show>
				<ChannelSidebar channel={channel()!} />
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
			<Resizable
				storageKey="channel-nav-width"
				side="left"
				initialWidth={256}
				minWidth={180}
				maxWidth={500}
			>
				<ChannelNav />
			</Resizable>
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
				<Resizable
					storageKey="channel-nav-width"
					side="left"
					initialWidth={256}
					minWidth={180}
					maxWidth={500}
				>
					<ChannelNav room_id={p.params.room_id} />
				</Resizable>
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
			<Resizable
				storageKey="channel-nav-width"
				side="left"
				initialWidth={256}
				minWidth={180}
				maxWidth={500}
			>
				<ChannelNav room_id={p.params.room_id} />
			</Resizable>
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

export function RouteInbox(p: RouteSectionProps) {
	return (
		<>
			<Title title="inbox" />
			<RoomNav />
			<Resizable
				storageKey="channel-nav-width"
				side="left"
				initialWidth={256}
				minWidth={180}
				maxWidth={500}
			>
				<ChannelNav room_id={p.params.room_id} />
			</Resizable>
			<Inbox />
		</>
	);
}

export function RouteFriends() {
	const api = useApi();

	const [friends] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/friend",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const sendRequest = () => {
		const target_id = prompt("target_id");
		if (!target_id) return;
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id } },
		});
	};

	return (
		<>
			<Title title="friends" />
			<RoomNav />
			<div class="friends" style="padding:8px">
				todo!
				<ul>
					<li>foo</li>
					<li>bar</li>
					<li>baz</li>
				</ul>
				<pre>{JSON.stringify(friends())}</pre>
				<button onClick={sendRequest}>send request</button>
			</div>
		</>
	);
}
