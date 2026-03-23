import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import {
	Attachment,
	type Channel,
	getTimestampFromUUID,
	type Message,
	type Preferences,
	type ReactionKey,
	User,
} from "sdk";
import { type MessageT, MessageType, ThreadT } from "../../../types.ts";
import {
	createEffect,
	createSignal,
	For,
	type JSX,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { useApi, useChannels2, useMessages2 } from "@/api";
import { useCtx } from "../../../context.ts";
import { useMenu, useUserPopout } from "../../../contexts/mod.tsx";
import { useModals } from "../../../contexts/modal";
import { useNavigate } from "@solidjs/router";
import { useChannel } from "../../../channelctx.tsx";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "../../../media/mod.tsx";
import { flags } from "../../../flags.ts";
import { getEmojiUrl, type MediaProps } from "../../../media/util.tsx";
import { Time } from "../../../atoms/Time";
import { Avatar, UserView } from "../../../User.tsx";
import { EmbedView } from "../../../UrlEmbed.tsx";
import { createEditor } from "../editor/Editor.tsx";
import { serializeToMarkdown } from "../editor/serializer.ts";
import { uuidv7 } from "uuidv7";
import { Reactions } from "./Reactions.tsx";
import icReply from "../../../assets/reply.png";
import icReactionAdd from "../../../assets/reaction-add.png";
import icEdit from "../../../assets/edit.png";
import icMore from "../../../assets/more.png";
import icMemberAdd from "../../../assets/member-add.png";
import icMemberRemove from "../../../assets/member-remove.png";
import icMemberJoin from "../../../assets/member-join.png";
import icPin from "../../../assets/pin.png";
import icThread from "../../../assets/threads.png";
import { Markdown } from "../../../atoms/Markdown.tsx";
import { openThread } from "../../../utils/channel";
import type { SetStoreFunction } from "solid-js/store";
import type { ChannelState } from "../../../contexts/channel";
import { useConfig } from "../../../config.tsx";
import { useAppConfig } from "../../../hooks/useAppConfig.ts";

type MessageProps = {
	message: MessageT;
	separate?: boolean;
};

type MessageTextMarkdownProps = {
	message: MessageT;
};

function MessageTextMarkdown(props: MessageTextMarkdownProps) {
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
			? props.message.latest_version.content ?? ""
			: "";

	return (
		<Markdown
			content={content()}
			channel_id={props.message.channel_id}
			class="body"
			classList={{ local: props.message.is_local }}
		>
			<Show when={props.message.id !== props.message.latest_version.version_id}>
				<span class="edited" onClick={viewHistory}>(edited)</span>
			</Show>
		</Markdown>
	);
}

function MessageEditor(
	props: { message: MessageT },
) {
	const messagesService = useMessages2();
	const [ch, chUpdate] = useChannel() ?? [null, null];

	// TODO: save edit draft per message?
	const [draft, setDraft] = createSignal(
		props.message.latest_version.type === "DefaultMarkdown"
			? props.message.latest_version.content ?? ""
			: "",
	);

	if (!ch || !chUpdate) {
		return <div class="message-editor">Error: No channel context</div>;
	}

	const editor = createEditor({
		channelId: () => props.message.channel_id,
		roomId: () => props.message.room_id,
		initialContent: draft(),
		initialSelection: ch.editingMessage
			?.selection,
		keymap: {
			ArrowUp: (state) => {
				if (state.selection.from !== 1) return false;

				const ranges = messagesService._ranges.get(
					props.message.channel_id,
				);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
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

				const ranges = messagesService._ranges.get(
					props.message.channel_id,
				);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
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

				// No next message, focus main input
				chUpdate("editingMessage", undefined);
				ch.input_focus?.();
				return true;
			},
		},
	});

	const save = async (content: string) => {
		const oldContent = props.message.latest_version.type === "DefaultMarkdown"
			? props.message.latest_version.content ?? ""
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
				escape to <button onClick={cancel}>cancel</button> • enter to{" "}
				<button onClick={() => save(draft())}>save</button>
			</div>
		</div>
	);
}

// TODO: make thread reactive (store thread in cache on message fetch, read thread from cache)
export function MessageThread(
	props: {
		thread: ThreadT;
		parentChannel: Channel;
		preferences: Preferences;
	},
) {
	const nav = useNavigate();
	const [_chan, setChan] = useChannel()!;
	const channels = useChannels2();
	const ctx = useCtx();

	const openThreadClick = () => {
		openThread(
			props.thread,
			channels.get(props.thread.parent_id!)!,
			ctx.preferences(),
			setChan,
			nav,
		);
	};

	const lastActivityAt = () =>
		getTimestampFromUUID(props.thread.last_version_id ?? props.thread.id);

	return (
		<div class="message-thread">
			<div class="spine"></div>
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

export function MessageView(props: MessageProps) {
	const api = useApi();
	const channels2 = useChannels2();
	const messagesService = useMessages2();
	const ctx = useCtx();
	const { menu } = useMenu();
	const { userView, setUserView } = useUserPopout();
	const { t } = ctx;
	const thread = channels2.use(() => props.message.channel_id);
	const [ch, chUpdate] = useChannel() ?? [null, null];
	let messageArticleRef: HTMLElement | undefined;
	const [hovered, setHovered] = createSignal(false);

	const isMenuOpen = () => {
		const m = menu();
		if (!m) return false;
		return m.type === "message" && m.message_id === props.message.id;
	};

	const isReactionPickerOpen = () => {
		const popout = ctx.popout();
		if (
			!popout || !("id" in popout) || popout.id !== "emoji" || !popout.ref ||
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
		const messages = messagesService._ranges.get(thread_id)?.live.items ??
			[];
		const currentIndex = messages.findIndex((m) => m.id === message_id);

		if (currentIndex === -1) return;

		// set read marker to the *previous* message (making current message unread)
		const prevMessage = messages[currentIndex - 1];
		if (prevMessage) {
			chUpdate("read_marker_id", prevMessage.id);
		} else {
			// TODO: theres probably a better way to handle this than clearing the read marker
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
			const messages = messagesService._ranges.get(thread_id)?.live.items ??
				[];
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

	function SystemMessage(props2: {
		icon: string;
		content: JSX.Element;
		date: Date;
		class?: string;
	}) {
		return (
			<article
				ref={messageArticleRef!}
				class={`message menu-message oneline ${props2.class ?? ""}`}
				data-message-id={props.message.id}
				classList={{
					separate: props.separate,
					notseparate: !props.separate,
					"toolbar-visible": toolbarVisible(),
				}}
				onClick={handleClick}
				onMouseDown={(e) => {
					onMouseDown(e);
					handleAltClick(e);
				}}
				onMouseEnter={() => setHovered(true)}
				onMouseLeave={() => setHovered(false)}
			>
				<img class="icon main" src={props2.icon} />
				<div class="content">{props2.content}</div>
				<Time date={props2.date} animGroup="message-ts" />
				<MessageToolbar message={props.message} />
			</article>
		);
	}

	function getComponent() {
		const date = new Date(
			props.message.latest_version.created_at ?? props.message.created_at ??
				new Date().toString(),
		);
		// FIXME: spacing between MessageDefault and oneline is missing
		if (props.message.latest_version.type === "MemberAdd") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			const target = (
				<span
					class="author"
					data-user-id={props.message.latest_version.target_user_id}
				>
					<Show when={thread()}>
						<Actor
							user_id={props.message.latest_version.target_user_id}
							thread={thread()!}
						/>
					</Show>
				</span>
			);
			return (
				<SystemMessage
					icon={icMemberAdd}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.member_add", author, target)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "MemberRemove") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			const target = (
				<span
					class="author"
					data-user-id={props.message.latest_version.target_user_id}
				>
					<Show when={thread()}>
						<Actor
							user_id={props.message.latest_version.target_user_id}
							thread={thread()!}
						/>
					</Show>
				</span>
			);
			return (
				<SystemMessage
					icon={icMemberRemove}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.member_remove", author, target)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "MemberJoin") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			return (
				<SystemMessage
					icon={icMemberJoin}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.member_join", author)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "MessagePinned") {
			const navigate = useNavigate();
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);

			const version = props.message.latest_version;
			const link = (text: string) => (
				<button
					style="color: oklch(var(--color-fg1))"
					class="link"
					onClick={(e) => {
						e.stopPropagation();
						navigate(
							`/channel/${props.message.channel_id}/message/${version.pinned_message_id}`,
						);
					}}
				>
					{text}
				</button>
			);

			return (
				<SystemMessage
					icon={icPin}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.message_pinned", author, link)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "ChannelRename") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			const name_new = <b>{props.message.latest_version.name_new}</b>;
			return (
				<SystemMessage
					icon={icEdit}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.channel_rename", author, name_new)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "Call") {
			// TODO: say "you missed a call" in dm channels
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			const count = props.message.latest_version.participants.length;
			return (
				<SystemMessage
					icon={icMemberJoin}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{props.message.latest_version.ended_at
								? t("message_content.call_ended", author, count)
								: t("message_content.call_started", author, count)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "ChannelPingback") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			return (
				<SystemMessage
					icon={icReply}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.channel_pingback", author)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "ChannelIcon") {
			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);
			return (
				<SystemMessage
					icon={icEdit}
					date={date}
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.channel_icon", author)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "ThreadCreated") {
			const navigate = useNavigate();
			const ctx = useCtx();
			const { t } = ctx;
			const threadId = () =>
				props.message.latest_version.type === "ThreadCreated"
					? props.message.latest_version.thread_id
					: undefined;

			const author = (
				<span
					class="author"
					data-user-id={props.message.author_id}
				>
					<Author message={props.message} thread={thread()} />
				</span>
			);

			const link = (text: string) => (
				<Show
					when={threadId()}
					fallback={<span>{text}</span>}
				>
					<button
						class="link"
						onClick={(e) => {
							e.stopPropagation();
							if (threadId()) {
								navigate(`/thread/${threadId()}`);
							}
						}}
					>
						{text}
					</button>
				</Show>
			);

			const viewAll = (text: string) => (
				<button
					class="link"
					onClick={(e) => {
						e.stopPropagation();
						const ref = e.currentTarget;
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
					icon={icThread}
					date={date}
					class="message-dim-content"
					content={
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							{/* @ts-ignore */}
							{t("message_content.thread_created", author, link, viewAll)}
						</div>
					}
				/>
			);
		} else if (props.message.latest_version.type === "DefaultMarkdown") {
			const [arrow_width, set_arrow_width] = createSignal(0);
			const user = api.users.fetch(() => props.message.author_id);
			const set_w = (e: HTMLElement) => {
				onMount(() => {
					set_arrow_width(
						e.querySelector(".user")!.getBoundingClientRect().width,
					);
				});
			};
			const ctx = useCtx();
			const [ch] = useChannel() ?? [null];
			const isEditing = () => {
				return ch?.editingMessage?.message_id ===
					props.message.id;
			};
			const messageStyle = ctx.preferences().frontend["message_style"] ||
				"cozy";
			const withAvatar = messageStyle === "cozy";

			// TODO: this code is getting messy and needs a refactor soon...
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message"
					data-message-id={props.message.id}
					classList={{
						withavatar: withAvatar,
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
					onMouseDown={(e) => {
						onMouseDown(e);
						handleAltClick(e);
					}}
					onMouseEnter={() => setHovered(true)}
					onMouseLeave={() => setHovered(false)}
				>
					<Show when={props.message.latest_version.reply_id}>
						<ReplyView
							thread_id={props.message.channel_id}
							reply_id={props.message.latest_version.reply_id!}
							arrow_width={arrow_width()}
							source_id={props.message.id}
						/>
					</Show>
					<Show when={withAvatar}>
						<Show when={props.separate}>
							<div
								class="avatar-wrap menu-user"
								data-user-id={props.message.author_id}
								onClick={(e) => {
									e.stopPropagation();
									const currentTarget = e.currentTarget as HTMLElement;
									if (userView()?.ref === currentTarget) {
										setUserView(null);
									} else {
										setUserView({
											user_id: props.message.author_id,
											room_id: (thread() as any)?.room_id,
											thread_id: props.message.channel_id,
											ref: currentTarget,
											source: "message",
										});
									}
								}}
							>
								<Avatar user={user()} animate={hovered()} />
							</div>
							<div class="author">
								<Author message={props.message} thread={thread()} />
								<Time date={date} animGroup="message-ts" />
							</div>
						</Show>
						<Show when={!props.separate}>
							<div class="avatar"></div>
						</Show>
						<div class="content">
							<Show
								when={!isEditing()}
								fallback={<MessageEditor message={props.message} />}
							>
								<MessageTextMarkdown message={props.message} />
							</Show>
							<Show when={props.message.latest_version.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.latest_version.attachments}>
										{(att) => <AttachmentView att={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.latest_version.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.latest_version.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show
								when={props.message.reactions &&
									props.message.reactions.length > 0}
							>
								<Reactions message={props.message} />
							</Show>
							<Show when={props.message.thread}>
								{(thread) => <MessageThread thread={thread()} />}
							</Show>
						</div>
					</Show>
					<Show when={!withAvatar}>
						<div class="author-wrap">
							<div
								class="author sticky"
								ref={set_w}
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</div>
						</div>
						<div class="content">
							<Show
								when={!isEditing()}
								fallback={<MessageEditor message={props.message} />}
							>
								<MessageTextMarkdown message={props.message} />
							</Show>
							<Show when={props.message.latest_version.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.latest_version.attachments}>
										{(att) => <AttachmentView att={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.latest_version.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.latest_version.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show
								when={props.message.reactions &&
									props.message.reactions.length > 0}
							>
								<Reactions message={props.message} />
							</Show>
							<Show when={props.message.thread}>
								{(thread) => <MessageThread thread={thread()} />}
							</Show>
						</div>
						<Time date={date} animGroup="message-ts" />
					</Show>
					<MessageToolbar message={props.message} />
				</article>
			);
		} else {
			return (
				<article
					ref={messageArticleRef!}
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
					<MessageToolbar message={props.message} />
				</article>
			);
		}
	}

	return <>{getComponent()}</>;
}

type ReplyProps = {
	thread_id: string;
	reply_id: string;
	arrow_width?: number;
	source_id: string;
};

function ReplyView(props: ReplyProps) {
	const ctx = useCtx();
	const api = useApi();
	const channels2 = useChannels2();
	const messagesService = useMessages2();
	const { setUserView } = useUserPopout();
	const reply = messagesService.use(
		() => props.reply_id,
	);
	const thread = channels2.use(() => props.thread_id);
	const [ch, chUpdate] = useChannel() ?? [null, null];

	const content = () => {
		const r = reply();
		if (!r) return;
		return (r.latest_version.type === "DefaultMarkdown" &&
			r.latest_version.content) ??
			((r.latest_version.type === "DefaultMarkdown" &&
					r.latest_version.attachments)
				? `${r.latest_version.attachments.length} attachment(s)`
				: undefined);
	};

	const ReplyContent = () => {
		const r = reply();
		if (
			!r || r.latest_version.type !== "DefaultMarkdown" ||
			!r.latest_version.content
		) return <>{content()}</>;

		return (
			<Markdown
				content={r.latest_version.content}
				channel_id={props.thread_id}
				inline
			/>
		);
	};

	const scrollToReply = () => {
		// if (!props.reply) return;
		if (!chUpdate) return;
		chUpdate("reply_jump_source", props.source_id);
		chUpdate("anchor", {
			type: "context",
			limit: 50, // TODO: calc dynamically
			message_id: props.reply_id,
		});
		chUpdate("highlight", props.reply_id);
	};

	return (
		<>
			<div class="reply">
				<div class="arrow">
					<svg
						viewBox="0 0 100 100"
						preserveAspectRatio="none"
						style={{ width: props.arrow_width ? `${props.arrow_width}px` : "" }}
					>
						<path
							vector-effect="non-scaling-stroke"
							shape-rendering="crispEdges"
							// M = move to x y
							// L = line to x y
							d="M 50 100 L 50 50 L 100 50"
						/>
					</svg>
				</div>
				<div class="content" style="display:flex" onClick={scrollToReply}>
					<Show when={!reply.loading} fallback="loading...">
						<Show
							when={reply() && thread()}
							fallback={<span class="author"></span>}
						>
							<Author message={reply()!} thread={thread()!} />
						</Show>
						{(() => {
							const r = reply();
							const version = r?.latest_version;
							return version?.type === "DefaultMarkdown" && version.content
								? <ReplyContent />
								: <>{content()}</>;
						})()}
					</Show>
				</div>
			</div>
		</>
	);
}

export function AttachmentView(
	props: { att: Attachment },
) {
	if (props.att.type !== "Media" || !props.att.media) return null;
	const b = () => props.att.media!.content_type.split("/")[0];
	if (b() === "image") {
		return (
			<li class="raw">
				<ImageView
					media={props.att.media!}
				/>
			</li>
		);
	} else if (b() === "video") {
		return (
			<li class="raw">
				<VideoView media={props.att.media!} />
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li class="raw">
				<AudioView media={props.att.media!} />
			</li>
		);
	} else if (
		b() === "text" ||
		/^application\/json\b/.test(props.att.media!.content_type)
	) {
		return (
			<li class="raw">
				<TextView media={props.att.media!} />
			</li>
		);
	} else {
		return (
			<li>
				<FileView media={props.att.media!} />
			</li>
		);
	}
}

export function Author(props: { message: Message; thread?: Channel }) {
	const api = useApi();
	const { userView, setUserView } = useUserPopout();
	const room_member = props.thread?.room_id
		? api.room_members.fetch(
			() => props.thread!.room_id!,
			() => props.message.author_id,
		)
		: () => null;
	const user = api.users.fetch(() => props.message.author_id);

	function name() {
		let name;
		const rm = room_member?.();
		if (rm) name ??= rm.override_name;

		const us = user();
		name ??= us?.name;

		return name;
	}

	return (
		<span
			class="user menu-user"
			data-user-id={props.message.author_id}
			onClick={(e) => {
				e.stopPropagation();
				const currentTarget = e.currentTarget as HTMLElement;
				if (userView()?.ref === currentTarget) {
					setUserView(null);
				} else {
					setUserView({
						user_id: props.message.author_id,
						room_id: (props.thread as any)?.room_id,
						thread_id: props.message.channel_id,
						ref: currentTarget,
						source: "message",
					});
				}
			}}
		>
			{name()}
		</span>
	);
}

function Actor(props: { user_id: string; thread: Channel }) {
	const api = useApi();
	const room_member = props.thread.room_id
		? api.room_members.fetch(
			() => props.thread.room_id!,
			() => props.user_id,
		)
		: () => null;
	const user = api.users.fetch(() => props.user_id);

	function name() {
		let name;

		const rm = room_member?.();
		if (rm) name ??= rm.override_name;

		const us = user();
		name ??= us?.name;

		return name;
	}

	return (
		<span class="user">
			{name()}
		</span>
	);
}

export const MessageToolbar = (props: { message: Message }) => {
	const ctx = useCtx();
	const { setMenu } = useMenu();
	const api = useApi();
	const messagesService = useMessages2();
	const [showReactionPicker, setShowReactionPicker] = createSignal(false);
	let reactionButtonRef: HTMLButtonElement | undefined;

	const areReactionKeysEqual = (a: ReactionKey, b: ReactionKey): boolean => {
		if (a.type !== b.type) return false;
		if (a.type === "Text" && b.type === "Text") return a.content === b.content;
		if (a.type === "Custom" && b.type === "Custom") return a.id === b.id;
		return false;
	};

	createEffect(() => {
		if (showReactionPicker()) {
			ctx.setPopout({
				id: "emoji",
				ref: reactionButtonRef,
				placement: "left-start",
				props: {
					selected: (emoji: string | null, keepOpen: boolean) => {
						if (emoji) {
							const existing = props.message.reactions?.find((r) =>
								areReactionKeysEqual(r.key, { type: "Text", content: emoji })
							);
							if (!existing || !existing.self) {
								api.reactions.add(
									props.message.channel_id,
									props.message.id,
									`t:${emoji}`,
								);
							}
						}
						if (!keepOpen) setShowReactionPicker(false);
					},
				},
			});
		} else {
			const popout = ctx.popout();
			if (
				popout && "id" in popout && popout.id === "emoji" &&
				popout.ref === reactionButtonRef
			) {
				ctx.setPopout(null);
			}
		}
	});

	const closePicker = (e: MouseEvent) => {
		const popoutEl = document.querySelector(".popout");
		if (
			reactionButtonRef && !reactionButtonRef.contains(e.target as Node) &&
			(!popoutEl || !popoutEl.contains(e.target as Node))
		) {
			setShowReactionPicker(false);
		}
	};

	createEffect(() => {
		if (showReactionPicker()) {
			document.addEventListener("click", closePicker);
		} else {
			document.removeEventListener("click", closePicker);
		}
		onCleanup(() => document.removeEventListener("click", closePicker));
	});

	const currentUser = useCurrentUser();
	const isOwnMessage = () => {
		return currentUser()?.id === props.message.author_id;
	};

	const canEditMessage = () => {
		return props.message.latest_version.type === "DefaultMarkdown" &&
			!props.message.is_local &&
			isOwnMessage();
	};

	const handleAddReaction = (e: MouseEvent) => {
		e.stopPropagation();
		setShowReactionPicker(!showReactionPicker());
	};

	const [ch, chUpdate] = useChannel() ?? [null, null];

	const handleReply = () => {
		if (!ch || !chUpdate) return;
		chUpdate("reply_id", props.message.id);
	};

	const handleEdit = () => {
		if (!canEditMessage() || !chUpdate) return;
		chUpdate("editingMessage", {
			message_id: props.message.id,
			selection: "end",
		});
	};

	const handleContextMenu = (e: MouseEvent) => {
		e.preventDefault();

		const button = e.currentTarget as HTMLButtonElement;
		const rect = button.getBoundingClientRect();

		queueMicrotask(() => {
			setMenu({
				x: rect.left,
				y: rect.bottom,
				type: "message",
				channel_id: props.message.channel_id,
				message_id: props.message.id,
				version_id: props.message.latest_version.version_id,
			});
		});
	};

	return (
		<div class="message-toolbar">
			<button
				ref={reactionButtonRef}
				onClick={handleAddReaction}
				title="Add reaction"
				aria-label="Add reaction"
			>
				<img class="icon" src={icReactionAdd} />
			</button>
			<button onClick={handleReply} title="Reply" aria-label="Reply">
				<img class="icon" src={icReply} />
			</button>
			<Show when={canEditMessage()}>
				<button onClick={handleEdit} title="Edit" aria-label="Edit">
					<img class="icon" src={icEdit} />
				</button>
			</Show>
			<button
				onClick={handleContextMenu}
				title="More options"
				aria-label="More options"
			>
				<img class="icon" src={icMore} />
			</button>
		</div>
	);
};
