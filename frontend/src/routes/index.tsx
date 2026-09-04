import type { RouteSectionProps } from "@solidjs/router";
import type { Channel } from "sdk";
import type { JSX, ParentProps } from "solid-js";
import {
	createEffect,
	createMemo,
	createSignal,
	Match,
	Show,
	Switch,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi, useChannels, useRooms } from "@/api";
import { useCtx } from "@/app/context";
import icX from "@/assets/x-1.png";
import { Icon } from "@/atoms/Icon";
import { Resizable } from "@/atoms/Resizable.tsx";
import { DocumentProvider } from "@/components/document/context";
import { Document } from "@/components/document/Document.tsx";
import { DocumentHistory } from "@/components/document/DocumentHistory";
import { Wiki } from "@/components/document/Wiki";
import { ChannelSettings } from "@/components/features/channel_settings/index";
import { ChatMain } from "@/components/features/chat/Chat.tsx";
import { ChatHeader } from "@/components/features/chat/ChatHeader.tsx";
import { PinnedMessages } from "@/components/features/chat/PinnedMessages.tsx";
import { SearchResults } from "@/components/features/chat/SearchResults.tsx";
import { ThreadMembers } from "@/components/features/chat/Thread.tsx";
import { Forum } from "@/components/features/forum/Forum";
import {
	Forum2,
	Forum2Thread,
	Forum2ThreadPage,
} from "@/components/features/forum/Forum2";
import { RoomSettings } from "@/components/features/room_settings/RoomSettings";
import { Scripts } from "@/components/features/scripts/Scripts";
import { UserSettings } from "@/components/features/user_settings";
import { Voice } from "@/components/features/voice/Voice.tsx";
import { Calendar } from "@/components/shared/Calendar";
import { Category } from "@/components/shared/Category";
import { ChannelNav } from "@/components/shared/ChannelNav";
import { Friends } from "@/components/shared/Friends";
import { Home } from "@/components/shared/Home";
import { Inbox } from "@/components/shared/Inbox";
import { RouteInviteInner } from "@/components/shared/Invite";
import { RoomHome, RoomMembers } from "@/components/shared/Room";
import { RoomHeader } from "@/components/shared/RoomHeader";
import { RoomNav } from "@/components/shared/RoomNav";
import { SearchPage } from "@/components/shared/SearchPage";
import { UserPage } from "@/components/shared/UserPage";
import { UserTray } from "@/components/shared/UserTray";
import {
	ChannelContext,
	createInitialChannelState,
	useChannel,
} from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { createInitialRoomState, RoomContext } from "@/contexts/room.tsx";
import { useSearch } from "@/contexts/search";
import { useCurrentRoomId } from "@/hooks/useCurrentRoomId";
import { flags } from "@/lib/flags";
import { MediaSidebar } from "@/media/Sidebar";
import type { RoomT } from "@/types";
import { icUser } from "@/utils/icons";

export { RouteAuthorize } from "@/components/shared/Oauth";

const Title = (props: { title?: string }) => {
	createEffect(() => (document.title = props.title ?? ""));
	return undefined;
};

export const AppLayoutMain = (props: ParentProps<RouteSectionProps>) => {
	const roomId = useCurrentRoomId();

	return (
		<>
			<Resizable
				storageKey="nav-tray-width"
				side="left"
				// 64px room nav + 256 channel nav
				initialWidth={320}
				// TODO: don't have magic numbers
				minWidth={244}
				maxWidth={564}
			>
				<div class="nav-tray">
					<RoomNav />
					<ChannelNav room_id={roomId()} />
					<UserTray />
				</div>
			</Resizable>
			{props.children}
		</>
	);
};

const RoomSidebar = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const { states } = useSearch();
	const search = () => states[props.room.id];

	const showMembers = () =>
		flags.has("room_member_list") &&
		ctx.preferences().frontend.showMembers !== false;

	return (
		<Switch>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults room={props.room} search={search()} />
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

export const RouteRoom = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const room = api.rooms.use(() => p.params.room_id);

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
				<Title title={room() ? room()?.name : t("loading")} />
				<Show when={room()}>
					<RoomHeader room={room()!} />
					<RoomHome room={room()!} />
					<RoomSidebar room={room()!} />
				</Show>
			</RoomContext.Provider>
		</Show>
	);
};

export const RouteRoomSettings = (
	p: ParentProps<RouteSectionProps>,
): JSX.Element => {
	const { t } = useCtx();
	const rooms = useRooms();
	const room = rooms.use(() => p.params.room_id);
	const title = () => {
		const r = room();
		return r?.name ? t("page.settings_room", r.name) : t("loading");
	};
	return (
		<>
			<Title title={title()} />
			<Show when={room()}>
				{(r) => <RoomSettings room={r()} page={p.params.page ?? ""} />}
			</Show>
		</>
	);
};

export const RouteChannelSettings = (
	p: ParentProps<RouteSectionProps>,
): JSX.Element => {
	const { t } = useCtx();
	const channels2 = useChannels();
	const channel = channels2.use(() => p.params.channel_id);
	const title = () => {
		const c = channel();
		return c?.name ? t("page.settings_channel", c.name) : t("loading");
	};
	return (
		<>
			<Title title={title()} />
			<Show when={channel()}>
				{(c) => <ChannelSettings channel={c()} page={p.params.page ?? ""} />}
			</Show>
		</>
	);
};

type ChangesetSelection = {
	start_seq: number;
	end_seq: number;
};

const ThreadChatSidebar = (props: { thread_id: string }) => {
	const channels2 = useChannels();
	const thread = channels2.use(() => props.thread_id);
	const ctx = useCtx();
	const [_ch, setChannelState] = useChannel()!;

	const getOrCreateChannelContext = () => {
		const channelId = props.thread_id;
		if (!channelId) return null;

		if (!ctx.channel_contexts.has(channelId)) {
			const store = createStore(createInitialChannelState());
			ctx.channel_contexts.set(channelId, store);
		}

		return ctx.channel_contexts.get(channelId)!;
	};

	const channelCtx = createMemo(() => getOrCreateChannelContext());

	const onClose = () => {
		setChannelState("thread_chat_sidebar_thread_id", undefined);
	};

	return (
		<div class="thread-chat-sidebar">
			<Show when={thread()}>
				{(t) => (
					<Show when={channelCtx()}>
						{(cc) => (
							<ChannelContext.Provider value={cc()}>
								<button type="button" class="close" onClick={onClose}>
									<Icon src={icX} />
								</button>
								<Switch>
									<Match when={t().type === "Document"}>
										<Document channel={t()} />
									</Match>
									<Match when={t().type === "ThreadForum2"}>
										<Forum2Thread channel={t()} />
									</Match>
									<Match when={true}>
										<ChatMain channel={t()} />
									</Match>
								</Switch>
							</ChannelContext.Provider>
						)}
					</Show>
				)}
			</Show>
		</div>
	);
};

const ChannelSidebar = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const [ch] = useChannel()!;
	const { states } = useSearch();
	const search = () => states[props.channel.id];
	const showMembers = () =>
		props.channel.type !== "Voice" &&
		flags.has("channel_member_list") &&
		ctx.preferences().frontend.showMembers !== false;
	const showPinned = () => ch.pinned_view ?? false;
	const showVoiceChat = () =>
		props.channel.type === "Voice" && ch.voice_chat_sidebar_open;
	const showHistory = () =>
		props.channel.type === "Document" && ch.history_view;
	const showThreadChatSidebar = () => ch.thread_chat_sidebar_thread_id;

	// TODO: make document sidebar and thread sidebar use different resizable storage keys

	return (
		<Switch>
			<Match when={showThreadChatSidebar()}>
				<Resizable
					storageKey="thread-chat-sidebar-width"
					initialWidth={400}
					minWidth={300}
					maxWidth={600}
				>
					<ThreadChatSidebar thread_id={ch.thread_chat_sidebar_thread_id!} />
				</Resizable>
			</Match>
			<Match when={showHistory()}>
				<Resizable storageKey="document-history-width" initialWidth={320}>
					<DocumentHistory channel={props.channel} />
				</Resizable>
			</Match>
			<Match when={showVoiceChat()}>
				<Resizable storageKey="voice-chat-sidebar-width" initialWidth={320}>
					<div class="voice-chat-sidebar">
						<ChatMain channel={props.channel} />
					</div>
				</Resizable>
			</Match>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults channel={props.channel} search={search()} />
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
			<Match when={ch.sidebar_media}>
				{(media) => (
					<Resizable
						storageKey="media-width"
						initialWidth={400}
						minWidth={300}
						maxWidth={600}
					>
						<MediaSidebar media={media()} />
					</Resizable>
				)}
			</Match>
		</Switch>
	);
};

export const RouteChannel = (
	p: ParentProps<RouteSectionProps>,
): JSX.Element => {
	const { t } = useCtx();
	const ctx = useCtx();
	const api = useApi();
	const channel = api.channels.use(() => p.params.channel_id);
	const room = api.rooms.use(() => channel()?.room_id!);

	const getOrCreateChannelContext = () => {
		const channelId = p.params.channel_id;
		if (!channelId) return null;

		if (!ctx.channel_contexts.has(channelId)) {
			const store = createStore(createInitialChannelState());
			ctx.channel_contexts.set(channelId, store);
		}

		return ctx.channel_contexts.get(channelId)!;
	};

	const channelCtx = createMemo(() => getOrCreateChannelContext());

	// store last viewed channel per room
	createEffect(() => {
		const ch = channel();
		const rm = room();
		if (ch?.room_id && rm) {
			const key = `last_channel_${rm.id}`;
			localStorage.setItem(key, ch.id);
		}
	});

	// Handle message anchor logic
	createEffect(() => {
		const { channel_id, message_id, script_id } = p.params;
		const c = channelCtx();
		if (!c) return;

		const [channelState, setChannelState] = c;

		if (channel_id && message_id) {
			channelState.timelineState.controller.jumpToMessage(
				message_id,
				false,
				true,
			);
		}
		if (channel_id && script_id) {
			setChannelState("script_id", script_id);
		}
	});

	const currentUser = useCurrentUser();
	const title = () => {
		const ch = channel();
		if (!ch) return t("loading");
		if (ch.type === "Dm") {
			const user_id = currentUser()?.id;
			return ch.recipients?.find((i) => i.id !== user_id)?.name ?? "dm";
		}

		return room() && ch.room_id ? `${ch.name} - ${room()?.name}` : ch.name;
	};

	return (
		<Show when={channelCtx()} fallback={<div>Loading channel...</div>}>
			{(cc) => (
				<ChannelContext.Provider value={cc()}>
					<Title title={title()} />
					<Show when={channel()}>
						{(ch) => (
							<>
								<Switch>
									<Match when={ch().type === "Voice"}>
										<Voice channel={ch()} />
									</Match>
									<Match
										when={
											ch().type === "Text" ||
											ch().type === "Dm" ||
											ch().type === "Gdm" ||
											ch().type === "Announcement" ||
											ch().type === "ThreadPublic" ||
											ch().type === "ThreadPrivate"
										}
									>
										<ChatHeader channel={ch()} />
										<ChatMain channel={ch()} />
									</Match>
									<Match when={ch().type === "Document"}>
										<DocumentProvider initialBranchId={ch().id}>
											<Document channel={ch()} />
											<ChannelSidebar channel={ch()} />
										</DocumentProvider>
									</Match>
									<Match when={ch().type === "Wiki"}>
										<ChatHeader channel={ch()} />
										<Wiki channel={ch()} />
									</Match>
									<Match when={ch().type === "Forum"}>
										<ChatHeader channel={ch()} />
										<Forum channel={ch()} />
									</Match>
									<Match when={ch().type === "Ticket"}>
										<ChatHeader channel={ch()} />
										<Forum channel={ch()} />
									</Match>
									<Match when={ch().type === "Forum2"}>
										<ChatHeader channel={ch()} />
										<Forum2 channel={ch()} />
									</Match>
									<Match when={ch().type === "ThreadForum2"}>
										<ChatHeader channel={ch()} />
										<Forum2ThreadPage channel={ch()} />
									</Match>
									<Match when={ch().type === "Calendar"}>
										<Calendar channel={ch()} />
									</Match>
									<Match when={ch().type === "Scripts"}>
										<Scripts channel={ch()} />
									</Match>
									<Match when={ch().type === "Category"}>
										<Category channel={ch()} />
									</Match>
								</Switch>
								<Show when={ch().type !== "Document"}>
									<ChannelSidebar channel={ch()} />
								</Show>
							</>
						)}
					</Show>
				</ChannelContext.Provider>
			)}
		</Show>
	);
};

export const RouteHome = (
	_props: ParentProps<RouteSectionProps>,
): JSX.Element => {
	const { t } = useCtx();
	return (
		<>
			<Title title={t("page.home")} />
			<Home />
		</>
	);
};

export const RouteInvite = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	return (
		<Show when={p.params.code}>
			<RouteInviteInner code={p.params.code!} />
		</Show>
	);
};

export const RouteUser = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	const api2 = useApi();
	const user = api2.users.use(() => p.params.user_id!);

	return (
		<>
			<Title title={user()?.name ?? "loading..."} />
			<Show when={user()}>
				{(u) => (
					<>
						<header class="chat-header">
							<div class="channel-icon">
								<Icon src={icUser} />
							</div>
							<div class="name">
								<h3 class="name-text">{u().name}</h3>
							</div>
							<div class="spacer"></div>
						</header>
						<UserPage user={u()} />
					</>
				)}
			</Show>
		</>
	);
};

export function RouteInbox(p: RouteSectionProps): JSX.Element {
	return (
		<>
			<Title title="inbox" />
			<Inbox />
		</>
	);
}

export function RouteFriends(): JSX.Element {
	return (
		<>
			<Title title="friends" />
			<Friends />
		</>
	);
}

export function RouteNotFound(): JSX.Element {
	const { t } = useCtx();

	return (
		<>
			<Title title="not found" />
			<div style="padding:8px">{t("not_found")}</div>
		</>
	);
}

export function RouteSettings(p: RouteSectionProps): JSX.Element {
	const { t } = useCtx();
	const user = useCurrentUser();
	createEffect(() => {
		console.log(user());
	});
	return (
		<>
			<Title title={user() ? t("page.settings_user") : t("loading")} />
			<Show when={user()}>
				{(u) => <UserSettings user={u()} page={p.params.page ?? ""} />}
			</Show>
		</>
	);
}

export function RouteSearch(): JSX.Element {
	const { t } = useCtx();

	return (
		<>
			<Title title={t("page.search")} />
			<SearchPage />
		</>
	);
}
