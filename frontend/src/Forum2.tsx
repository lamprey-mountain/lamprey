import { useCurrentUser } from "./contexts/currentUser.tsx";
// TODO: refactor out duplicated code from here and Message.tsx

import {
	type Attachment,
	Channel,
	getTimestampFromUUID,
	type Media,
	Message,
} from "sdk";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	createUniqueId,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { useCtx } from "./context";
import { useMenu, useUserPopout } from "./contexts/mod.tsx";
import {
	useApi2,
	useChannels2,
	useMessages2,
	useRoomMembers2,
	useThreads2,
	useUsers2,
} from "@/api";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "./atoms/Time";
import { A, useNavigate } from "@solidjs/router";
import { serializeToMarkdown } from "./components/features/editor/serializer.ts";
import { useModals } from "./contexts/modal";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { usePermissions } from "./hooks/usePermissions";
import { md } from "./markdown_utils";
import { flags } from "./flags";
import { Dropdown } from "./atoms/Dropdown";
import { Author, MessageToolbar } from "./components/features/chat/Message";
import { Markdown } from "./atoms/Markdown";
import { render } from "solid-js/web";
import { getEmojiUrl, type MediaProps } from "./media/util";
import { Reactions } from "./components/features/chat/Reactions";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "./media/mod";
import {
	ChannelContext,
	createInitialChannelState,
	useChannel,
} from "./channelctx";
import { createStore } from "solid-js/store";
import { useMessageSubmit } from "./hooks/useMessageSubmit";
import { createEditor } from "./components/features/editor/Editor";
import type { EditorState } from "prosemirror-state";

import { Resizable } from "./atoms/Resizable";
import { getMessageOverrideName } from "./utils/general";
import cancelIc from "./assets/x.png";
import { createTooltip } from "./atoms/Tooltip";
import { leading, throttle } from "@solid-primitives/scheduled";
import { ChannelIcon } from "./User";
import { EmojiButton } from "./atoms/EmojiButton";
import { uuidv7 } from "uuidv7";
import { useUploads } from "./contexts/uploads";
import { Match, Switch } from "solid-js";
import icDelete from "./assets/delete.png";

function AttachmentView(props: MediaProps) {
	const b = () => props.media.content_type.split("/")[0];
	const ty = () => props.media.content_type.split(";")[0];
	if (b() === "image") {
		return (
			<li class="raw">
				<ImageView media={props.media} />
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
		b() === "text" ||
		/^application\/json\b/.test(props.media.content_type)
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

const InputReply = (props: { thread: Channel; reply: Message }) => {
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();
	const tip = createTooltip({ tip: () => "remove reply" });
	const [_ch, chUpdate] = useChannel()!;
	const getName = (user_id: string) => {
		const user = users2.use(() => user_id);
		const room_id = props.thread.room_id;
		if (!room_id) {
			return user()?.name;
		}
		const member = roomMembers2.use(() => `${room_id}:${user_id}`);

		const m = member();
		return ((m as any)?.membership === "Join" && (m as any)?.override_name) ??
			user()?.name;
	};

	const getNameNullable = (user_id?: string) => {
		if (user_id) return getName(user_id);
	};

	return (
		<div class="reply">
			<button
				class="cancel"
				onClick={() => chUpdate("reply_id", undefined)}
				ref={tip.content}
			>
				<img class="icon" src={cancelIc} />
			</button>
			<div class="info">
				replying to{" "}
				<b>
					{getMessageOverrideName(props.reply) ??
						getNameNullable(props.reply?.author_id)}
				</b>
			</div>
		</div>
	);
};

export const Forum2 = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const channels2 = useChannels2();
	const threads2 = useThreads2();
	const nav = useNavigate();
	const [, modalctl] = useModals();
	const room_id = () => props.channel.room_id!;
	const forum_id = () => props.channel.id;

	const [threadFilter, setThreadFilter] = createSignal("active");
	const [sortBy, setSortBy] = createSignal<
		"new" | "activity" | "reactions:+1" | "random" | "hot" | "hot2"
	>("new");
	const [viewAs, setViewAs] = createSignal<"list" | "gallery">("list");
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
			!referenceEl()!.contains(e.target as Node) &&
			floatingEl() &&
			!floatingEl()!.contains(e.target as Node)
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

	const getThreadsList = () => {
		const filter = threadFilter();
		if (filter === "active") return activeThreads;
		if (filter === "archived") return archivedThreads;
		if (filter === "removed") return removedThreads;
		return activeThreads;
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	// TODO: Implement proper pagination for threads

	const getThreads = () => {
		const list = getThreadsList()?.();
		if (!list) return [];
		const items = list.state.ids.map((id) => channels2.cache.get(id)).filter((
			t,
		): t is Channel => t !== undefined && t.parent_id === props.channel.id);
		// sort descending by id
		return [...items].sort((a, b) => {
			if (sortBy() === "new") {
				return a.id < b.id ? 1 : -1;
			} else if (sortBy() === "activity") {
				// activity
				const tA = (a as any).last_version_id ?? a.id;
				const tB = (b as any).last_version_id ?? b.id;
				return tA < tB ? 1 : -1;
			}
			return 0;
		});
	};

	function createThread(room_id: string) {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			channels2.create(room_id, {
				name,
				parent_id: props.channel.id,
			});
		});
	}

	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	const [threadId, setThreadId] = createSignal<null | string>(null);

	const getOrCreateChannelContext = (channelId: string) => {
		if (!ctx.channel_contexts.has(channelId)) {
			const store = createStore(createInitialChannelState());
			ctx.channel_contexts.set(channelId, store);
		}
		return ctx.channel_contexts.get(channelId)!;
	};

	return (
		<div class="forum2">
			<Resizable
				storageKey="forum-sidebar-width"
				side="left"
				initialWidth={350}
				minWidth={250}
				maxWidth={600}
			>
				<div class="list">
					<Show when={flags.has("thread_quick_create")}>
						<br />
						{/* TODO: <QuickCreate channel={props.channel} /> */}
						<br />
					</Show>
					<div style="display:flex; align-items:center">
						<input placeholder="search" type="search" class="search-pad" />
						<button
							class="primary"
							style="margin-left: 8px;border-radius:4px"
							onClick={() => createThread(room_id())}
						>
							create thread
						</button>
					</div>
					<div style="display:flex; align-items:center">
						<h3 style="font-size:1rem; margin-top:8px;flex:1">
							{getThreads().length} {threadFilter()} threads
						</h3>
						<div class="sort-view-container">
							<button
								ref={setReferenceEl}
								onClick={() => setMenuOpen(!menuOpen())}
								class="secondary sort-view-button"
								classList={{ selected: menuOpen() }}
							>
								<span>sort and view</span>
								<svg
									width="10"
									height="6"
									viewBox="0 0 10 6"
									fill="none"
									xmlns="http://www.w3.org/2000/svg"
								>
									<path
										d="M1 1L5 5L9 1"
										stroke="currentColor"
										stroke-width="1.5"
										stroke-linecap="round"
										stroke-linejoin="round"
									/>
								</svg>
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
										<menu>
											<div class="subtext header">
												sort by
											</div>
											<button
												onClick={() => {
													setSortBy("new");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Newest threads first
												<Show when={sortBy() === "new"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setSortBy("activity");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Recently active threads
												<Show when={sortBy() === "activity"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setSortBy("reactions:+1");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Expected to be helpful
												<Show when={sortBy() === "reactions:+1"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setSortBy("random");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Random ordering
												<Show when={sortBy() === "random"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setSortBy("hot");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Hot
												<Show when={sortBy() === "hot"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setSortBy("hot2");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Hot 2
												<Show when={sortBy() === "hot2"}>
													<span>✓</span>
												</Show>
											</button>
											<hr />
											<div class="subtext header">
												view as
											</div>
											<button
												onClick={() => {
													setViewAs("list");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												List
												<Show when={viewAs() === "list"}>
													<span>✓</span>
												</Show>
											</button>
											<button
												onClick={() => {
													setViewAs("gallery");
													setMenuOpen(false);
												}}
												class="menu-item"
											>
												Gallery
												<Show when={viewAs() === "gallery"}>
													<span>✓</span>
												</Show>
											</button>
										</menu>
									</div>
								</Show>
							</Portal>
						</div>
						<div class="filters">
							<button
								classList={{ selected: threadFilter() === "active" }}
								onClick={[setThreadFilter, "active"]}
							>
								active
							</button>
							<button
								classList={{ selected: threadFilter() === "archived" }}
								onClick={[setThreadFilter, "archived"]}
							>
								archived
							</button>
							<Show when={perms.has("ThreadManage")}>
								<button
									classList={{ selected: threadFilter() === "removed" }}
									onClick={[setThreadFilter, "removed"]}
								>
									removed
								</button>
							</Show>
						</div>
					</div>
					<ul>
						<For each={getThreads()}>
							{(thread) => (
								<li>
									<article
										class="thread menu-thread thread-card"
										data-thread-id={thread.id}
									>
										<header onClick={() => setThreadId(thread.id)}>
											<div class="top">
												<ChannelIcon channel={thread} />
												<div class="spacer">{thread.name}</div>
												<div class="time">
													Created{" "}
													<Time date={getTimestampFromUUID(thread.id)} />
												</div>
											</div>
											<div
												class="bottom"
												onClick={() => setThreadId(thread.id)}
											>
												<div class="dim">
													{thread.message_count} message(s) &bull; last msg{" "}
													<Time
														date={getTimestampFromUUID(
															(thread as any).last_version_id ?? thread.id,
														)}
													/>
												</div>
												<Show when={thread.description}>
													<div
														class="description markdown"
														innerHTML={md(thread.description ?? "") as string}
													>
													</div>
												</Show>
											</div>
										</header>
									</article>
								</li>
							)}
						</For>
					</ul>
					<div ref={setBottom}></div>
				</div>
			</Resizable>
			<Show when={threadId()}>
				{(tid) => {
					const threadChannel = channels2.cache.get(tid());
					if (!threadChannel) return;
					const threadCtx = getOrCreateChannelContext(tid());
					return (
						<ChannelContext.Provider value={threadCtx}>
							<Forum2Thread channel={threadChannel} />
						</ChannelContext.Provider>
					);
				}}
			</Show>
		</div>
	);
};

function EditorUserMention(props: { id: string }) {
	const users2 = useUsers2();
	const user = users2.use(() => props.id);
	return <span class="mention-user">@{user()?.name ?? props.id}</span>;
}

function EditorChannelMention(props: { id: string }) {
	const channels2 = useChannels2();
	const channel = createMemo(() => channels2.cache.get(props.id));
	return <span class="mention-channel">#{channel()?.name ?? props.id}</span>;
}

export const Forum2Thread = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const channels2 = useChannels2();
	const messagesService = useMessages2();
	const [ch, chUpdate] = useChannel()!;
	const submit = useMessageSubmit(props.channel.id);
	const uploads = useUploads();
	const currentUser = useCurrentUser();
	const reply_id = () => ch.reply_id;
	const reply = () => messagesService.cache.get(reply_id()!);
	const storageKey = () => `editor_draft_${props.channel.id}`;

	function handleUpload(file: File) {
		const local_id = uuidv7();
		uploads.init(local_id, props.channel.id, file);
	}

	function uploadFile(e: InputEvent) {
		const target = e.target! as HTMLInputElement;
		const files = Array.from(target.files!);
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
		const items = comments()?.items;
		if (!items) return [];

		const commentMap = new Map<string, CommentNode>();
		for (const message of items) {
			commentMap.set(message.id, { message, children: [] });
		}

		const rootComments: CommentNode[] = [];
		for (const node of commentMap.values()) {
			const msg = node.message;
			const replyId = msg.latest_version.type === "DefaultMarkdown"
				? msg.latest_version.reply_id
				: undefined;

			if (replyId && commentMap.has(replyId)) {
				commentMap.get(replyId)!.children.push(node);
			} else {
				rootComments.push(node);
			}
		}

		return rootComments;
	});

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
		submit(text, bypassSlowmode);
		localStorage.removeItem(storageKey());
		return true;
	};

	const editor = createEditor({
		channelId: () => props.channel.id,
		roomId: () => props.channel.room_id!,
		initialContent: (() => {
			const draft = localStorage.getItem(storageKey());
			if (!draft) return "";
			try {
				const parsed = JSON.parse(draft);
				return parsed.content ?? draft;
			} catch {
				return draft;
			}
		})(),
		mentionRenderer: (node, userId) => {
			render(() => <EditorUserMention id={userId} />, node);
		},
		mentionChannelRenderer: (node, channelId) => {
			render(() => <EditorChannelMention id={channelId} />, node);
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
			let tr;
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

	const bypassSlowmode = () =>
		perms.has("ChannelManage") ||
		perms.has("ThreadManage") ||
		perms.has("MemberTimeout");

	const locked = () => {
		return !perms.has("MessageCreate") ||
			((props.channel.locked as any) && !perms.has("ThreadManage"));
	};

	const [remainingTime, setRemainingTime] = createSignal(0);
	const slowmodeRemaining = () => remainingTime();
	const slowmodeActive = () => slowmodeRemaining() > 0;

	createEffect(() => {
		const expireAt = ch.slowmode_expire_at;
		if (expireAt) {
			const updateTimer = () => {
				const now = new Date().getTime();
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
				const time = mins === 0
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

	return (
		<div class="thread">
			<div class="main">
				<div>
					<h2 class="title">{props.channel.name}</h2>
				</div>
				<div style="display:flex">
					<div style="flex:1">
						{comments()?.items.length ?? 0} comments
						<button onClick={collapseAll}>collapse replies</button>
						<button onClick={expandAll}>expand all</button>
					</div>
					<div>
						<div>
							order by{" "}
							<Dropdown
								options={[
									{ item: "new", label: "newest comments first" },
									{ item: "old", label: "oldest comments first" },
									{
										item: "activity",
										label: "recently active comment threads",
									},
									{ item: "reactions:+1", label: "most +1 reactions" },
									{ item: "random", label: "random ordering" },
									{ item: "hot", label: "mystery algorithm 1" },
									{ item: "hot2", label: "mystery algorithm 2" },
									// NOTE: hacker news algorithm
									//   score = points / ((time + 2) ** gravity)
									//   time = how old the post is in hours(?)
									//   gravity = 1.8
								]}
							/>
						</div>
					</div>
				</div>
				<Forum2Comments
					channel={props.channel}
					commentTree={commentTree()}
					collapsed={collapsed}
				/>
				<div
					class="comment-input"
					classList={{ locked: locked() }}
				>
					<Show when={reply()}>
						<InputReply thread={props.channel} reply={reply()!} />
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
											att={att as any}
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
							placeholder={locked()
								? "This thread is locked"
								: "add a comment..."}
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
								class="big primary"
								onClick={send}
								disabled={locked() || isEmpty()}
							>
								send
							</button>
						</menu>
					</footer>
				</div>
			</div>
		</div>
	);
};

const ThreadLog = (props: { comments: any; commentTree: any }) => {
	const comments = () => props.comments;
	const commentTree = () => props.commentTree;

	return (
		<div class="aside">
			<h3 class="dim">thread info</h3>
			<ul>
				<li>tags: [foo] [bar] [baz]</li>
				<li>
					comments: [{comments()?.items.length ?? 0}] comments ([{commentTree()
						.length}] threads/top level comments)
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
		</div>
	);
};

export interface CommentNode {
	message: Message;
	children: CommentNode[];
}

export const Forum2Comments = (
	props: {
		channel: Channel;
		commentTree: CommentNode[];
		collapsed: ReactiveSet<string>;
	},
) => {
	return (
		<div class="comments">
			<div>comments</div>
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

const contentToHtml = new WeakMap();

function highlight(el: Element) {
	el.animate([
		{
			boxShadow: "4px 0 0 -1px inset #cc1856",
			backgroundColor: "#cc185622",
			offset: 0,
		},
		{
			boxShadow: "4px 0 0 -1px inset #cc1856",
			backgroundColor: "#cc185622",
			offset: .8,
		},
		{
			boxShadow: "none",
			backgroundColor: "transparent",
			offset: 1,
		},
	], {
		duration: 1000,
	});
}

function CommentEditor(
	props: { message: Message; channel: Channel },
) {
	const ctx = useCtx();
	const messagesService = useMessages2();
	const [ch, chUpdate] = useChannel()!;
	const [draft, setDraft] = createSignal(
		props.message.latest_version.type === "DefaultMarkdown"
			? props.message.latest_version.content ?? ""
			: "",
	);

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const customMatch = emoji.match(/^<:([^:]+):([^>]+)>$/);
			let tr;
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
		initialContent: draft(),
		initialSelection: "end",
	});

	const save = (content: string) => {
		const currentContent =
			props.message.latest_version.type === "DefaultMarkdown"
				? props.message.latest_version.content ?? ""
				: "";

		if (content.trim() === currentContent.trim()) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		messagesService.edit(
			props.message.channel_id,
			props.message.id,
			content,
		).catch((e) => {
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
				escape to <button onClick={cancel}>cancel</button> • enter to{" "}
				<button onClick={() => save(draft())}>save</button>
			</div>
		</div>
	);
}

const Comment = (
	props: {
		collapsed: ReactiveSet<string>;
		channel: Channel;
		node: CommentNode;
		depth: number;
	},
) => {
	const message = () => props.node.message;
	const children = () => props.node.children;
	const api2 = useApi2();
	const [ch, chUpdate] = useChannel()!;

	const collapsed = () => props.collapsed.has(message().id);
	const isEditing = () => ch.editingMessage?.message_id === message().id;
	const isSelected = () => ch.selectedMessages?.includes(message().id) ?? false;
	const isReplyTarget = () => ch.reply_id === message().id;
	const inSelectMode = () => ch.selectMode ?? false;

	const currentUser = useCurrentUser();
	const isOwnMessage = () => {
		return currentUser()?.id === message().author_id;
	};

	const canEditMessage = () => {
		return (message() as any).type === "DefaultMarkdown" &&
			!message().is_local &&
			isOwnMessage();
	};

	const handleClick = (e: MouseEvent) => {
		if (!inSelectMode() || !chUpdate) return;
		e.preventDefault();
		e.stopPropagation();

		const message_id = message().id;
		const selected = ch.selectedMessages;

		if (e.shiftKey && selected.length > 0) {
			// TODO: range selection for comments
			if (selected.includes(message_id)) {
				chUpdate(
					"selectedMessages",
					selected.filter((id) => id !== message_id),
				);
			} else {
				chUpdate("selectedMessages", [...selected, message_id]);
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

	const [summary] = createResource(
		() => {
			const v = message().latest_version;
			if (v.type === "DefaultMarkdown" && v.content) {
				return {
					content: v.content,
					channel_id: message().channel_id,
					mentions: v.mentions,
				};
			}
			return null;
		},
		async (data) => {
			if (!data) return "(no content)";
			return await api2.stripMarkdownAndResolveMentions(
				data.content,
				data.channel_id,
				data.mentions,
			);
		},
	);

	const countAllChildren = (node: CommentNode): number => {
		return node.children.length +
			node.children.reduce((sum, child) => sum + countAllChildren(child), 0);
	};

	let contentEl!: HTMLElement;

	createEffect(() => {
		const hl = ch.highlight;
		if (hl === message().id) {
			// expand parent comments
			// props.expand(); // TODO: we need a way to expand parents if they are collapsed
			// for now we just scroll to it
			const el = contentEl?.closest(".comment");
			if (el) {
				el.scrollIntoView({ block: "center" });
				highlight(el);
				chUpdate("highlight", undefined);
			}
		}
	});

	return (
		<div
			class="comment menu-message"
			data-message-id={message().id}
			classList={{
				collapsed: collapsed(),
				selected: isSelected(),
				"reply-target": isReplyTarget(),
				selectable: inSelectMode(),
			}}
			style={{
				"--depth": props.depth,
				"--is-darker": props.depth % 2 === 1 ? 1 : 0,
			}}
			onClick={handleClick}
		>
			<div class="inner">
				<header>
					<button
						class="collapse"
						onClick={() =>
							collapsed()
								? props.collapsed.delete(message().id)
								: props.collapsed.add(message().id)}
					>
						{collapsed() ? "+" : "-"}
					</button>
					<Show when={collapsed()}>
						<span class="childCount dim">[{countAllChildren(props.node)}]</span>
					</Show>
					<Show when={props.channel}>
						<Author message={props.node.message} thread={props.channel} />
					</Show>
					<Time date={getTimestampFromUUID(message().id)} />
					<Show when={collapsed()}>
						<div class="summary">
							{summary() ?? "..."}
						</div>
					</Show>
				</header>
				<Show when={!collapsed()}>
					<Show
						when={!isEditing()}
						fallback={
							<CommentEditor message={message()} channel={props.channel} />
						}
					>
						<Markdown
							content={(message().latest_version as any).type ===
									"DefaultMarkdown"
								? (message().latest_version as any).content ?? ""
								: ""}
							channel_id={message().channel_id}
							class="content"
							ref={contentEl}
						/>
						<div style="padding: 0 8px">
							{(() => {
								const version = message().latest_version;
								return (
									<Show
										when={version.type === "DefaultMarkdown" &&
											version.attachments?.length}
									>
										<ul class="attachments">
											<For
												each={version.type === "DefaultMarkdown"
													? version.attachments
													: []}
											>
												{(att) => (
													<Show when={att.type === "Media"}>
														<AttachmentView media={att.media as Media} />
													</Show>
												)}
											</For>
										</ul>
									</Show>
								);
							})()}
						</div>

						<Show when={message().reactions?.length}>
							<Reactions message={message()} />
						</Show>
						<MessageToolbar message={message()} />
					</Show>
				</Show>
			</div>
			<Show when={!collapsed() && children().length > 0}>
				<ul class="children">
					<For each={children()}>
						{(child) => (
							<li>
								<Comment
									collapsed={props.collapsed}
									channel={props.channel}
									node={child}
									depth={props.depth + 1}
								/>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
};

export function RenderUploadItem(
	props: { thread_id: string; att: Attachment },
) {
	const ctx = useCtx();
	const uploads = useUploads();
	const thumbUrl = URL.createObjectURL((props.att as any).file);
	onCleanup(() => {
		URL.revokeObjectURL(thumbUrl);
	});

	function renderInfo(att: Attachment) {
		const a = att as any;
		if (a.status === "uploading") {
			if (a.progress === 1) {
				return `processing`;
			} else {
				const percent = (a.progress * 100).toFixed(2);
				return `${percent}%`;
			}
		} else {
			return "";
		}
	}

	function getProgress(att: Attachment) {
		const a = att as any;
		if (a.status === "uploading") {
			return a.progress;
		} else {
			return 1;
		}
	}

	function removeAttachment(local_id: string) {
		uploads.cancel(local_id, props.thread_id);
	}

	function pause() {
		uploads.pause((props.att as any).local_id);
	}

	function resume() {
		uploads.resume((props.att as any).local_id);
	}

	return (
		<>
			<div class="upload-item">
				<div class="thumb" style={{ "background-image": `url(${thumbUrl})` }}>
				</div>
				<div class="info">
					<svg class="progress" viewBox="0 0 1 1" preserveAspectRatio="none">
						<rect class="bar" height="1" width={getProgress(props.att)}></rect>
					</svg>
					<div style="display: flex">
						<div style="flex: 1;white-space:nowrap;text-overflow:ellipsis;overflow:hidden">
							{(props.att as any).file.name}
							<span style="color:#888;margin-left:.5ex">
								{renderInfo(props.att)}
							</span>
						</div>
						<menu>
							<Switch>
								<Match
									when={(props.att as any).status === "uploading" &&
										(props.att as any).paused}
								>
									<button onClick={resume}>
										⬆️
									</button>
								</Match>
								<Match when={(props.att as any).status === "uploading"}>
									<button onClick={pause}>⏸️</button>
								</Match>
							</Switch>
							<button
								onClick={() => removeAttachment((props.att as any).local_id)}
							>
								<img class="icon" src={icDelete} />
							</button>
						</menu>
					</div>
				</div>
			</div>
		</>
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
			<div class="thread">
				<div class="main">
					<Forum2Thread channel={props.channel} />
				</div>
			</div>
		</div>
	);
};
