import { Navigate, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChannelNav } from "./ChannelNav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, createResource, Match, Show, Switch } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ChannelSettings } from "./ChannelSettings.tsx";
import { ChatHeader, ChatMain, SearchResults } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice, VoiceTray } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { RouteInviteInner } from "./Invite.tsx";
import { Forum } from "./Forum.tsx";
import { Category } from "./Category.tsx";
import { type Channel, SERVER_ROOM_ID } from "sdk";
import { PinnedMessages } from "./PinnedMessages.tsx";
import { Resizable } from "./Resizable.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { Inbox } from "./Inbox.tsx";
import { RoomNav } from "./RoomNav.tsx";
export { RouteAuthorize } from "./Oauth.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => (document.title = props.title ?? ""));
	return undefined;
};

type LayoutDefaultProps = {
	title?: string;
	children: any;
	showChannelNav?: boolean;
	channelNavRoomId?: string;
	showVoiceTray?: boolean;
	showMembers?: boolean;
	memberComponent?: any;
	showMembersWidth?: number;
};

export const LayoutDefault = (props: LayoutDefaultProps) => {
	const { t } = useCtx();

	return (
		<>
			<Title title={props.title ?? t("loading")} />
			<RoomNav />
			<Show when={props.showChannelNav !== false}>
				<Resizable
					storageKey="channel-nav-width"
					side="left"
					initialWidth={256}
					minWidth={180}
					maxWidth={500}
				>
					<ChannelNav room_id={props.channelNavRoomId} />
				</Resizable>
			</Show>
			{props.children}
			<Show when={props.showMembers}>
				<Resizable
					storageKey="room-members-width"
					initialWidth={props.showMembersWidth ?? 198}
					minWidth={180}
					maxWidth={500}
				>
					{props.memberComponent}
				</Resizable>
			</Show>
			<Show when={props.showVoiceTray !== false}>
				<VoiceTray />
			</Show>
		</>
	);
};

export const RouteRoom = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<LayoutDefault
			title={room() ? room()!.name : t("loading")}
			showChannelNav={true}
			channelNavRoomId={p.params.room_id}
			showVoiceTray={true}
			showMembers={flags.has("room_member_list") &&
				ctx.userConfig().frontend.showMembers !== false}
			memberComponent={room() ? <RoomMembers room={room()!} /> : undefined}
		>
			<header
				classList={{
					"menu-room": !!p.params.room_id,
				}}
				data-room-id={p.params.room_id}
			>
				<b>home</b>
			</header>
			<Show when={room()}>
				<RoomHome room={room()!} />
			</Show>
		</LayoutDefault>
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
		<LayoutDefault
			title={title()}
			showChannelNav={true}
			channelNavRoomId={channel()?.room_id ?? undefined}
			showVoiceTray={true}
		>
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
			</Show>
		</LayoutDefault>
	);
};

export const RouteHome = () => {
	const { t } = useCtx();
	return (
		<LayoutDefault
			title={t("page.home")}
			showChannelNav={true}
			showVoiceTray={true}
		>
			<Home />
		</LayoutDefault>
	);
};

export const RouteFeed = () => {
	return (
		<LayoutDefault
			title="feed"
			showChannelNav={false}
			showVoiceTray={true}
		>
			<Feed />
		</LayoutDefault>
	);
};

export const RouteInvite = (p: RouteSectionProps) => {
	return (
		<Show when={p.params.code}>
			<LayoutDefault
				showChannelNav={true}
				channelNavRoomId={p.params.room_id}
				showVoiceTray={true}
			>
				<RouteInviteInner code={p.params.code!} />
			</LayoutDefault>
		</Show>
	);
};

export const RouteUser = (p: RouteSectionProps) => {
	const api = useApi();
	const user = api.users.fetch(() => p.params.user_id);

	return (
		<LayoutDefault
			title={user()?.name ?? "loading..."}
			showChannelNav={true}
			channelNavRoomId={p.params.room_id}
			showVoiceTray={true}
		>
			<header class="chat-header">
				<b>{user()?.name}</b>
			</header>
			<Show when={user()}>
				<UserProfile user={user()!} />
			</Show>
		</LayoutDefault>
	);
};

export function RouteInbox(p: RouteSectionProps) {
	return (
		<LayoutDefault
			title="inbox"
			showChannelNav={true}
			channelNavRoomId={p.params.room_id}
			showVoiceTray={false}
		>
			<Inbox />
		</LayoutDefault>
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
		<LayoutDefault
			title="friends"
			showChannelNav={false}
			showVoiceTray={false}
		>
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
		</LayoutDefault>
	);
}
