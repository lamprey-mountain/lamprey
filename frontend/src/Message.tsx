import { getTimestampFromUUID, type Message, type Thread, User } from "sdk";
import { type MessageT, MessageType } from "./types.ts";
import {
	createEffect,
	createSignal,
	For,
	Match,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { marked } from "marked";
import sanitizeHtml from "sanitize-html";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "./media/mod.tsx";
import { flags } from "./flags.ts";
import { type MediaProps } from "./media/util.tsx";
import { Time } from "./Time.tsx";
import { createTooltip, tooltip } from "./Tooltip.tsx";
import { Avatar, UserView } from "./User.tsx";
import { EmbedView } from "./UrlEmbed.tsx";
import { createEditor } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";

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

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	// transformTags: {
	// 	del: "s",
	// },
	allowedTags: sanitizeHtml.defaults.allowedTags.concat(["ins", "del"]),
};

export const md = marked.use({
	breaks: true,
	gfm: true,
});

const contentToHtml = new WeakMap();

function MessageTextMarkdown(props: MessageTextMarkdownProps) {
	function getHtml(): string {
		const cached = contentToHtml.get(props.message);
		if (cached) return cached;
		// console.count("render_html");
		const html = sanitizeHtml(
			md.parse(props.message.content ?? "") as string,
			sanitizeHtmlOptions,
		).trim();
		contentToHtml.set(props.message, html);
		return html;
	}

	let highlightEl: HTMLDivElement;
	function highlight() {
		getHtml();
		import("highlight.js").then(({ default: hljs }) => {
			// HACK: retain line numbers
			// FIXME: use language if provided instead of guessing
			for (const el of [...highlightEl!.querySelectorAll("pre")]) {
				el.dataset.highlighted = "";
				hljs.highlightElement(el);
			}
		});
	}

	createEffect(highlight);

	const ctx = useCtx();
	const viewHistory = () => {
		ctx.dispatch({
			do: "modal.open",
			modal: {
				type: "message_edits",
				message_id: props.message.id,
				thread_id: props.message.thread_id,
			},
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

	const [draft, setDraft] = createSignal(
		ctx.thread_edit_drafts.get(props.message.id) ?? props.message.content ??
			"",
	);

	const editor = createEditor({
		initialContent: draft(),
		initialSelection: ctx.editingMessage.get(props.message.thread_id)
			?.selection,
		keymap: {
			ArrowUp: (state) => {
				if (state.selection.from !== 1) return false;

				const ranges = api.messages.cacheRanges.get(props.message.thread_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex - 1; i >= 0; i--) {
					const msg = messages[i];
					if (msg.type === "DefaultMarkdown") {
						ctx.editingMessage.set(props.message.thread_id, {
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

				const ranges = api.messages.cacheRanges.get(props.message.thread_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex((m) =>
					m.id === props.message.id
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex + 1; i < messages.length; i++) {
					const msg = messages[i];
					if (msg.type === "DefaultMarkdown") {
						ctx.editingMessage.set(props.message.thread_id, {
							message_id: msg.id,
							selection: "start",
						});
						return true;
					}
				}

				// No next message, focus main input
				ctx.editingMessage.delete(props.message.thread_id);
				ctx.thread_input_focus.get(props.message.thread_id)?.();
				return true;
			},
		},
	});

	const save = async (content: string) => {
		if (content.trim() === (props.message.content ?? "").trim()) {
			ctx.editingMessage.delete(props.message.thread_id);
			return;
		}
		if (content.trim().length === 0) {
			ctx.editingMessage.delete(props.message.thread_id);
			return;
		}
		try {
			await api.messages.edit(
				props.message.thread_id,
				props.message.id,
				content,
			);
		} catch (e) {
			console.error("failed to edit message", e);
		}
		ctx.editingMessage.delete(props.message.thread_id);
	};

	const cancel = () => {
		ctx.editingMessage.delete(props.message.thread_id);
		ctx.thread_input_focus.get(props.message.thread_id)?.();
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
					ctx.thread_edit_drafts.set(props.message.id, text);
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
	const thread = api.threads.fetch(() => props.message.thread_id);

	const inSelectMode = () =>
		ctx.selectMode.get(props.message.thread_id) ?? false;

	const handleClick = (e: MouseEvent) => {
		if (!inSelectMode()) return;
		e.preventDefault();
		e.stopPropagation();

		const thread_id = props.message.thread_id;
		const message_id = props.message.id;
		const selected = ctx.selectedMessages.get(thread_id) ?? [];

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
				ctx.selectedMessages.set(thread_id, newSelected);
			}
		} else {
			if (selected.includes(message_id)) {
				ctx.selectedMessages.set(
					thread_id,
					selected.filter((id) => id !== message_id),
				);
			} else {
				ctx.selectedMessages.set(thread_id, [...selected, message_id]);
			}
		}
	};

	function reactionAdd(key: string) {
		api.client.http.PUT(
			"/api/v1/thread/{thread_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id: props.message.id,
						thread_id: props.message.thread_id,
					},
				},
			},
		);
	}

	function reactionDel(key: string) {
		api.client.http.DELETE(
			"/api/v1/thread/{thread_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id: props.message.id,
						thread_id: props.message.thread_id,
					},
				},
			},
		);
	}

	function getComponent() {
		const date = new Date(
			props.message.edited_at ?? props.message.created_at ??
				new Date().toString(),
		);
		// TODO: replace emoji with actual icons
		// FIXME: spacing between MessageDefault and oneline is missing
		if (props.message.type === "MemberAdd") {
			return (
				<article
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<div class="emojiicon">&#x1f465;</div>
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author menu-user"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>
							{" added "}
							<span
								class="author menu-user"
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
				</article>
			);
		} else if (props.message.type === "MemberRemove") {
			return (
				<article
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<div class="emojiicon">&#x1f465;</div>
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author menu-user"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>
							{" removed "}
							<span
								class="author menu-user"
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
				</article>
			);
		} else if (props.message.type === "MemberJoin") {
			return (
				<article
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<div class="emojiicon">&#x1F44B;</div>
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author menu-user"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							joined the room
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
				</article>
			);
		} else if (props.message.type === "MessagePinned") {
			return (
				<article
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<div class="emojiicon">&#x1F4CC;</div>
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author menu-user"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							pinned a message
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
				</article>
			);
		} else if (props.message.type === "ThreadRename") {
			return (
				<article
					class="message menu-message oneline"
					data-message-id={props.message.id}
					classList={{
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<div class="emojiicon">&#x270F;&#xFE0F;</div>
					<div class="content">
						<div
							class="body markdown"
							classList={{ local: props.message.is_local }}
						>
							<span
								class="author menu-user"
								data-user-id={props.message.author_id}
							>
								<Author message={props.message} thread={thread()} />
							</span>{" "}
							renamed the thread to <b>{props.message.name_new}</b>
						</div>
					</div>
					<Time date={date} animGroup="message-ts" />
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
			const isEditing = () => {
				return ctx.editingMessage.get(props.message.thread_id)?.message_id ===
					props.message.id;
			};
			const withAvatar = ctx.userConfig().frontend["message_pfps"] === "yes";

			// TODO: this code is getting messy and needs a refactor soon...
			return (
				<article
					class="message menu-message"
					data-message-id={props.message.id}
					classList={{
						withavatar: withAvatar,
						separate: props.separate,
						notseparate: !props.separate,
					}}
					onClick={handleClick}
				>
					<Show when={props.message.reply_id}>
						<ReplyView
							thread_id={props.message.thread_id}
							reply_id={props.message.reply_id!}
							arrow_width={arrow_width()}
						/>
					</Show>
					<Show when={withAvatar}>
						<Show when={props.separate}>
							<Avatar user={user()} />
							<div
								class="author menu-user"
								classList={{ "override-name": !!props.message.override_name }}
								data-user-id={props.message.author_id}
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
								<ul class="reactions">
									<For each={props.message.reactions}>
										{(r) => (
											<li
												classList={{ me: r.self }}
												onClick={() =>
													r.self ? reactionAdd(r.key) : reactionDel(r.key)}
											>
												<span class="emoji">{r.key.toString()}</span>
												<span class="count">{r.count}</span>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</div>
					</Show>
					<Show when={!withAvatar}>
						<div class="author-wrap">
							<div
								class="author sticky menu-user"
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
								<ul class="reactions">
									<For each={props.message.reactions}>
										{(r) => (
											<li
												classList={{ me: r.self }}
												onClick={() =>
													r.self ? reactionAdd(r.key) : reactionDel(r.key)}
											>
												<span class="emoji">{r.key.toString()}</span>
												<span class="count">{r.count}</span>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</div>
						<Time date={date} animGroup="message-ts" />
					</Show>
				</article>
			);
		} else {
			return (
				<article
					class="message menu-message"
					data-message-id={props.message.id}
					onClick={handleClick}
				>
					unknown message: {props.message.type}
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
	const thread = api.threads.fetch(() => props.thread_id);

	const content = () => {
		const r = reply();
		if (!r) return;
		return ("content" in r && r.content) ??
			(("attachments" in r && r.attachments)
				? `${r.attachments.length} attachment(s)`
				: undefined);
	};

	const scrollToReply = () => {
		// if (!props.reply) return;
		ctx.thread_anchor.set(props.thread_id, {
			type: "context",
			limit: 50, // TODO: calc dynamically
			message_id: props.reply_id,
		});
		ctx.thread_highlight.set(props.thread_id, props.reply_id);
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
				<div class="content" onClick={scrollToReply}>
					<Show when={!reply.loading} fallback="loading...">
						<Show
							when={reply() && thread()}
							fallback={<span class="author"></span>}
						>
							<Author message={reply()!} thread={thread()!} />
						</Show>
						{content()}
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

function Author(props: { message: Message; thread?: Thread }) {
	const api = useApi();
	const room_member = props.thread?.room_id
		? api.room_members.fetch(
			() => props.thread!.room_id!,
			() => props.message.author_id,
		)
		: () => null;
	const thread_member = api.thread_members.fetch(
		() => props.message.thread_id,
		() => props.message.author_id,
	);
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

	const { content } = createTooltip({
		// animGroup: "users",
		placement: "right-start",
		interactive: true,
		tip: () => (
			<UserView
				user={user()}
				room_member={room_member()}
				thread_member={thread_member()}
			/>
		),
	});

	return (
		<span
			class="user"
			classList={{ "override-name": !!props.message.override_name }}
			data-user-id={props.message.author_id}
			use:content
		>
			{name()}
		</span>
	);
}

function Actor(props: { user_id: string; thread: Thread }) {
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
