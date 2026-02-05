import { Navigate, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChannelNav } from "./ChannelNav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { createEffect, createResource, Match, Show, Switch } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ChannelSettings } from "./ChannelSettings.tsx";
import { ChatHeader, ChatMain, RoomHeader, SearchResults } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice, VoiceTray } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { RouteInviteInner } from "./Invite.tsx";
import { Forum } from "./Forum.tsx";
import { Forum2 } from "./Forum2.tsx";
import { Category } from "./Category.tsx";
import { type Channel, SERVER_ROOM_ID } from "sdk";
import { PinnedMessages } from "./PinnedMessages.tsx";
import { Resizable } from "./Resizable.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { Inbox } from "./Inbox.tsx";
import { RoomNav } from "./RoomNav.tsx";
import { ChannelContext, useChannel } from "./channelctx.tsx";
import { createInitialChannelState } from "./channelctx.tsx";
import {
	createInitialRoomState,
	RoomContext,
	useRoom,
} from "./contexts/room.tsx";
import { createStore } from "solid-js/store";
import { RoomT } from "./types.ts";
import { Friends } from "./Friends.tsx";
import { Calendar } from "./Calendar.tsx";
import { Document, Wiki } from "./Document.tsx";
import {
	createInitialDocumentState,
	DocumentContext,
} from "./contexts/document.tsx";
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

const RoomSidebar = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const roomCtx = useRoom();
	const search = () => roomCtx?.[0].search;

	const showMembers = () =>
		flags.has("room_member_list") &&
		ctx.userConfig().frontend.showMembers !== false;

	return (
		<Switch>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults room={props.room} search={search()!} />
				</Resizable>
			</Match>
			<Match when={showMembers()}>
				<Resizable
					storageKey="room-members-width"
					initialWidth={198}
					minWidth={180}
					maxWidth={500}
				>
					<RoomMembers room={props.room} />
				</Resizable>
			</Match>
		</Switch>
	);
};

export const RouteRoom = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);

	const getOrCreateRoomContext = () => {
		const roomId = p.params.room_id;
		if (!roomId) return null;

		if (!ctx.room_contexts.has(roomId)) {
			const store = createStore(createInitialRoomState());
			ctx.room_contexts.set(roomId, store);
		}

		return ctx.room_contexts.get(roomId)!;
	};

	const roomCtx = getOrCreateRoomContext();

	return (
		<Show when={roomCtx} fallback={<div>Loading room...</div>}>
			<RoomContext.Provider value={roomCtx!}>
				<LayoutDefault
					title={room() ? room()!.name : t("loading")}
					showChannelNav={true}
					channelNavRoomId={p.params.room_id}
					showVoiceTray={true}
				>
					<Show when={room()}>
						<RoomHeader room={room()!} />
						<RoomHome room={room()!} />
						<RoomSidebar room={room()!} />
					</Show>
				</LayoutDefault>
			</RoomContext.Provider>
		</Show>
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
	const [ch] = useChannel()!;
	const search = () => ch.search;
	const showMembers = () =>
		props.channel.type !== "Voice" &&
		flags.has("channel_member_list") &&
		ctx.userConfig().frontend.showMembers !== false;
	const showPinned = () => ch.pinned_view ?? false;
	const showVoiceChat = () =>
		props.channel.type === "Voice" &&
		ch.voice_chat_sidebar_open;

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

	const getOrCreateChannelContext = () => {
		const channelId = p.params.channel_id;
		if (!channelId) return null;

		if (!ctx.channel_contexts.has(channelId)) {
			const store = createStore(createInitialChannelState());
			ctx.channel_contexts.set(channelId, store);
		}

		return ctx.channel_contexts.get(channelId)!;
	};

	// TODO: don't create document context if the channel isnt a document
	const getOrCreateDocumentContext = () => {
		const channelId = p.params.channel_id;
		if (!channelId) return null;

		if (!ctx.document_contexts.has(channelId)) {
			const store = createStore(createInitialDocumentState(channelId));
			ctx.document_contexts.set(channelId, store);
		}

		return ctx.document_contexts.get(channelId)!;
	};

	const documentCtx = getOrCreateDocumentContext();
	const channelCtx = getOrCreateChannelContext();

	// Handle message anchor logic
	createEffect(() => {
		const { channel_id, message_id } = p.params;
		if (!channelCtx) return;

		const [, setChannelState] = channelCtx;

		if (channel_id && message_id) {
			setChannelState("anchor", {
				type: "context",
				limit: 50,
				message_id: message_id,
			});
			setChannelState("highlight", message_id);
		} else if (channel_id) {
			if (channelCtx[0].anchor?.type === "context") {
				setChannelState("anchor", undefined);
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
		<Show when={channelCtx} fallback={<div>Loading channel...</div>}>
			<ChannelContext.Provider value={channelCtx}>
				<DocumentContext.Provider value={documentCtx}>
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
									channel()!.type === "Announcement" ||
									channel()!.type === "ThreadPublic" ||
									channel()!.type === "ThreadPrivate"}
							>
								<ChatMain channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Voice"}>
								<Voice channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Document"}>
								<Document channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Wiki"}>
								<Wiki channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Forum"}>
								<Forum channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Forum2"}>
								<Forum2 channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Calendar"}>
								<Calendar channel={channel()!} />
							</Show>
							<Show when={channel()!.type === "Category"}>
								<Category channel={channel()!} />
							</Show>
							<ChannelSidebar channel={channel()!} />
						</Show>
					</LayoutDefault>
				</DocumentContext.Provider>
			</ChannelContext.Provider>
		</Show>
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
	return (
		<LayoutDefault
			title="friends"
			showChannelNav={true}
			showVoiceTray={true}
		>
			<Friends />
		</LayoutDefault>
	);
}
