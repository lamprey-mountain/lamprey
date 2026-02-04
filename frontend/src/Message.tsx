import { type Channel, getTimestampFromUUID, type Message, User } from "sdk";
import { type MessageT, MessageType } from "./types.ts";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { useModals } from "./contexts/modal";
import { useNavigate } from "@solidjs/router";
import { useChannel } from "./channelctx.tsx";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "./media/mod.tsx";
import { flags } from "./flags.ts";
import { getEmojiUrl, type MediaProps } from "./media/util.tsx";
import { Time } from "./Time.tsx";
import { Avatar, UserView } from "./User.tsx";
import { EmbedView } from "./UrlEmbed.tsx";
import { createEditor } from "./Editor.tsx";
import { render } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import twemoji from "twemoji";
import { Reactions } from "./Reactions.tsx";
import { md } from "./markdown.tsx";
import icReply from "./assets/reply.png";
import icReactionAdd from "./assets/reaction-add.png";
import icEdit from "./assets/edit.png";
import icMore from "./assets/more.png";
import icMemberAdd from "./assets/member-add.png";
import icMemberRemove from "./assets/member-remove.png";
import icMemberJoin from "./assets/member-join.png";
import icPin from "./assets/pin.png";

type MessageProps = {
	message: MessageT;
	separate?: boolean;
};

type MessageTextTaggedProps = {
	message: MessageT & { type: "DefaultTagged" };
};

type MessageTextMarkdownProps = {
	message: MessageT & { type: "DefaultMarkdown" };
};

const contentToHtml = new WeakMap();

function UserMention(props: { id: string; channel: Channel }) {
	const api = useApi();
	const ctx = useCtx();
	const user = api.users.fetch(() => props.id);
	// FIXME: handle mentions outside of rooms (eg. in dms)
	const room_member = api.room_members.fetch(
		() => props.channel.room_id!,
		() => props.id,
	);
	return (
		<span
			class="mention-user"
			onClick={(e) => {
				e.stopPropagation();
				const currentTarget = e.currentTarget as HTMLElement;
				if (ctx.userView()?.ref === currentTarget) {
					ctx.setUserView(null);
				} else {
					ctx.setUserView({
						user_id: props.id,
						room_id: props.channel.room_id,
						thread_id: props.channel.id,
						ref: currentTarget,
						source: "message",
					});
				}
			}}
		>
			@{room_member()?.override_name ?? user()?.name ?? "unknown user"}
		</span>
	);
}

function RoleMention(props: { id: string; thread: Channel }) {
	const api = useApi();
	const [role] = createResource(() => props.thread.room_id, async (room_id) => {
		if (!room_id) return null;
		const roles = api.roles.list(() => room_id)();
		return roles?.items.find((r) => r.id === props.id) ?? null;
	});
	return <span class="mention-role">@{role()?.name ?? "..."}</span>;
}

function ChannelMention(props: { id: string }) {
	const api = useApi();
	const navigate = useNavigate();
	const channel = api.channels.fetch(() => props.id);
	return (
		<span
			class="mention-channel"
			onClick={(e) => {
				e.stopPropagation();
				navigate(`/channel/${props.id}`);
			}}
		>
			#{channel()?.name ?? "unknown channel"}
		</span>
	);
}

function Emoji(props: { id: string; name: string; animated: boolean }) {
	const api = useApi();
	// const emoji = api.emoji.fetch(() => props.id);
	const url = () => {
		return getEmojiUrl(props.id);
	};
	return (
		<img
			class="emoji"
			src={url()}
			alt={`:${props.name}:`}
			title={`:${props.name}:`}
		/>
	);
}

// HACK: this is terrible and extremely cursed and ugly and horrible code and i hate it
// i should NOT need to break out of solidjs/tsx reactivity then manually recreate it with another render
function hydrateMentions(el: HTMLElement, thread: Channel) {
	el.querySelectorAll<HTMLSpanElement>("span.mention[data-mention-type]")
		.forEach((mentionEl) => {
			const type = mentionEl.dataset.mentionType;
			if (type === "user") {
				const userId = mentionEl.dataset.userId!;
				render(() => <UserMention channel={thread} id={userId} />, mentionEl);
			} else if (type === "role") {
				const roleId = mentionEl.dataset.roleId!;
				render(() => <RoleMention id={roleId} thread={thread} />, mentionEl);
			} else if (type === "channel") {
				const channelId = mentionEl.dataset.channelId!;
				render(() => <ChannelMention id={channelId} />, mentionEl);
			} else if (type === "emoji") {
				const emojiId = mentionEl.dataset.emojiId!;
				const emojiName = mentionEl.dataset.emojiName!;
				const emojiAnimated = mentionEl.dataset.emojiAnimated === "true";
				render(
					() => (
						<Emoji id={emojiId} name={emojiName} animated={emojiAnimated} />
					),
					mentionEl,
				);
			}
		});
}

function MessageTextMarkdown(props: MessageTextMarkdownProps) {
	function getHtml(): string {
		const cached = contentToHtml.get(props.message);
		if (cached) return cached;

		const content = props.message.content ?? "";

		function escape(html: string) {
			return html.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(
				/>/g,
				"&gt;",
			).replace(/"/g, "&quot;").replace(/'/g, "&#39;");
		}

		const tokens = md.lexer(content);
		md.walkTokens(tokens, (token) => {
			if (token.type === "html") {
				(token as any).text = escape((token as any).text);
			}
		});

		const html = (md.parser(tokens) as string).trim();

		const twemojified = twemoji.parse(html, {
			base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
			folder: "svg",
			ext: ".svg",
		});
		contentToHtml.set(props.message, twemojified);
		return twemojified;
	}

	let highlightEl!: HTMLDivElement;
	const thread = useApi().channels.fetch(() => props.message.channel_id);
	function highlight() {
		getHtml();
		import("highlight.js").then(({ default: hljs }) => {
			// HACK: retain line numbers
			// FIXME: use language if provided instead of guessing
			for (const el of [...highlightEl.querySelectorAll("pre")]) {
				el.dataset.highlighted = "";
				hljs.highlightElement(el);
			}
		});
	}

	createEffect(highlight);
	createEffect(() => {
		const t = thread();
		if (t && highlightEl) {
			hydrateMentions(highlightEl, t);
		}
	});

	const [, modalctl] = useModals();
	const viewHistory = () => {
		modalctl.open({
			type: "message_edits",
			message_id: props.message.id,
			channel_id: props.message.channel_id,
		});
	};

	return (
		<div
			class="body markdown"
			classList={{ local: props.message.is_local }}
			ref={highlightEl!}
		>
			<span innerHTML={getHtml()}></span>
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited" onClick={viewHistory}>(edited)</span>
			</Show>
		</div>
	);
}

function MessageEditor(
	props: { message: MessageT & { type: "DefaultMarkdown" } },
) {
	const ctx = useCtx();
	const api = useApi();
	const [ch, chUpdate] = useChannel()!;

	// TODO: save edit draft per message?
	const [draft, setDraft] = createSignal(props.message.content ?? "");

	const editor = createEditor({
		initialContent: draft(),
		initialSelection: ch.editingMessage
			?.selection,
		keymap: {
			ArrowUp: (state) => {
				if (state.selection.from !== 1) return false;

				const ranges = api.messages.cacheRanges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex - 1; i >= 0; i--) {
					const msg = messages[i];
					if (msg.type === "DefaultMarkdown") {
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

				const ranges = api.messages.cacheRanges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex + 1; i < messages.length; i++) {
					const msg = messages[i];
					if (msg.type === "DefaultMarkdown") {
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
		if (content.trim() === (props.message.content ?? "").trim()) {
			chUpdate("editingMessage", undefined);
			return;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return;
		}
		try {
			await api.messages.edit(
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
				onSubmit={save}
				onChange={(state) => {
					const text = state.doc.textContent;
					setDraft(text);
					chUpdate("edit_draft", text);
				}}
			/>
			<div class="edit-info dim">
				escape to <button onClick={cancel}>cancel</button> • enter to{" "}
				<button onClick={() => save(draft())}>save</button>
			</div>
		</div>
	);
}

export function MessageView(props: MessageProps) {
	const api = useApi();
	const ctx = useCtx();
	const thread = api.channels.fetch(() => props.message.channel_id);
	const [ch, chUpdate] = useChannel()!;
	let messageArticleRef: HTMLElement | undefined;

	const isMenuOpen = () => {
		const menu = ctx.menu();
		if (!menu) return false;
		return menu.type === "message" && menu.message_id === props.message.id;
	};

	const isReactionPickerOpen = () => {
		const popout = ctx.popout();
		if (popout.id !== "emoji" || !popout.ref || !messageArticleRef) {
			return false;
		}
		return messageArticleRef.contains(popout.ref as Node);
	};
	const toolbarVisible = () => isMenuOpen() || isReactionPickerOpen();

	const inSelectMode = () => ch.selectMode;

	const onMouseDown = (e: MouseEvent) => {
		if (inSelectMode() && e.shiftKey) {
			e.preventDefault();
		}
	};

	const handleClick = (e: MouseEvent) => {
		if (!inSelectMode()) return;
		e.preventDefault();
		e.stopPropagation();

		const thread_id = props.message.channel_id;
		const message_id = props.message.id;
		const selected = ch.selectedMessages;

		if (e.shiftKey && selected.length > 0) {
			const lastSelected = selected[selected.length - 1];
			const messages = api.messages.cacheRanges.get(thread_id)?.live.items ??
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

	function getComponent() {
		const date = new Date(
			props.message.edited_at ?? props.message.created_at ??
				new Date().toString(),
		);
		// FIXME: spacing between MessageDefault and oneline is missing
		if (props.message.type === "MemberAdd") {
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
					onMouseDown={onMouseDown}
				>
					<img class="icon main" src={icMemberAdd} />
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>
							{" added "}
							<span
								class="author"
								data-user-id={props.message.target_user_id}
							>
								<Show when={thread()}>
									<Actor
										user_id={props.message.target_user_id}
										thread={thread()!}
									/>
								</Show>
							</span>{" "}
							to the thread
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
					<MessageToolbar message={props.message} />
				</article>
			);
		} else if (props.message.type === "MemberRemove") {
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
				>
					<img class="icon main" src={icMemberRemove} />
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>
							{" removed "}
							<span
								class="author"
								data-user-id={props.message.target_user_id}
							>
								<Show when={thread()}>
									<Actor
										user_id={props.message.target_user_id}
										thread={thread()!}
									/>
								</Show>
							</span>{" "}
							from the thread
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
					<MessageToolbar message={props.message} />
				</article>
			);
		} else if (props.message.type === "MemberJoin") {
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
				>
					<img class="icon main" src={icMemberJoin} />
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							joined the room
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
					<MessageToolbar message={props.message} />
				</article>
			);
		} else if (props.message.type === "MessagePinned") {
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
				>
					<img class="icon main" src={icPin} />
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							pinned a message
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
					<MessageToolbar message={props.message} />
				</article>
			);
		} else if (props.message.type === "ThreadRename") {
			return (
				<article
					ref={messageArticleRef!}
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
						"toolbar-visible": toolbarVisible(),
					}}
					onClick={handleClick}
				>
					<img class="icon main" src={icEdit} />
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							renamed the thread to <b>{props.message.name_new}</b>
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
					<MessageToolbar message={props.message} />
				</article>
			);
		} else if (props.message.type === "DefaultMarkdown") {
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
			const [ch] = useChannel()!;
			const isEditing = () => {
				return ch.editingMessage?.message_id ===
					props.message.id;
			};
			const messageStyle = ctx.userConfig().frontend["message_style"] || "cozy";
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
					onMouseDown={onMouseDown}
				>
					<Show when={props.message.reply_id}>
						<ReplyView
							thread_id={props.message.channel_id}
							reply_id={props.message.reply_id!}
							arrow_width={arrow_width()}
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
									if (ctx.userView()?.ref === currentTarget) {
										ctx.setUserView(null);
									} else {
										ctx.setUserView({
											user_id: props.message.author_id,
											room_id: thread()?.room_id,
											thread_id: props.message.channel_id,
											ref: currentTarget,
											source: "message",
										});
									}
								}}
							>
								<Avatar user={user()} />
							</div>
							<div
								class="author"
								classList={{ "override-name": !!props.message.override_name }}
							>
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
							<Show when={props.message.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.attachments}>
										{(att) => <AttachmentView media={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.reactions?.length}>
								<Reactions message={props.message} />
							</Show>
						</div>
					</Show>
					<Show when={!withAvatar}>
						<div class="author-wrap">
							<div
								class="author sticky"
								classList={{ "override-name": !!props.message.override_name }}
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
							<Show when={props.message.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.attachments}>
										{(att) => <AttachmentView media={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.reactions?.length}>
								<Reactions message={props.message} />
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
				>
					unknown message: {props.message.type}
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
};

function ReplyView(props: ReplyProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply = api.messages.fetch(() => props.thread_id, () => props.reply_id);
	const thread = api.channels.fetch(() => props.thread_id);
	const [ch, chUpdate] = useChannel()!;

	const content = () => {
		const r = reply();
		if (!r) return;
		return ("content" in r && r.content) ??
			(("attachments" in r && r.attachments)
				? `${r.attachments.length} attachment(s)`
				: undefined);
	};

	// Function to render content with inline markdown and mentions
	const ReplyContent = () => {
		const r = reply();
		if (!r || !r.content) return <>{content()}</>;

		let contentStr = r.content;
		const m = r.mentions;

		for (const user of m?.users ?? []) {
			contentStr = contentStr.replace(
				new RegExp(`<@${user.id}>`, "g"),
				`<span class="mention-user" data-user-id="${user.id}">@${user.resolved_name}</span>`,
			);
		}
		for (const channel of m?.channels ?? []) {
			contentStr = contentStr.replace(
				new RegExp(`<#${channel.id}>`, "g"),
				`<span class="mention-channel" data-channel-id="${channel.id}">#${channel.name}</span>`,
			);
		}
		for (const role of m?.roles ?? []) {
			const roleData = thread()?.room_id
				? api.roles.fetch(() => thread()!.room_id, () => role.id)()
				: null;
			contentStr = contentStr.replace(
				new RegExp(`<@&${role.id}>`, "g"),
				`<span class="mention-role" data-role-id="${role.id}">@${
					roleData?.name || "..."
				}</span>`,
			);
		}
		for (const emoji of m?.emojis ?? []) {
			contentStr = contentStr.replace(
				new RegExp(`<a?:${emoji.name}:${emoji.id}>`, "g"),
				`<span class="mention-emoji">:${emoji.name}:</span>`,
			);
		}

		// Process the content with markdown, now that mentions are replaced with proper HTML
		const processedHTML = md(contentStr);

		// Create a ref to allow post-processing of the rendered content
		let contentRef: HTMLSpanElement | undefined;

		// Post-process to make mentions interactive
		createEffect(() => {
			if (!contentRef) return;

			// Add click handlers to make mentions interactive
			const userMentions = contentRef.querySelectorAll(
				".mention-user[data-user-id]",
			);
			userMentions.forEach((mention) => {
				const userId = mention.getAttribute("data-user-id");
				if (userId) {
					mention.addEventListener("click", (e) => {
						e.stopPropagation();
						// Open user profile modal or similar
						ctx.setUserView({
							user_id: userId,
							room_id: thread()?.room_id,
							thread_id: props.thread_id,
							ref: mention as HTMLElement,
							source: "reply",
						});
					});
				}
			});

			const channelMentions = contentRef.querySelectorAll(
				".mention-channel[data-channel-id]",
			);
			channelMentions.forEach((mention) => {
				const channelId = mention.getAttribute("data-channel-id");
				if (channelId) {
					mention.addEventListener("click", (e) => {
						e.stopPropagation();
						// Navigate to channel
						location.href = `/channel/${channelId}`;
					});
				}
			});
		});

		return <span ref={contentRef} innerHTML={processedHTML as string} />;
	};

	// Components for different types of mentions
	const UserMentionContent = (props: { userId: string }) => {
		const user = api.users.fetch(() => props.userId)();
		return (
			<span class="mention-user">
				@{user?.name || "..."}
			</span>
		);
	};

	const ChannelMentionContent = (props: { channelId: string }) => {
		const channel = api.channels.fetch(() => props.channelId)();
		return (
			<span class="mention-channel">
				#{channel?.name || "..."}
			</span>
		);
	};

	const RoleMentionContent = (props: { roleId: string; threadId: string }) => {
		const thread = api.channels.fetch(() => props.threadId)();
		const role = thread?.room_id
			? api.roles.fetch(() => thread.room_id, () => props.roleId)()
			: null;
		return (
			<span class="mention-role">
				@{role?.name || "..."}
			</span>
		);
	};

	const EmojiMentionContent = (
		props: { emojiName: string; emojiId: string },
	) => {
		return (
			<span class="mention-emoji">
				:{props.emojiName}:
			</span>
		);
	};

	const scrollToReply = () => {
		// if (!props.reply) return;
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
						{reply()?.content ? <ReplyContent /> : <>{content()}</>}
					</Show>
				</div>
			</div>
		</>
	);
}

export function AttachmentView(props: MediaProps) {
	const b = () => props.media.source.mime.split("/")[0];
	const ty = () => props.media.source.mime.split(";")[0];
	if (b() === "image") {
		return (
			<li class="raw">
				<ImageView
					media={props.media}
					thumb_height={props.size}
					thumb_width={props.size}
				/>
			</li>
		);
	} else if (b() === "video") {
		return (
			<li class="raw">
				<VideoView media={props.media} />
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li class="raw">
				<AudioView media={props.media} />
			</li>
		);
	} else if (
		b() === "text" || /^application\/json\b/.test(props.media.source.mime)
	) {
		return (
			<li class="raw">
				<TextView media={props.media} />
			</li>
		);
	} else {
		return (
			<li>
				<FileView media={props.media} />
			</li>
		);
	}
}

export function Author(props: { message: Message; thread?: Channel }) {
	const api = useApi();
	const ctx = useCtx();
	const room_member = props.thread?.room_id
		? api.room_members.fetch(
			() => props.thread!.room_id!,
			() => props.message.author_id,
		)
		: () => null;
	const user = api.users.fetch(() => props.message.author_id);

	function name() {
		let name = ("override_name" in props.message)
			? props.message.override_name
			: undefined;
		const rm = room_member?.();
		if (rm?.membership === "Join") name ??= rm.override_name;

		const us = user();
		name ??= us?.name;

		return name;
	}

	return (
		<span
			class="user menu-user"
			classList={{ "override-name": !!props.message.override_name }}
			data-user-id={props.message.author_id}
			onClick={(e) => {
				e.stopPropagation();
				const currentTarget = e.currentTarget as HTMLElement;
				if (ctx.userView()?.ref === currentTarget) {
					ctx.setUserView(null);
				} else {
					ctx.setUserView({
						user_id: props.message.author_id,
						room_id: props.thread?.room_id,
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
		if (rm?.membership === "Join") name ??= rm.override_name;

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

const MessageToolbar = (props: { message: Message }) => {
	const ctx = useCtx();
	const api = useApi();
	const [showReactionPicker, setShowReactionPicker] = createSignal(false);
	let reactionButtonRef: HTMLButtonElement | undefined;

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
								r.key === emoji
							);
							if (!existing || !existing.self) {
								api.reactions.add(
									props.message.channel_id,
									props.message.id,
									emoji,
								);
							}
						}
						if (!keepOpen) setShowReactionPicker(false);
					},
				},
			});
		} else {
			if (
				ctx.popout().id === "emoji" && ctx.popout().ref === reactionButtonRef
			) {
				ctx.setPopout({});
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

	const isOwnMessage = () => {
		const currentUser = api.users.cache.get("@self");
		return currentUser && currentUser.id === props.message.author_id;
	};

	const canEditMessage = () => {
		return props.message.type === "DefaultMarkdown" &&
			!props.message.is_local &&
			isOwnMessage();
	};

	const handleAddReaction = (e: MouseEvent) => {
		e.stopPropagation();
		setShowReactionPicker(!showReactionPicker());
	};

	const [ch, chUpdate] = useChannel()!;

	const handleReply = () => {
		chUpdate("reply_id", props.message.id);
	};

	const handleEdit = () => {
		if (canEditMessage()) {
			chUpdate("editingMessage", {
				message_id: props.message.id,
				selection: "end",
			});
		}
	};

	const handleContextMenu = (e: MouseEvent) => {
		e.preventDefault();

		const button = e.currentTarget as HTMLButtonElement;
		const rect = button.getBoundingClientRect();

		queueMicrotask(() => {
			ctx.setMenu({
				x: rect.left,
				y: rect.bottom,
				type: "message",
				channel_id: props.message.channel_id,
				message_id: props.message.id,
				version_id: props.message.version_id,
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
