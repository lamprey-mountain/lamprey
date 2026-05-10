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
import { ChannelSettings } from "@/components/features/channel_settings/index";
import { ChatMain } from "@/components/features/chat/Chat.tsx";
import { ChatHeader } from "@/components/features/chat/ChatHeader.tsx";
import { PinnedMessages } from "@/components/features/chat/PinnedMessages.tsx";
import { SearchResults } from "@/components/features/chat/SearchResults.tsx";
import { ThreadMembers } from "@/components/features/chat/Thread.tsx";
import { Document } from "@/components/features/editor/Document.tsx";
import { DocumentHistory } from "@/components/features/editor/DocumentHistory.tsx";
import { RoomSettings } from "@/components/features/room_settings/RoomSettings";
import { Scripts } from "@/components/features/scripts/Scripts";
import { Voice, VoiceTray } from "@/components/features/voice/Voice.tsx";
import { Calendar } from "@/components/shared/Calendar";
import { Category } from "@/components/shared/Category";
import { ChannelNav } from "@/components/shared/ChannelNav";
import { Feed } from "@/components/shared/Feed";
import { Forum } from "@/components/shared/Forum";
import { Forum2, Forum2ThreadPage } from "@/components/shared/Forum2";
import { Friends } from "@/components/shared/Friends";
import { Home } from "@/components/shared/Home";
import { Inbox } from "@/components/shared/Inbox";
import { RouteInviteInner } from "@/components/shared/Invite";
import { RoomHome, RoomMembers } from "@/components/shared/Room";
import { RoomHeader } from "@/components/shared/RoomHeader";
import { RoomNav } from "@/components/shared/RoomNav";
import { UserProfile } from "@/components/shared/UserProfile";
import { Wiki } from "@/components/shared/Wiki";
import {
	ChannelContext,
	createInitialChannelState,
	useChannel,
} from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import {
	createInitialDocumentState,
	DocumentContext,
	useDocument,
} from "@/contexts/document.tsx";
import {
	createInitialRoomState,
	RoomContext,
	useRoom,
} from "@/contexts/room.tsx";
import { flags } from "@/lib/flags";
import type { RoomT } from "@/types";
import type { ChannelSearch } from "@/types/chat";

export { RouteAuthorize } from "@/components/shared/Oauth";

const Title = (props: { title?: string }) => {
	createEffect(() => (document.title = props.title ?? ""));
	return undefined;
};

type LayoutDefaultProps = {
	title?: string;
	children?: JSX.Element;
	showChannelNav?: boolean;
	channelNavRoomId?: string;
	showVoiceTray?: boolean;
	showMembers?: boolean;
	memberComponent?: JSX.Element;
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
		ctx.preferences().frontend.showMembers !== false;

	return (
		<Switch>
			<Match when={search()}>
				<Resizable storageKey="search-sidebar-width" initialWidth={320}>
					<SearchResults
						room={props.room}
						search={search()! as ChannelSearch}
					/>
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
	const rooms = useRooms();
	const room = rooms.use(() => p.params.room_id);

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
					title={room() ? room()?.name : t("loading")}
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

	const getOrCreateDocumentContext = () => {
		const channelId = props.thread_id;
		if (!channelId) return null;

		if (!ctx.document_contexts.has(channelId)) {
			const store = createStore(createInitialDocumentState(channelId));
			ctx.document_contexts.set(channelId, store);
		}

		return ctx.document_contexts.get(channelId)!;
	};

	const documentCtx = createMemo(() => getOrCreateDocumentContext());
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
							<Show when={documentCtx()}>
								{(dc) => (
									<ChannelContext.Provider value={cc()}>
										<DocumentContext.Provider value={dc()}>
											<button type="button" class="close" onClick={onClose}>
												<Icon src={icX} />
											</button>
											<ChatMain channel={t()} />
										</DocumentContext.Provider>
									</ChannelContext.Provider>
								)}
							</Show>
						)}
					</Show>
				)}
			</Show>
		</div>
	);
};

const ChannelSidebar = (props: {
	channel: Channel;
	selectedSeq: ChangesetSelection | null;
	onSelectChangeset: (changeset: ChangesetSelection | null) => void;
	onHoverChangeset: (changeset: ChangesetSelection | null) => void;
}) => {
	const ctx = useCtx();
	const [ch] = useChannel()!;
	const [doc] = useDocument()!;
	const branchId = doc.branchId;
	const search = () => ch.search;
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
					<DocumentHistory
						channel={props.channel}
						branchId={branchId}
						isOpen={ch.history_view}
						selectedSeq={props.selectedSeq}
						onSelectChangeset={props.onSelectChangeset}
						onHoverChangeset={props.onHoverChangeset}
					/>
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
					<SearchResults
						channel={props.channel}
						search={search()! as ChannelSearch}
					/>
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

export const RouteChannel = (
	p: ParentProps<RouteSectionProps>,
): JSX.Element => {
	const { t } = useCtx();
	const ctx = useCtx();
	const rooms = useRooms();
	const channels2 = useChannels();
	const channel = channels2.use(() => p.params.channel_id);
	const room = rooms.use(() => channel()?.room_id!);

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

	const documentCtx = createMemo(() => getOrCreateDocumentContext());
	const channelCtx = createMemo(() => getOrCreateChannelContext());

	const [selectedSeq, setSelectedSeq] = createSignal<ChangesetSelection | null>(
		null,
	);
	const [hoverSeq, setHoverSeq] = createSignal<ChangesetSelection | null>(null);

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
		const { channel_id, message_id } = p.params;
		const c = channelCtx();
		if (!c) return;

		const [, setChannelState] = c;

		if (channel_id && message_id) {
			setChannelState("anchor", {
				type: "context",
				limit: 50,
				message_id: message_id,
			});
			setChannelState("highlight", message_id);
		} else if (channel_id) {
			if (c[0].anchor?.type === "context") {
				setChannelState("anchor", undefined);
			}
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
				<Show when={documentCtx()}>
					{(dc) => (
						<ChannelContext.Provider value={cc()}>
							<DocumentContext.Provider value={dc()}>
								<LayoutDefault
									title={title()}
									showChannelNav={true}
									channelNavRoomId={channel()?.room_id ?? undefined}
									showVoiceTray={true}
								>
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
														<Document
															channel={ch()}
															selectedSeq={selectedSeq()}
															onSelectChangeset={setSelectedSeq}
															hoverSeq={hoverSeq()}
															onHoverChangeset={setHoverSeq}
														/>
													</Match>
													<Match when={ch().type === "Wiki"}>
														<Wiki channel={ch()} />
													</Match>
													<Match when={ch().type === "Forum"}>
														<Forum channel={ch()} />
													</Match>
													<Match when={ch().type === "Forum2"}>
														<Forum2 channel={ch()} />
													</Match>
													<Match when={ch().type === "ThreadForum2"}>
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
												<ChannelSidebar
													channel={ch()}
													selectedSeq={selectedSeq()}
													onSelectChangeset={setSelectedSeq}
													onHoverChangeset={setHoverSeq}
												/>
											</>
										)}
									</Show>
								</LayoutDefault>
							</DocumentContext.Provider>
						</ChannelContext.Provider>
					)}
				</Show>
			)}
		</Show>
	);
};

export const RouteHome = (
	_props: ParentProps<RouteSectionProps>,
): JSX.Element => {
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

export const RouteFeed = (
	_props: ParentProps<RouteSectionProps>,
): JSX.Element => {
	return (
		<LayoutDefault title="feed" showChannelNav={false} showVoiceTray={true}>
			<Feed />
		</LayoutDefault>
	);
};

export const RouteInvite = (p: ParentProps<RouteSectionProps>): JSX.Element => {
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

export const RouteUser = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	const api2 = useApi();
	const user = api2.users.use(() => p.params.user_id!);

	return (
		<LayoutDefault
			title={user()?.name ?? "loading..."}
			showChannelNav={true}
			channelNavRoomId={p.params.room_id ?? ""}
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

export function RouteInbox(p: RouteSectionProps): JSX.Element {
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

export function RouteFriends(): JSX.Element {
	return (
		<LayoutDefault title="friends" showChannelNav={true} showVoiceTray={true}>
			<Friends />
		</LayoutDefault>
	);
}

export function RouteNotFound(): JSX.Element {
	const { t } = useCtx();

	return (
		<LayoutDefault title="not found" showChannelNav={true} showVoiceTray={true}>
			<div style="padding:8px">{t("not_found")}</div>
		</LayoutDefault>
	);
}
