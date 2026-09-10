import { useNavigate } from "@solidjs/router";
import {
	type Attachment,
	type Channel as ChannelT,
	getTimestampFromUUID,
	type Message as MessageT,
	type Preferences,
	type UserWithRelationship,
} from "sdk";
import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { useApi, useChannels, useFlumes, useMessages, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import icGear from "@/assets/gear.png";
import { Components } from "@/atoms/Components.tsx";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown.tsx";
import { Time } from "@/atoms/Time";
import { EmbedView } from "@/components/shared/UrlEmbed";
import { Avatar } from "@/components/shared/User";
import { useOptionalChannel } from "@/contexts/channel";
import { useMenu } from "@/contexts/mod.tsx";
import { useModals } from "@/contexts/modal";
import { useReadTracking } from "@/contexts/read-tracking.tsx";
import { colors } from "@/lib/colors.ts";
import { countEmojiOnly } from "@/lib/markdown";
import { MediaView } from "@/media/Media.tsx";
import { getMediaIcon } from "@/media/util.tsx";
import { openThread } from "@/utils/channel";
import { icInfo, icSword } from "@/utils/icons.ts";
import { UserDisplayName } from "../../shared/User.tsx";
import { MessageEditor } from "./MessageEditor.tsx";
import { useMessageToolbar } from "./message-toolbar-context.tsx";
import { Reactions } from "./Reactions.tsx";
import {
	SystemMessageAutomodExecution,
	type SystemMessageBaseProps,
	SystemMessageCall,
	SystemMessageChannelIcon,
	SystemMessageChannelMoved,
	SystemMessageChannelPingback,
	SystemMessageChannelRename,
	SystemMessageMemberAdd,
	SystemMessageMemberJoin,
	SystemMessageMemberRemove,
	SystemMessagePinned,
	SystemMessageThreadCreated,
} from "./SystemMessage.tsx";
import { asMarkdown2, isMarkdown, isMarkdown2 } from "./util.ts";

// TEMP: compat
export { UserDisplayName } from "../../shared/User.tsx";
export { isMarkdown } from "./util.ts";

export type MessageProps = {
	message: MessageT;
	separate?: boolean;
	diff?: boolean;
};

export function MessageTextMarkdown(props: {
	message: MessageT;
	diff?: boolean;
}) {
	const [, modalctl] = useModals();
	const viewHistory = () => {
		modalctl.open({
			type: "message_edits",
			message_id: props.message.id,
			channel_id: props.message.channel_id,
		});
	};

	const content = createMemo(() => {
		const v = asMarkdown2(props.message.latest_version);
		if (v) {
			return v.content ?? "";
		} else {
			return "";
		}
	});

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
		if (!r) return "*loading...*";

		const v = r.latest_version;
		switch (v.type) {
			case "DefaultMarkdown":
			case "ThreadInitial": {
				if (v.content) return v.content;
				if (v.attachments.length)
					return `${v.attachments.length} attachment(s)`;
				// TODO: handle embeds, components only messages
				// TODO: apply different style for messages without content
				return "*TODO: more robust renderer*";
			}

			// TODO: handle more message types in replies
			default: {
				return `${v.type} message`;
			}
		}
	};

	const icon = () => {
		const r = reply();
		if (!r) return;

		const v = r.latest_version;
		if (isMarkdown2(v)) {
			if (v.attachments.length) {
				// NOTE: maybe theres a better way to get an icon than only using the first attacment?
				return getMediaIcon(v.attachments[0].media);
			}
			return;
		}
	};

	const scrollToReply = () => {
		if (!chUpdate) return;
		chUpdate("reply_jump_source", props.source_id);
		ch.timelineState.controller.jumpToMessage(props.reply_id, true, true);
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
								<Show when={icon()}>{(icon) => <Icon src={icon()} />}</Show>
								<Show when={content()}>
									{(c) => (
										<Markdown
											content={c()}
											channel_id={props.channel_id}
											inline
										/>
									)}
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
	return (
		<Switch>
			<Match when={props.att.type === "Media" && props.att.media}>
				{(media) => <MediaView media={media()} attachment={props.att} />}
			</Match>
		</Switch>
	);
}

// TODO: rename to Message
export function MessageView(props: MessageProps) {
	const channels = useChannels();
	const messagesService = useMessages();
	const ctx = useCtx();
	const { menu } = useMenu();
	const thread = channels.use(() => props.message.channel_id);
	const [ch, chUpdate] = useOptionalChannel();
	const readTracking = useReadTracking();
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
		const ackId = prevMessage?.id ?? message_id;
		readTracking.ack(props.message.channel_id, ackId, true, false);
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
			<Match when={isMarkdown(props.message.latest_version.type)}>
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
	props: SystemMessageBaseProps & {
		user: UserWithRelationship | undefined;
		hovered: boolean;
		isEditing: boolean;
		channels2: ReturnType<typeof useChannels>;
		ctx: ReturnType<typeof useCtx>;
		diff?: boolean;
	},
) {
	const api = useApi();
	const flumes = useFlumes();
	const toolbar = useMessageToolbar();
	const version = () => asMarkdown2(props.message.latest_version);
	const flume = () =>
		props.message.flume?.state === "Live" && flumes.get(props.message.id);

	const dismissMessage = () => {
		api.messages.handleMessageDelete(
			props.message.channel_id,
			props.message.id,
		);
	};

	const isMediaEmbed = () => {
		const v = version();
		if (!v) return false;
		const e = v.embeds[0];
		if (!e) return false;
		return e.type === "Media" && v.content === e.url;
	};

	const thread = () =>
		props.message.latest_version.type !== "ThreadInitial" &&
		props.message.thread;

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
				<Show when={thread()}>
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

				<Show when={!isMediaEmbed()}>
					<Show
						when={!props.isEditing}
						fallback={<MessageEditor message={props.message} />}
					>
						<MessageTextMarkdown message={props.message} diff={props.diff} />
					</Show>
				</Show>
				<Switch>
					<Match when={props.message.automodded}>
						{(am) => (
							<div class="automodded">
								<Icon src={icSword} />
								<div>
									<div>
										<span class="automodded-blocked">Blocked by automod: </span>
										<span class="automodded-message">{am().message}</span>
									</div>
									<div class="automodded-ephemeral">
										<span>This message may be viewed by moderators</span>
										<span class="bull"> &bull; </span>
										<button
											class="button link dismiss"
											onClick={dismissMessage}
										>
											dismiss message
										</button>
									</div>
								</div>
							</div>
						)}
					</Match>
					<Match when={props.message.ephemeral}>
						<div class="ephemeral">
							<Icon src={icInfo} />
							<span>only you can see this</span>
							<span class="bull">&bull;</span>
							<button class="button link dismiss" onClick={dismissMessage}>
								dismiss message
							</button>
						</div>
					</Match>
				</Switch>
			</div>

			<div class="accessories">
				{/* attachments */}
				<Show when={version()?.attachments?.length}>
					<ul class="attachments">
						<For each={version()?.attachments}>
							{(att) => (
								<li>
									<AttachmentView att={att} />
								</li>
							)}
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
						<Components components={f().components} message={props.message} />
					)}
				</Show>

				{/* components */}
				<Show when={version()?.components?.length && !flume()}>
					<Components
						components={version()?.components ?? []}
						message={props.message}
					/>
				</Show>

				{/* reactions */}
				<Show
					when={props.message.reactions && props.message.reactions.length > 0}
				>
					<Reactions message={props.message} />
				</Show>

				{/* thread */}
				{/* TODO: should i not include thread for ThreadInitial messages at all? */}
				<Show when={thread()}>
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
