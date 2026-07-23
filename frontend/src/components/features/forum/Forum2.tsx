import { useCurrentUser } from "@/contexts/currentUser";

// TODO: refactor out duplicated code from here and Message.tsx

import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { leading, throttle } from "@solid-primitives/scheduled";
import { ReactiveSet } from "@solid-primitives/set";
import type { EditorState, Transaction } from "prosemirror-state";
import type { Channel, Message, RepliesMessage, RoomMember } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import {
	useChannels,
	useMessages,
	usePreferences,
	useRoomMembers,
	useThreads,
	useUsers,
} from "@/api";
import cancelIc from "@/assets/x.png";
import { Dropdown } from "@/atoms/Dropdown";
import { EmojiButton } from "@/atoms/EmojiButton";
import { Icon } from "@/atoms/Icon";
import { Search } from "@/atoms/Search.tsx";
import { createTooltip } from "@/atoms/Tooltip";
import {
	MessageView,
	UserDisplayName,
} from "@/components/features/chat/Message";
import { Reactions } from "@/components/features/chat/Reactions";
import { createEditor } from "@/components/features/editor/Editor";
import { serializeToMarkdown } from "@/components/features/editor/serializer.ts";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useChannel } from "@/contexts/channel";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useUploads } from "@/contexts/uploads";
import { useMessageSubmit } from "@/hooks/useMessageSubmit";
import { usePermissions } from "@/hooks/usePermissions";
import { flags } from "@/lib/flags";
import { getMessageOverrideName } from "@/utils/general";
import {
	icCheck,
	icChevron,
	icCollapse,
	icEdit,
	icExpand,
	icReply,
	icSort,
} from "@/utils/icons.ts";
import { Forum2CreateForm } from "../../shared/Forum2CreateForm.tsx";
import { RenderUploadItem } from "../chat/Input.tsx";
import { MessageSkeleton } from "../chat/MessageSkeleton.tsx";
import { MessageToolbarMount } from "../chat/MessageToolbar.tsx";
import { MessageToolbarProvider } from "../chat/message-toolbar-context.tsx";
import { TimelineProvider, useTimeline } from "../chat/timeline-context.tsx";
import { Comment } from "./Comment.tsx";
import { type CommentSort, CommentSorting } from "./CommentSorting.tsx";
import { ThreadCard } from "./ThreadCard.tsx";
import {
	type Forum2Sort,
	type Forum2View,
	ThreadSorting,
} from "./ThreadSorting.tsx";

// Type guard for RoomMember with override_name
function hasOverrideName(
	m: RoomMember | undefined,
): m is RoomMember & { override_name: string } {
	return m !== undefined && "override_name" in m;
}

// Type guard for Channel with last_version_id
function hasLastVersionId(
	ch: Channel,
): ch is Channel & { last_version_id: string } {
	return "last_version_id" in ch;
}

const InputReply = (props: { thread: Channel; reply: Message }) => {
	const tip = createTooltip({ tip: () => "remove reply" });
	const [_ch, chUpdate] = useChannel();

	return (
		<div class="reply">
			<button
				type="button"
				class="cancel"
				onClick={() => chUpdate("reply_id", undefined)}
				ref={tip.content}
			>
				<Icon src={cancelIc} />
			</button>
			<div class="info">
				replying to{" "}
				<UserDisplayName
					user_id={props.reply.author_id}
					room_id={props.thread.room_id ?? undefined}
					thread_id={props.thread.id}
				/>
			</div>
		</div>
	);
};

export const Forum2 = (props: { channel: Channel }) => {
	const channels2 = useChannels();
	const threads2 = useThreads();
	const room_id = () => props.channel.room_id ?? "";
	const forum_id = () => props.channel.id;
	const prefsService = usePreferences();
	const prefs = prefsService.useRead();
	const openInSidebar = () => prefs.frontend.threads_sidebar_forum === "yes";

	const [sortBy, setSortBy] = createSignal<Forum2Sort>("new");
	const [viewAs, setViewAs] = createSignal<Forum2View>("list");
	const [showRemoved, setShowRemoved] = createSignal(false);
	const [menuOpen, setMenuOpen] = createSignal(false);
	const [referenceEl, setReferenceEl] = createSignal<HTMLElement>();
	const [floatingEl, setFloatingEl] = createSignal<HTMLElement>();
	const position = useFloating(referenceEl, floatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(5), flip(), shift()],
		placement: "bottom-end",
	});

	const clickOutside = (e: MouseEvent) => {
		if (
			menuOpen() &&
			referenceEl() &&
			!referenceEl()?.contains(e.target as Node) &&
			floatingEl() &&
			!floatingEl()?.contains(e.target as Node)
		) {
			setMenuOpen(false);
		}
	};

	createEffect(() => {
		if (menuOpen()) {
			document.addEventListener("mousedown", clickOutside);
			onCleanup(() => document.removeEventListener("mousedown", clickOutside));
		}
	});

	// Call the appropriate hook based on filter at component level
	const activeThreads = threads2.useListForChannel(forum_id);
	const archivedThreads = threads2.useListArchivedForChannel(forum_id);
	const removedThreads = threads2.useListRemovedForChannel(forum_id);

	// TODO: Implement proper pagination for threads

	const sortThreads = (items: Channel[]) => {
		return [...items].sort((a, b) => {
			if (sortBy() === "new") {
				return a.id < b.id ? 1 : -1;
			} else if (sortBy() === "activity") {
				const tA = hasLastVersionId(a) ? a.last_version_id : a.id;
				const tB = hasLastVersionId(b) ? b.last_version_id : b.id;
				return tA < tB ? 1 : -1;
			}
			return 0;
		});
	};

	const unorderedThreads = createMemo(() => {
		const allIds = new Set([
			...(activeThreads()?.state.ids ?? []),
			...(archivedThreads()?.state.ids ?? []),
			...(showRemoved() ? (removedThreads()?.state.ids ?? []) : []),
		]);
		const threads = [...allIds]
			.map((id) => channels2.cache.get(id))
			.filter(
				(t): t is Channel =>
					t !== undefined && t.parent_id === props.channel.id,
			);
		return sortThreads(threads);
	});

	const threads = createMemo(() => {
		const all = unorderedThreads();
		return all.reduce(
			(acc, t) => {
				if (t.archived_at) {
					acc.archived.push(t);
				} else {
					acc.active.push(t);
				}
				return acc;
			},
			{ active: [] as Channel[], archived: [] as Channel[] },
		);
	});

	function createThread() {
		setShowCreateForm(true);
	}

	const [_bottom, setBottom] = createSignal<Element | undefined>();
	const [showCreateForm, setShowCreateForm] = createSignal(false);

	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	// TODO: implement this
	// const timeline = useTimeline();
	// timeline.commands.on("scrollBy", () => { });
	// timeline.commands.on("jumpToBottom", () => { });
	// timeline.commands.on("jumpToTop", () => { });
	// timeline.commands.on("jumpToMessage", () => { });
	// timeline.commands.on("ackMessage", () => { });
	// <TimelineProvider channel={...}></TimelineProvider>

	return (
		<div class="forum2">
			<div class="forum2-list list">
				<Show when={flags.has("thread_quick_create") && false}>
					<br />
					{/* TODO: <QuickCreate channel={props.channel} /> */}
					<br />
				</Show>
				<div class="forum2-header">
					<Search placeholder="search threads..." />
					<button
						type="button"
						class="button primary"
						style="margin-left: 8px;border-radius:4px"
						onClick={createThread}
					>
						create thread
					</button>
				</div>
				<Show when={showCreateForm()}>
					<Forum2CreateForm
						channel={props.channel}
						onCancel={() => setShowCreateForm(false)}
						onSuccess={() => setShowCreateForm(false)}
					/>
				</Show>
				<div style="display:flex; align-items:center">
					<h3 style="font-size:1rem; margin-top:8px;flex:1">
						{activeThreads()?.state.ids.length ?? "loading"} threads
					</h3>
					<div class="sort-view-container">
						<button
							type="button"
							class="button sort-view-button"
							ref={setReferenceEl}
							onClick={() => setMenuOpen(!menuOpen())}
							classList={{ selected: menuOpen() }}
						>
							<span>sort and view</span>
							<Icon src={icChevron} />
						</button>
						<Portal>
							<Show when={menuOpen()}>
								<div
									ref={setFloatingEl}
									class="sort-view-menu"
									style={{
										position: position.strategy,
										top: `${position.y ?? 0}px`,
										left: `${position.x ?? 0}px`,
										"z-index": 1000,
									}}
								>
									<ThreadSorting
										sorting={sortBy()}
										view={viewAs()}
										onSort={(s) => {
											setSortBy(s);
											setMenuOpen(false);
										}}
										onView={(v) => {
											setViewAs(v);
											setMenuOpen(false);
										}}
										showRemoved={showRemoved()}
										onToggleRemoved={(s) => {
											setShowRemoved(s);
											setMenuOpen(false);
										}}
										canManage={perms.has("ThreadManage")}
									/>
								</div>
							</Show>
						</Portal>
					</div>
				</div>

				<ul>
					<For each={threads().active}>
						{(thread) => (
							<li>
								<ThreadCard thread={thread} openInSidebar={openInSidebar()} />
							</li>
						)}
					</For>
				</ul>

				<h3 class="dim" style="margin-top:16px;">
					older threads
				</h3>
				<ul>
					<For each={threads().archived}>
						{(thread) => (
							<li>
								<ThreadCard thread={thread} openInSidebar={openInSidebar()} />
							</li>
						)}
					</For>
				</ul>

				<div ref={setBottom}></div>
			</div>
		</div>
	);
};

export const Forum2Thread = (props: { channel: Channel }) => {
	const channels2 = useChannels();
	const messagesService = useMessages();
	const [commentSortBy, setCommentSortBy] = createSignal<CommentSort>("new");
	const [ch, chUpdate] = useChannel();
	const [commentMenuOpen, setCommentMenuOpen] = createSignal(false);
	const [commentReferenceEl, setCommentReferenceEl] =
		createSignal<HTMLElement>();
	const [commentFloatingEl, setCommentFloatingEl] = createSignal<HTMLElement>();
	const commentPosition = useFloating(commentReferenceEl, commentFloatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(5), flip(), shift()],
		placement: "bottom-end",
	});

	const clickOutsideComment = (e: MouseEvent) => {
		if (
			commentMenuOpen() &&
			commentReferenceEl() &&
			!commentReferenceEl()?.contains(e.target as Node) &&
			commentFloatingEl() &&
			!commentFloatingEl()?.contains(e.target as Node)
		) {
			setCommentMenuOpen(false);
		}
	};

	createEffect(() => {
		if (commentMenuOpen()) {
			document.addEventListener("mousedown", clickOutsideComment);
			onCleanup(() =>
				document.removeEventListener("mousedown", clickOutsideComment),
			);
		}
	});

	const submit = useMessageSubmit(() => props.channel.id);
	const uploads = useUploads();
	const currentUser = useCurrentUser();
	const reply_id = () => ch.reply_id;
	const reply = () => {
		const rid = reply_id();
		return rid ? messagesService.cache.get(rid) : undefined;
	};
	const storageKey = () => `editor_draft_${props.channel.id}`;
	const channelId = createMemo(() => props.channel.id);

	function handleUpload(file: File) {
		const local_id = uuidv7();
		uploads.init(local_id, props.channel.id, file);
	}

	function uploadFile(e: InputEvent) {
		const target = e.target as HTMLInputElement | null;
		if (!target?.files) return;
		const files = Array.from(target.files);
		for (const file of files) {
			handleUpload(file);
		}
	}

	const atts = () => ch.attachments;
	const sendTyping = leading(
		throttle,
		() => {
			channels2.typing(props.channel.id);
		},
		8000,
	);
	const comments = messagesService.listReplies(
		() => props.channel.id,
		() => undefined,
		() => ({ depth: 8, breadth: 9999, limit: 1024 }),
	);

	const commentTree = createMemo<CommentNode[]>(() => {
		const children = comments()?.children;
		if (!children) return [];

		const buildNodes = (nodes: RepliesMessage[]): CommentNode[] => {
			return nodes
				.filter((i) => i.message.id !== i.message.channel_id)
				.map((node) => ({
					message: node.message,
					children: node.children ? buildNodes(node.children) : [],
				}));
		};

		return buildNodes(children);
	});

	const collapseTooltip = createTooltip({ tip: () => "Collapse Replies" });
	const expandTooltip = createTooltip({ tip: () => "Expand All" });
	const sortTooltip = createTooltip({ tip: () => "Sort Comments" });

	const collapsed = new ReactiveSet<string>();

	const expandAll = () => {
		collapsed.clear();
	};

	const collapseAll = () => {
		function collapseChildren(nodes: CommentNode[]) {
			for (const node of nodes) {
				collapsed.add(node.message.id);
				collapseChildren(node.children);
			}
		}

		for (const topLevelNode of commentTree()) {
			collapseChildren(topLevelNode.children);
		}
	};

	let slowmodeRef!: HTMLDivElement;

	const slowmodeShake = () => {
		const SCALEX = 1.5;
		const SCALEY = 0.4;
		const FRAMES = 10;
		const rnd = (sx: number, sy: number) =>
			`${Math.random() * sx - sx / 2}px ${Math.random() * sy - sy / 2}px`;
		const translations = new Array(FRAMES)
			.fill(0)
			.map((_, i) => rnd(i * SCALEX, i * SCALEY))
			.reverse();
		const reduceMotion = false; // TODO
		slowmodeRef.animate(
			{
				translate: reduceMotion ? [] : translations,
				color: ["red", ""],
			},
			{ duration: 200, easing: "linear" },
		);
	};

	const onSubmit = (text: string) => {
		if (locked()) return false;
		if (slowmodeActive()) {
			slowmodeShake();
			return false;
		}
		submit(text, bypassSlowmode());
		localStorage.removeItem(storageKey());
		return true;
	};

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();
	const editor = createEditor({
		channelId: () => props.channel.id,
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => {
			const draft = localStorage.getItem(storageKey());
			if (!draft) return "";
			try {
				const parsed = JSON.parse(draft);
				return parsed.content ?? draft;
			} catch {
				return draft;
			}
		},
	});

	const onChange = (state: EditorState) => {
		chUpdate("editor_state", state);
		const content = serializeToMarkdown(state.doc);
		localStorage.setItem(
			storageKey(),
			JSON.stringify({
				content,
				timestamp: Date.now(),
			}),
		);
		if (content.trim().length > 0) {
			sendTyping();
		} else {
			sendTyping.clear();
		}
	};

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const customMatch = emoji.match(/^<:([^:]+):([^>]+)>$/);
			let tr: Transaction;
			if (customMatch) {
				const name = customMatch[1];
				const id = customMatch[2];
				tr = editorState.tr.replaceWith(
					from,
					to,
					editor.schema.nodes.emojiCustom.create({ id, name }),
				);
			} else {
				tr = editorState.tr.insertText(emoji, from, to);
			}
			const newState = editorState.apply(tr);
			chUpdate("editor_state", newState);
		}
	};

	const send = () => {
		if (locked()) return;
		const state = ch.editor_state;
		if (!state) return;
		const content = serializeToMarkdown(state.doc).trim();
		if (!content) return;
		if (onSubmit(content)) {
			const tr = state.tr.deleteRange(0, state.doc.nodeSize - 2);
			chUpdate("editor_state", state.apply(tr));
		}
	};

	createEffect(() => {
		const state = ch.editor_state;
		editor.setState(state);
		editor.focus();
	});

	createEffect(() => {
		const expireAt = props.channel.slowmode_message_expire_at;
		if (expireAt) {
			const currentExpireAt = ch.slowmode_expire_at;
			const newExpireAt = new Date(expireAt);
			if (
				!currentExpireAt ||
				currentExpireAt.getTime() !== newExpireAt.getTime()
			) {
				chUpdate("slowmode_expire_at", newExpireAt);
			}
		}
	});

	const perms = usePermissions(
		() => currentUser()?.id ?? "",
		() => props.channel.room_id ?? undefined,
		() => props.channel.id,
	);

	const bypassSlowmode = (): boolean =>
		perms.has("ChannelManage") ||
		perms.has("ThreadManage") ||
		perms.has("MemberTimeout");

	const locked = () => {
		return (
			!perms.has("MessageCreate") ||
			("locked" in props.channel &&
				!!props.channel.locked &&
				!perms.has("ThreadManage"))
		);
	};

	const [remainingTime, setRemainingTime] = createSignal(0);
	const slowmodeRemaining = () => remainingTime();
	const slowmodeActive = () => slowmodeRemaining() > 0;

	createEffect(() => {
		const expireAt = ch.slowmode_expire_at;
		if (expireAt) {
			const updateTimer = () => {
				const now = Date.now();
				const remaining = expireAt.getTime() - now;
				setRemainingTime(Math.max(0, remaining));
			};

			updateTimer();
			const interval = setInterval(updateTimer, 1000);
			onCleanup(() => clearInterval(interval));
		} else {
			setRemainingTime(0);
		}
	});

	const slowmodeFormatted = () => {
		const remainingMs = slowmodeRemaining();
		if (remainingMs <= 0 || bypassSlowmode()) {
			const channelSlowmode = props.channel.slowmode_message;
			if (channelSlowmode) {
				const mins = Math.floor(channelSlowmode / 60);
				const secs = channelSlowmode % 60;
				const time =
					mins === 0
						? `slowmode set to ${secs}s`
						: `slowmode set to ${mins}m${secs.toString().padStart(2, "0")}s`;
				return `slowmode set to ${time}${
					bypassSlowmode() ? " (bypassed)" : ""
				}`;
			} else return "no slowmode";
		}
		const seconds = Math.ceil(remainingMs / 1000);
		const mins = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${mins}:${secs.toString().padStart(2, "0")}`;
	};

	const isEmpty = () => !ch.editor_state?.doc.textContent.trim();

	// TODO: more menu buttons
	// - copy link
	// - join/unfollow
	// - view
	// - jump to top

	// const [activity] = createResource(
	// 	() => props.channel.id,
	// 	(channel_id) => messagesService.fetchActivity(channel_id),
	// );
	// activity();

	const firstMessage = messagesService.use(
		() => props.channel.id,
		() => props.channel.id,
	);

	return (
		<MessageToolbarProvider>
			<div class="thread forum2">
				<div class="main">
					<header class="header">
						<h2 class="title">{props.channel.name}</h2>
						<Switch>
							{/* TODO: better rendering for loading/error/not found */}
							<Match when={firstMessage.loading}>
								<MessageSkeleton />
							</Match>
							<Match when={firstMessage.error}>error</Match>
							<Match when={firstMessage()}>
								{(m) => (
									<>
										<MessageView
											message={{ ...m(), thread: null, reactions: [] }}
											separate
										/>
										<br />
										<Reactions message={m()} />
									</>
								)}
							</Match>
							<Match when={!firstMessage.loading && !firstMessage()}>
								not found
							</Match>
						</Switch>
					</header>
					<menu class="forum2-post-menu">
						<div class="left">
							{/* FIXME: total comment count */}
							<div style="margin-right:8px;flex:1">
								{comments()?.children.length ?? 0} comments
							</div>
						</div>
						<div class="right">
							<button
								type="button"
								class="button icon-button"
								ref={collapseTooltip.content}
								onClick={collapseAll}
							>
								<Icon src={icCollapse} />
							</button>
							<button
								type="button"
								class="button icon-button"
								ref={expandTooltip.content}
								onClick={expandAll}
							>
								<Icon src={icExpand} />
							</button>
							<button
								type="button"
								class="button icon-button"
								ref={(el) => {
									setCommentReferenceEl(el);
									sortTooltip.content(el);
								}}
								onClick={() => setCommentMenuOpen(!commentMenuOpen())}
								classList={{ selected: commentMenuOpen() }}
							>
								<Icon src={icSort} />
							</button>
						</div>
						<Portal>
							<Show when={commentMenuOpen()}>
								<div
									ref={setCommentFloatingEl}
									style={{
										position: commentPosition.strategy,
										top: `${commentPosition.y ?? 0}px`,
										left: `${commentPosition.x ?? 0}px`,
										"z-index": 1000,
									}}
								>
									<CommentSorting
										sorting={commentSortBy()}
										onSort={(s) => {
											setCommentSortBy(s);
											setCommentMenuOpen(false);
										}}
									/>
								</div>
							</Show>
						</Portal>
					</menu>
					<Forum2Comments
						channel={props.channel}
						commentTree={commentTree()}
						collapsed={collapsed}
					/>
					<div class="comment-input" classList={{ locked: locked() }}>
						<Show when={reply()}>
							{(r) => <InputReply thread={props.channel} reply={r()} />}
						</Show>
						<Show when={atts()?.length}>
							<div class="attachments">
								<header>
									{atts()?.length}{" "}
									{atts()?.length === 1 ? "attachment" : "attachments"}
								</header>
								<ul>
									<For each={atts()}>
										{(att) => (
											<RenderUploadItem
												thread_id={props.channel.id}
												att={att}
											/>
										)}
									</For>
								</ul>
							</div>
						</Show>
						<div class="text">
							<label class="upload">
								+
								<input
									multiple
									type="file"
									onInput={uploadFile}
									value="upload file"
									disabled={locked()}
								/>
							</label>
							<editor.View
								onSubmit={onSubmit}
								onChange={onChange}
								placeholder={
									locked() ? "This thread is locked" : "add a comment..."
								}
								channelId={props.channel.id}
								submitOnEnter={false}
								disabled={locked()}
							/>
							<EmojiButton picked={onEmojiPick} />
						</div>
						<footer style="display: flex; align-items: center;">
							<Show when={props.channel.slowmode_message || slowmodeActive()}>
								<div class="slowmode" ref={slowmodeRef}>
									{slowmodeFormatted()}
								</div>
							</Show>
							<div style="flex:1"></div>
							<menu>
								<button
									type="button"
									class="button big"
									onClick={send}
									disabled={locked() || isEmpty()}
								>
									send
								</button>
							</menu>
						</footer>
					</div>
				</div>
				<MessageToolbarMount />
			</div>
		</MessageToolbarProvider>
	);
};

type ThreadActivityProps = {
	comments: { items: Array<{ id: string }> } | undefined;
	commentTree: Array<unknown>;
};

// TODO: implement
const ThreadActivity = (props: ThreadActivityProps) => {
	const comments = () => props.comments;
	const commentTree = () => props.commentTree;

	return (
		<aside class="aside">
			<h3 class="dim">thread info</h3>
			<ul>
				<li>tags: [foo] [bar] [baz]</li>
				<li>
					comments: [{comments()?.items.length ?? 0}] comments ([
					{commentTree().length}] threads/top level comments)
				</li>
				<li>
					last comment: <a href="#">some time ago</a>
				</li>
			</ul>
			<br />
			<h3 class="dim">thread log</h3>
			<ul>
				<li>[user] renamed to [name]</li>
				<li>[user] added tag to [name]</li>
				<li>[user] pinned [a message]</li>
				<li>[user] added [member] to the thread</li>
				<li>[user] removed [member] from the thread</li>
				<li>mentioned in [thread]</li>
			</ul>
		</aside>
	);
};

export interface CommentNode {
	message: Message;
	children: CommentNode[];
}

export const Forum2Comments = (props: {
	channel: Channel;
	commentTree: CommentNode[];
	collapsed: ReactiveSet<string>;
}) => {
	return (
		<div class="comments">
			<ul>
				<For each={props.commentTree}>
					{(node) => (
						<li class="toplevel">
							<Comment
								collapsed={props.collapsed}
								channel={props.channel}
								node={node}
								depth={0}
							/>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

export function CommentEditor(props: { message: Message; channel: Channel }) {
	const messagesService = useMessages();
	const [ch, chUpdate] = useChannel();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();
	const [draft, setDraft] = createSignal(
		props.message.latest_version.type === "DefaultMarkdown"
			? (props.message.latest_version.content ?? "")
			: "",
	);

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const customMatch = emoji.match(/^<:([^:]+):([^>]+)>$/);
			let tr: Transaction;
			if (customMatch) {
				const name = customMatch[1];
				const id = customMatch[2];
				tr = editorState.tr.replaceWith(
					from,
					to,
					editor.schema.nodes.emojiCustom.create({ id, name }),
				);
			} else {
				tr = editorState.tr.insertText(emoji, from, to);
			}
			const newState = editorState.apply(tr);
			chUpdate("editor_state", newState);
		}
	};

	const editor = createEditor({
		channelId: () => props.message.channel_id,
		roomId: () => props.message.room_id!,
		toolbar,
		autocomplete,
		initialContent: () => draft(),
		initialSelection: "end",
	});

	const save = (content: string) => {
		const currentContent =
			props.message.latest_version.type === "DefaultMarkdown"
				? (props.message.latest_version.content ?? "")
				: "";

		if (content.trim() === currentContent.trim()) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		messagesService
			.edit(props.message.channel_id, props.message.id, content)
			.catch((e) => {
				console.error("failed to edit comment", e);
			});
		chUpdate("editingMessage", undefined);
		return true;
	};

	const cancel = () => {
		chUpdate("editingMessage", undefined);
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
		<div class="comment-editor" ref={containerRef}>
			<div class="text">
				<editor.View
					onSubmit={save}
					onChange={(state) => {
						const text = state.doc.textContent;
						setDraft(text);
					}}
					channelId={props.channel.id}
					submitOnEnter={false}
				/>
				<EmojiButton picked={onEmojiPick} />
			</div>
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

// TODO: name colors
// <div class="author">
//   {#await author}
//     <i>loading...</i>
//   {:then author}
//     {@const name = author?.getContent()?.name}
//     {#if name && isFromOp}
//       <b>{name}</b> (op)
//     {:else if name && author?.origin_ts < (Date.now() + 1000 * 60 * 60 * 24 * 7)}
//       <span class="green">{name}</span>
//     {:else if name}
//       {name}
//     {:else}
//       <i>anonymous</i>
//     {/if}
//   {:catch}
//     <i>anonymous</i>
//   {/await}
// </div>

export const Forum2ThreadPage = (props: { channel: Channel }) => {
	return (
		<div class="forum2">
			<Forum2Thread channel={props.channel} />
		</div>
	);
};
