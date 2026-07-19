import { useNavigate } from "@solidjs/router";
import {
	type Attachment,
	type Channel as ChannelT,
	getTimestampFromUUID,
	type Message as MessageT,
	type MessageVersion as MessageVersionT,
	type Preferences,
	type ReactionKey,
	type UserWithRelationship,
} from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	type JSX,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import {
	useApi,
	useChannels,
	useFlumes,
	useMessages,
	useRoomMembers,
	useUsers,
} from "@/api";
import { useCtx } from "@/app/context";
import icEdit from "@/assets/edit.png";
import icGear from "@/assets/gear.png";
import icMemberAdd from "@/assets/member-add.png";
import icMemberJoin from "@/assets/member-join.png";
import icMemberRemove from "@/assets/member-remove.png";
import icMore from "@/assets/more.png";
import icPin from "@/assets/pin.png";
import icReactionAdd from "@/assets/reaction-add.png";
import icReply from "@/assets/reply.png";
import icThread from "@/assets/threads.png";
import { Components } from "@/atoms/Components.tsx";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown.tsx";
import { Time } from "@/atoms/Time";
import { Avatar2 } from "@/avatar/UserAvatar.tsx";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { serializeToMarkdown } from "@/components/features/editor/serializer.ts";
import { EmbedView } from "@/components/shared/UrlEmbed";
import { Avatar } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useOptionalChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useMenu, useUserPopout } from "@/contexts/mod.tsx";
import { useModals } from "@/contexts/modal";
import { colors } from "@/lib/colors.ts";
import { countEmojiOnly } from "@/lib/markdown";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "@/media/mod.tsx";
import { openThread } from "@/utils/channel";
import { icChannelMove, icSword } from "@/utils/icons.ts";
import { useMessageToolbar } from "./message-toolbar-context.tsx";
import { Reactions } from "./Reactions.tsx";

export type MessageProps = {
	message: MessageT;
	separate?: boolean;
	diff?: boolean;
};

// TODO: move elsewhere
// TODO: extract user name logic into a hook
export function UserDisplayName(props: {
	user_id: string;
	room_id?: string;
	thread_id?: string;
	onClick?: boolean;
	class?: string;
}) {
	const roomMembers2 = useRoomMembers();
	const users2 = useUsers();
	const { userView, setUserView } = useUserPopout();

	const room_member = () =>
		props.room_id
			? roomMembers2.cache.get(`${props.room_id}:${props.user_id}`)
			: null;
	const user = () => users2.cache.get(props.user_id);

	const name = () => room_member()?.override_name ?? user()?.name;

	const handleClick = (e: MouseEvent) => {
		if (!props.onClick) return;
		e.stopPropagation();
		e.preventDefault();
		const currentTarget = e.currentTarget as HTMLElement;
		if (userView()?.ref === currentTarget) {
			setUserView(null);
		} else {
			setUserView({
				user_id: props.user_id,
				room_id: props.room_id,
				thread_id: props.thread_id,
				ref: currentTarget,
				source: "message",
			});
		}
	};

	return (
		<span
			class={`user ${props.class ?? ""}`}
			classList={{ "menu-user": props.onClick }}
			data-user-id={props.user_id}
			onClick={handleClick}
		>
			{name()}
		</span>
	);
}

function MessageTextMarkdown(props: { message: MessageT; diff?: boolean }) {
	const [, modalctl] = useModals();
	const viewHistory = () => {
		modalctl.open({
			type: "message_edits",
			message_id: props.message.id,
			channel_id: props.message.channel_id,
		});
	};

	const content = () =>
		props.message.latest_version.type === "DefaultMarkdown"
			? (props.message.latest_version.content ?? "")
			: "";

	const emojiCount = () => countEmojiOnly(content());
	const isEmojiOnly = () => emojiCount() > 0 && emojiCount() <= 20;

	return (
		<Markdown
			content={content()}
			channel_id={props.message.channel_id}
			class="body"
			classList={{ local: props.message.is_local, "emoji-only": isEmojiOnly() }}
			kindaInline
			allowDiffFormatting={props.diff}
		>
			<Show when={props.message.id !== props.message.latest_version.version_id}>
				<span class="edited" onClick={viewHistory}>
					(edited)
				</span>
			</Show>
		</Markdown>
	);
}

// TODO: move elsewhere
function MessageEditor(props: { message: MessageT }) {
	const messagesService = useMessages();
	const [ch, chUpdate] = useOptionalChannel();

	const [draft, setDraft] = createSignal(
		props.message.latest_version.type === "DefaultMarkdown"
			? (props.message.latest_version.content ?? "")
			: "",
	);

	if (!ch || !chUpdate) {
		return <div class="message-editor">Error: No channel context</div>;
	}

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.message.channel_id ?? "",
		roomId: () => props.message.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => draft(),
		initialSelection: ch.editingMessage?.selection,
		keymap: {
			ArrowUp: (state) => {
				if (state.selection.from !== 1) return false;

				const ranges = messagesService._ranges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex(
					(m) => m.id === props.message.id,
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex - 1; i >= 0; i--) {
					const msg = messages[i];
					if (msg.latest_version.type === "DefaultMarkdown") {
						chUpdate("editingMessage", {
							message_id: msg.id,
							selection: "end",
						});
						return true;
					}
				}

				return false;
			},
			ArrowDown: (state) => {
				if (state.selection.to !== state.doc.content.size - 1) return false;

				const ranges = messagesService._ranges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex(
					(m) => m.id === props.message.id,
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex + 1; i < messages.length; i++) {
					const msg = messages[i];
					if (msg.latest_version.type === "DefaultMarkdown") {
						chUpdate("editingMessage", {
							message_id: msg.id,
							selection: "start",
						});
						return true;
					}
				}

				chUpdate("editingMessage", undefined);
				ch.input_focus?.();
				return true;
			},
		},
	});

	const save = async (content: string) => {
		const oldContent =
			props.message.latest_version.type === "DefaultMarkdown"
				? (props.message.latest_version.content ?? "")
				: "";
		if (content.trim() === oldContent.trim()) {
			chUpdate("editingMessage", undefined);
			return;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return;
		}
		try {
			await messagesService.edit(
				props.message.channel_id,
				props.message.id,
				content,
			);
		} catch (e) {
			console.error("failed to edit message", e);
		}
		chUpdate("editingMessage", undefined);
	};

	const cancel = () => {
		chUpdate("editingMessage", undefined);
		ch.input_focus?.();
	};

	let containerRef: HTMLDivElement | undefined;
	onMount(() => {
		containerRef?.addEventListener(
			"keydown",
			(e) => {
				if (e.key === "Escape") {
					e.stopPropagation();
					cancel();
				}
			},
			{ capture: true },
		);
		editor.focus();
	});

	return (
		<div class="message-editor" ref={containerRef}>
			<editor.View
				placeholder="edit message..."
				onSubmit={(text) => {
					save(text);
					return true;
				}}
				onChange={(state) => {
					const text = serializeToMarkdown(state.doc);
					setDraft(text);
				}}
			/>
			<div class="edit-info dim">
				escape to{" "}
				<button type="button" class="button" onClick={cancel}>
					cancel
				</button>{" "}
				• enter to{" "}
				<button type="button" class="button" onClick={() => save(draft())}>
					save
				</button>
			</div>
		</div>
	);
}

export function MessageThread(props: {
	thread: ChannelT;
	parentChannel: ChannelT;
	preferences: Preferences;
}) {
	const nav = useNavigate();
	const [chan, setChan] = useOptionalChannel();
	const channels = useChannels();
	const ctx = useCtx();

	const openThreadClick = () => {
		if (!props.thread.parent_id) return;
		const parentChannel = channels.get(props.thread.parent_id);
		if (!chan || !parentChannel) return;
		openThread(props.thread, parentChannel, ctx.preferences(), setChan, nav);
	};

	const lastActivityAt = () =>
		getTimestampFromUUID(props.thread.last_version_id ?? props.thread.id);

	return (
		<div class="message-thread">
			<div class="main" onClick={openThreadClick}>
				<div class="top">
					<div class="name">{props.thread.name}</div>
					<div class="count">{props.thread.message_count} messages</div>
				</div>
				<div>
					last message <Time date={lastActivityAt()} />
				</div>
			</div>
		</div>
	);
}

type SystemMessageProps = {
	message: MessageT;
	date: Date;
	separate: boolean;
	toolbarVisible: boolean;
	handleClick: (e: MouseEvent) => void;
	onMouseDown: (e: MouseEvent) => void;
	handleAltClick: (e: MouseEvent) => void;
	setHovered: (v: boolean) => void;
	messageArticleRef: (el: HTMLElement | undefined) => void;
	room_id?: string;
};

function SystemMessage(
	props: SystemMessageProps & {
		icon: string;
		content: JSX.Element;
		class?: string;
	},
) {
	const toolbar = useMessageToolbar();

	return (
		<article
			ref={props.messageArticleRef}
			class={`message menu-message oneline ${props.class ?? ""}`}
			data-message-id={props.message.id}
			classList={{
				separate: props.separate,
			}}
			onClick={props.handleClick}
			onMouseDown={(e) => {
				props.onMouseDown(e);
				props.handleAltClick(e);
			}}
			onMouseEnter={(e) => {
				props.setHovered(true);
				toolbar.setTarget({ message: props.message, element: e.currentTarget });
			}}
			onMouseLeave={(e) => {
				props.setHovered(false);
				const toolbarEl = toolbar.containerRef();
				if (
					toolbarEl &&
					e.relatedTarget instanceof Node &&
					toolbarEl.contains(e.relatedTarget)
				) {
					return;
				}
				toolbar.setTarget(null);
			}}
		>
			<aside class="aside">
				<img class="icon" src={props.icon} />
			</aside>
			<div class="content">
				{props.content}
				<Time
					date={props.date}
					animGroup="message-ts"
					class="onlytime"
					format="time"
				/>
				<Time
					date={props.date}
					animGroup="message-ts"
					class="full"
					format="full"
				/>
			</div>
		</article>
	);
}

export function ReplyView(props: {
	channel_id: string;
	reply_id: string;
	source_id: string;
	room_id?: string;
}) {
	const messages = useMessages();
	const users = useUsers();
	const reply = messages.use(
		() => props.channel_id,
		() => props.reply_id,
	);
	const [ch, chUpdate] = useOptionalChannel();

	const content = () => {
		const r = reply();
		if (!r) return;
		// TODO: handle embeds, components only messages
		// TODO: apply different style for messages without content
		return (
			(r.latest_version.type === "DefaultMarkdown" &&
				r.latest_version.content) ||
			(r.latest_version.type === "DefaultMarkdown" &&
			r.latest_version.attachments
				? `${r.latest_version.attachments.length} attachment(s)`
				: undefined)
		);
	};

	const scrollToReply = () => {
		if (!chUpdate) return;
		chUpdate("reply_jump_source", props.source_id);
		ch.timeline.jumpToMessage(props.reply_id, true, true);
	};

	const author = users.use(() => reply()?.author_id);

	return (
		<div class="reply" onClick={scrollToReply}>
			<div class="spine"></div>
			<Show when={true && author()} fallback={<div class="avatar"></div>}>
				{(a) => <Avatar animate={false} user={a()} />}
			</Show>
			<div class="content">
				<Show when={!reply.loading} fallback="loading...">
					<Show when={reply()}>
						{(r) => (
							<>
								<UserDisplayName
									user_id={r().author_id}
									room_id={props.room_id}
									thread_id={r().channel_id}
									onClick
									class="author"
								/>
								{/* TODO: typescript compatibility */}
								<Show
									when={r().latest_version.type === "DefaultMarkdown"}
									fallback={content()}
								>
									<Markdown
										content={(r().latest_version as any).content}
										channel_id={props.channel_id}
										inline
									/>
								</Show>
							</>
						)}
					</Show>
				</Show>
			</div>
		</div>
	);
}

export function AttachmentView(props: { att: Attachment }) {
	if (props.att.type !== "Media" || !props.att.media) return null;
	const b = () => props.att.media?.content_type.split("/")[0];
	if (b() === "image") {
		return (
			<li class="raw">
				<ImageView media={props.att.media} />
			</li>
		);
	} else if (b() === "video") {
		return (
			<li class="raw">
				<VideoView media={props.att.media} />
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li class="raw">
				<AudioView media={props.att.media} />
			</li>
		);
	} else if (
		b() === "text" ||
		/^application\/json\b/.test(props.att.media?.content_type)
	) {
		return (
			<li class="raw">
				<TextView media={props.att.media} />
			</li>
		);
	} else {
		return (
			<li>
				<FileView media={props.att.media} />
			</li>
		);
	}
}

// TODO: rename to Message
export function MessageView(props: MessageProps) {
	const channels = useChannels();
	const messagesService = useMessages();
	const ctx = useCtx();
	const { menu } = useMenu();
	const thread = channels.use(() => props.message.channel_id);
	const [ch, chUpdate] = useOptionalChannel();
	let messageArticleRef: HTMLElement | undefined;
	const [hovered, setHovered] = createSignal(false);

	const users2 = useUsers();
	const user = users2.use(() => props.message.author_id);

	const isMenuOpen = () => {
		const m = menu();
		if (!m) return false;
		return m.type === "message" && m.message_id === props.message.id;
	};

	const isReactionPickerOpen = () => {
		const popout = ctx.popout();
		if (
			!popout ||
			!("id" in popout) ||
			popout.id !== "emoji" ||
			!popout.ref ||
			!messageArticleRef
		) {
			return false;
		}
		return messageArticleRef.contains(popout.ref);
	};
	const toolbarVisible = () => isMenuOpen() || isReactionPickerOpen();

	const inSelectMode = () => ch?.selectMode ?? false;

	const onMouseDown = (e: MouseEvent) => {
		if (inSelectMode() && e.shiftKey) {
			e.preventDefault();
		}
	};

	const handleAltClick = (e: MouseEvent) => {
		if (!e.altKey || !ch || !chUpdate) return;
		e.preventDefault();
		e.stopPropagation();

		const thread_id = props.message.channel_id;
		const message_id = props.message.id;
		const messages = messagesService._ranges.get(thread_id)?.live.items ?? [];
		const currentIndex = messages.findIndex((m) => m.id === message_id);

		if (currentIndex === -1) return;

		const prevMessage = messages[currentIndex - 1];
		if (prevMessage) {
			chUpdate("read_marker_id", prevMessage.id);
		} else {
			chUpdate("read_marker_id", undefined);
		}
	};

	const handleClick = (e: MouseEvent) => {
		if (!inSelectMode() || !ch || !chUpdate) return;
		e.preventDefault();
		e.stopPropagation();

		const thread_id = props.message.channel_id;
		const message_id = props.message.id;
		const selected = ch.selectedMessages;

		if (e.shiftKey && selected.length > 0) {
			const lastSelected = selected[selected.length - 1];
			const messages = messagesService._ranges.get(thread_id)?.live.items ?? [];
			const lastIndex = messages.findIndex((m) => m.id === lastSelected);
			const currentIndex = messages.findIndex((m) => m.id === message_id);

			if (lastIndex !== -1 && currentIndex !== -1) {
				const start = Math.min(lastIndex, currentIndex);
				const end = Math.max(lastIndex, currentIndex);
				const rangeIds = messages.slice(start, end + 1).map((m) => m.id);
				const newSelected = [...new Set([...selected, ...rangeIds])];
				chUpdate("selectedMessages", newSelected);
			}
		} else {
			if (selected.includes(message_id)) {
				chUpdate(
					"selectedMessages",
					selected.filter((id) => id !== message_id),
				);
			} else {
				chUpdate("selectedMessages", [...selected, message_id]);
			}
		}
	};

	const date = createMemo(() => {
		return new Date(
			props.message.latest_version.created_at ??
				props.message.created_at ??
				new Date().toString(),
		);
	});

	const isEditing = () => {
		return ch?.editingMessage?.message_id === props.message.id;
	};

	const systemProps = {
		get message() {
			return props.message;
		},
		get diff() {
			return props.diff;
		},
		get date() {
			return date();
		},
		get separate() {
			return props.separate ?? false;
		},
		get toolbarVisible() {
			return toolbarVisible();
		},
		handleClick,
		onMouseDown,
		handleAltClick,
		setHovered,
		messageArticleRef: (el: HTMLElement | undefined) =>
			(messageArticleRef = el),
		get room_id() {
			return (thread() as any)?.room_id;
		},
	};

	return (
		<Switch
			fallback={
				<article
					ref={messageArticleRef}
					class="message menu-message"
					data-message-id={props.message.id}
					classList={{ "toolbar-visible": toolbarVisible() }}
					onClick={handleClick}
					onMouseDown={(e) => {
						onMouseDown(e);
						handleAltClick(e);
					}}
					onMouseEnter={() => setHovered(true)}
					onMouseLeave={() => setHovered(false)}
				>
					unknown message: {props.message.latest_version.type}
					{/* TODO: re-add message toolbar? */}
				</article>
			}
		>
			<Match when={props.message.latest_version.type === "MemberAdd"}>
				<SystemMessageMemberAdd {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "MemberRemove"}>
				<SystemMessageMemberRemove {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "MemberJoin"}>
				<SystemMessageMemberJoin {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "MessagePinned"}>
				<SystemMessagePinned {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "ChannelRename"}>
				<SystemMessageChannelRename {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "Call"}>
				<SystemMessageCall {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "ChannelPingback"}>
				<SystemMessageChannelPingback {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "ChannelIcon"}>
				<SystemMessageChannelIcon {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "ThreadCreated"}>
				<SystemMessageThreadCreated {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "AutomodExecution"}>
				<SystemMessageAutomodExecution {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "ChannelMoved"}>
				<SystemMessageChannelMoved {...systemProps} />
			</Match>
			<Match when={props.message.latest_version.type === "DefaultMarkdown"}>
				<DefaultMessage
					{...systemProps}
					user={user()}
					hovered={hovered()}
					isEditing={isEditing()}
					channels2={channels}
					ctx={ctx}
				/>
			</Match>
		</Switch>
	);
}

// TODO: move props into DefaultMessageProps
function DefaultMessage(
	props: SystemMessageProps & {
		user: UserWithRelationship | undefined;
		hovered: boolean;
		isEditing: boolean;
		channels2: ReturnType<typeof useChannels>;
		ctx: ReturnType<typeof useCtx>;
		diff?: boolean;
	},
) {
	const flumes = useFlumes();
	const toolbar = useMessageToolbar();
	const version = () =>
		props.message.latest_version.type === "DefaultMarkdown"
			? props.message.latest_version
			: null;
	const flume = () =>
		props.message.flume?.state === "Live" && flumes.get(props.message.id);

	const isCozy = () =>
		(props.ctx.preferences().frontend.message_style || "cozy") === "cozy";

	const openUserView = (e: MouseEvent) => {
		e.stopPropagation();
		const currentTarget = e.currentTarget as HTMLElement;
		const { userView, setUserView } = useUserPopout();
		if (userView()?.ref === currentTarget) {
			setUserView(null);
		} else {
			setUserView({
				user_id: props.message.author_id,
				room_id: props.room_id,
				channel_id: props.message.channel_id,
				ref: currentTarget,
				source: "message",
			});
		}
	};

	return (
		<article
			ref={props.messageArticleRef}
			class="message menu-message"
			data-message-id={props.message.id}
			data-author-id={props.message.author_id}
			classList={{
				separate: props.separate,
			}}
			onClick={props.handleClick}
			onMouseDown={(e) => {
				props.onMouseDown(e);
				props.handleAltClick(e);
			}}
			onMouseEnter={(e) => {
				props.setHovered(true);
				toolbar.setTarget({ message: props.message, element: e.currentTarget });
			}}
			onMouseLeave={(e) => {
				props.setHovered(false);
				const toolbarEl = toolbar.containerRef();
				if (
					toolbarEl &&
					e.relatedTarget instanceof Node &&
					toolbarEl.contains(e.relatedTarget)
				) {
					return;
				}
				toolbar.setTarget(null);
			}}
		>
			<Show when={version()?.reply_id}>
				{(reply) => (
					<ReplyView
						channel_id={props.message.channel_id}
						reply_id={reply()}
						source_id={props.message.id}
						room_id={props.room_id}
					/>
				)}
			</Show>

			<aside class="aside">
				<Avatar user={props.user} animate={props.hovered} />
				<Time date={props.date} animGroup="message-ts" format="time" />
				<Show when={props.message.thread}>
					<div class="thread-spine"></div>
				</Show>
			</aside>

			<div class="content">
				<h3 class="header">
					<Show when={flume()}>
						<div class="flume-spinner">
							<Icon src={icGear} color={colors.fg600} />
						</div>
					</Show>
					<UserDisplayName
						user_id={props.message.author_id}
						room_id={props.room_id}
						thread_id={props.message.channel_id}
						onClick
						class="author"
					/>
					<Time
						date={props.date}
						animGroup="message-ts"
						class="onlytime"
						format="time"
					/>
					<Time
						date={props.date}
						animGroup="message-ts"
						class="full"
						format="full"
					/>
				</h3>

				<Show
					when={!props.isEditing}
					fallback={<MessageEditor message={props.message} />}
				>
					<MessageTextMarkdown message={props.message} diff={props.diff} />
				</Show>
			</div>

			<div class="accessories">
				{/* attachments */}
				<Show when={version()?.attachments?.length}>
					<ul class="attachments">
						<For each={version()?.attachments}>
							{(att) => <AttachmentView att={att} />}
						</For>
					</ul>
				</Show>

				{/* embeds */}
				<Show when={version()?.embeds?.length}>
					<ul class="embeds">
						<For each={version()?.embeds}>
							{(embed) => <EmbedView embed={embed} />}
						</For>
					</ul>
				</Show>

				{/* flume */}
				<Show when={flume()}>
					{(f) => (
						<Components
							components={f().components}
							channelId={props.message.channel_id}
						/>
					)}
				</Show>

				{/* components */}
				<Show when={version()?.components?.length && !flume()}>
					<Components
						components={version()?.components ?? []}
						channelId={props.message.channel_id}
					/>
				</Show>

				{/* reactions */}
				<Show
					when={props.message.reactions && props.message.reactions.length > 0}
				>
					<Reactions message={props.message} />
				</Show>

				{/* thread */}
				<Show when={props.message.thread}>
					{(thread) => (
						<Show when={props.channels2.get(props.message.channel_id)}>
							{(parentChannel) => (
								<MessageThread
									thread={thread()}
									parentChannel={parentChannel()}
									preferences={props.ctx.preferences()}
								/>
							)}
						</Show>
					)}
				</Show>
			</div>
		</article>
	);
}

function SystemMessageMemberAdd(props: SystemMessageProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			target_user_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icMemberAdd}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_add",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<span class="author">
							<UserDisplayName
								user_id={version().target_user_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageMemberRemove(props: SystemMessageProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			target_user_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icMemberRemove}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_remove",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<span class="author">
							<UserDisplayName
								user_id={version().target_user_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageMemberJoin(props: SystemMessageProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icMemberJoin}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_join",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessagePinned(props: SystemMessageProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			pinned_message_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icPin}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.message_pinned",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						(text: string) => (
							<button
								type="button"
								style="color: oklch(var(--color-fg1))"
								class="link"
								onClick={(e) => {
									e.stopPropagation();
									navigate(
										`/channel/${props.message.channel_id}/message/${
											version().pinned_message_id
										}`,
									);
								}}
							>
								{text}
							</button>
						),
					)}
				</div>
			}
		/>
	);
}

function SystemMessageChannelRename(props: SystemMessageProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & { name_new: string };

	return (
		<SystemMessage
			{...props}
			icon={icEdit}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_rename",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<b>{version().name_new}</b>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageCall(props: SystemMessageProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			ended_at?: string | null;
			participants: string[];
		};

	return (
		<SystemMessage
			{...props}
			icon={icMemberJoin}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{version().ended_at
						? t(
								"message_content.call_ended",
								<span class="author">
									<UserDisplayName
										user_id={props.message.author_id}
										room_id={props.room_id}
										onClick
									/>
								</span>,
								version().participants.length,
							)
						: t(
								"message_content.call_started",
								<span class="author">
									<UserDisplayName
										user_id={props.message.author_id}
										room_id={props.room_id}
										onClick
									/>
								</span>,
								version().participants.length,
							)}
				</div>
			}
		/>
	);
}

function SystemMessageChannelPingback(props: SystemMessageProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icReply}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_pingback",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageChannelIcon(props: SystemMessageProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icEdit}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_icon",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageThreadCreated(props: SystemMessageProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const ctx = useCtx();

	const threadId = () =>
		(props.message.latest_version as MessageVersionT & { thread_id?: string })
			.thread_id;

	const link = (text: string) => (
		<Show when={threadId()} fallback={<span>{text}</span>}>
			<button
				type="button"
				class="link"
				onClick={(e) => {
					e.stopPropagation();
					if (threadId()) {
						navigate(`/channel/${threadId()}`);
					}
				}}
			>
				{text}
			</button>
		</Show>
	);

	const viewAll = (text: string) => (
		<button
			type="button"
			class="link"
			onClick={(e) => {
				e.stopPropagation();
				const ref = ctx.headerThreadsButtonRef() ?? e.currentTarget;
				queueMicrotask(() => {
					ctx.setThreadsView({
						channel_id: props.message.channel_id,
						ref,
					});
				});
			}}
		>
			{text}
		</button>
	);

	return (
		<SystemMessage
			{...props}
			icon={icThread}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.thread_created",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						link,
						viewAll,
					)}
				</div>
			}
		/>
	);
}

function SystemMessageChannelMoved(props: SystemMessageProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const channels = useChannels();

	const m = () =>
		props.message.latest_version as MessageVersionT & { type: "ChannelMoved" };
	const oldChan = channels.use(() => m().parent_id_old ?? undefined);

	return (
		<SystemMessage
			{...props}
			icon={icChannelMove}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_moved",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<button
							type="button"
							class="link"
							onClick={(e) => {
								e.stopPropagation();
								const oldId = m().parent_id_old;
								if (oldId) {
									navigate(`/channel/${oldId}`);
								}
							}}
						>
							<Show when={oldChan()} fallback={<em>unknown channel</em>}>
								{(c) => c().name}
							</Show>
						</button>,
					)}
				</div>
			}
		/>
	);
}

// TODO: better component for automod executions
function SystemMessageAutomodExecution(props: SystemMessageProps) {
	const { t } = useCtx();

	// const m = () => props.message.latest_version as MessageVersionT & { type: "AutomodExecution" };
	// m().actions;
	// m().flagged_message_id;
	// m().matches;
	// m().rules;

	return (
		<SystemMessage
			{...props}
			icon={icSword}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.automod_execution",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}
