// TODO: refactor out duplicated code from here and Message.tsx

import { Channel, getTimestampFromUUID, Message } from "sdk";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	Show,
} from "solid-js";
import { useCtx } from "./context";
import { useApi } from "./api";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "./Time";
import { A, useNavigate } from "@solidjs/router";
import { useModals } from "./contexts/modal";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { usePermissions } from "./hooks/usePermissions";
import { md } from "./markdown";
import { flags } from "./flags";
import { Dropdown } from "./Dropdown";
import { Author } from "./Message";
import { render } from "solid-js/web";
import twemoji from "twemoji";
import { getEmojiUrl, type MediaProps } from "./media/util";
import { Reactions } from "./Reactions";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "./media/mod";

function UserMention(props: { id: string; channel: Channel }) {
	const api = useApi();
	const ctx = useCtx();
	const user = api.users.fetch(() => props.id);
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
			@{user()?.name ?? "..."}
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
			#{channel()?.name ?? "..."}
		</span>
	);
}

function Emoji(props: { id: string; name: string; animated: boolean }) {
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

function AttachmentView(props: MediaProps) {
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

export const Forum2 = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const [, modalctl] = useModals();
	const room_id = () => props.channel.room_id!;
	const forum_id = () => props.channel.id;

	const [threadFilter, setThreadFilter] = createSignal("active");

	const fetchMore = () => {
		const filter = threadFilter();
		if (filter === "active") {
			return api.threads.listForChannel(forum_id);
		} else if (filter === "archived") {
			return api.threads.listArchivedForChannel(forum_id);
		} else if (filter === "removed") {
			return api.threads.listRemovedForChannel(forum_id);
		}
	};

	const threadsResource = createMemo(fetchMore);

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const getThreads = () => {
		const items = threadsResource()?.()?.items;
		if (!items) return [];
		// sort descending by id
		return [...items].filter((t) => t.parent_id === props.channel.id).sort((
			a,
			b,
		) => (a.id < b.id ? 1 : -1));
	};

	function createThread(room_id: string) {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			api.channels.create(room_id, {
				name,
				parent_id: props.channel.id,
				type: "ThreadPublic",
			});
		});
	}

	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	const [threadId, setThreadId] = createSignal<null | string>(null);

	// TODO: split out room-home thread styling
	return (
		<div class="room-home forum2-main">
			<div class="list">
				<div style="display:flex">
					<div style="flex:1">
						<h2>{props.channel.name}</h2>
						<p
							class="markdown"
							innerHTML={md(props.channel.description ?? "") as string}
						>
						</p>
					</div>
					<div style="display:flex;flex-direction:column;gap:4px">
						<A
							style="padding: 0 4px"
							href={`/channel/${props.channel.id}/settings`}
						>
							settings
						</A>
					</div>
				</div>
				<Show when={flags.has("thread_quick_create")}>
					<br />
					{/* TODO: <QuickCreate channel={props.channel} /> */}
					<br />
				</Show>
				<div style="display:flex; align-items:center">
					<h3 style="font-size:1rem; margin-top:8px;flex:1">
						{getThreads().length} {threadFilter()} threads
					</h3>
					{
						/*
					TODO: thread ordering
					<div>
						<h3 class="dim">order by</h3>
						<Dropdown
							style="max-width:150px"
							options={[
								{ item: "new", label: "newest threads first" },
								{
									item: "activity",
									label: "recently active threads",
								},
								{ item: "reactions:+1", label: "most +1 reactions" },
								{ item: "random", label: "random ordering" },
								{ item: "hot", label: "mystery algorithm 1" },
								{ item: "hot2", label: "mystery algorithm 2" },
							]}
						/>
					</div>
					*/
					}
					{
						/*
					TODO: gallery view
					<div>
						<h3 class="dim">view as</h3>
						<Dropdown
							style="max-width:150px"
							options={[
								{ item: "list", label: "list" },
								{ item: "gallery", label: "gallery" },
							]}
						/>
					</div>
				*/
					}
					<div class="thread-filter">
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
						<Show when={perms.has("ThreadRemove")}>
							<button
								classList={{ selected: threadFilter() === "removed" }}
								onClick={[setThreadFilter, "removed"]}
							>
								removed
							</button>
						</Show>
					</div>
					<button
						class="primary"
						style="margin-left: 8px;border-radius:4px"
						onClick={() => createThread(room_id())}
					>
						create thread
					</button>
				</div>
				<ul>
					<For each={getThreads()}>
						{(thread) => (
							<li>
								<article class="thread menu-thread" data-thread-id={thread.id}>
									<header onClick={() => setThreadId(thread.id)}>
										<div class="top">
											<div class="icon"></div>
											<div class="spacer">{thread.name}</div>
											<div class="time">
												Created <Time date={getTimestampFromUUID(thread.id)} />
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
														thread.last_version_id ?? thread.id,
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
			<Show when={threadId()}>
				{(tid) => <Forum2View channel={api.channels.cache.get(tid())!} />}
			</Show>
		</div>
	);
};

export const Forum2View = (props: { channel: Channel }) => {
	const api = useApi();
	const comments = api.messages.listReplies(
		() => props.channel.id,
		() => undefined,
		() => ({ depth: 8, breadth: 9999 }),
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
			if (node.message.reply_id && commentMap.has(node.message.reply_id)) {
				commentMap.get(node.message.reply_id)!.children.push(node);
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

	return (
		<div class="forum2-thread">
			<div class="main">
				<div>
					<h2>{props.channel.name}</h2>
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
				<div style="display:flex;flex-direction:column;gap:2px">
					{/* TODO: support markdown */}
					<textarea style="padding: 2px 4px" placeholder="add a comment...">
					</textarea>
					<menu style="align-self:end">
						<button class="big primary">send</button>
					</menu>
				</div>
			</div>
			<div class="aside">
				<h3 class="dim">thread info</h3>
				<ul>
					<li>tags: [foo] [bar] [baz]</li>
					<li>
						comments: [{comments()?.items.length ?? 0}] comments
						([{commentTree().length}] threads/top level comments)
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
		<div class="forum">
			<div>forum</div>
			<ul>
				<For each={props.commentTree}>
					{(node) => (
						<li class="toplevel">
							<Comment
								collapsed={props.collapsed}
								channel={props.channel}
								node={node}
							/>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

const contentToHtml = new WeakMap();

const Comment = (
	props: {
		collapsed: ReactiveSet<string>;
		channel: Channel;
		node: CommentNode;
	},
) => {
	const message = () => props.node.message;
	const children = () => props.node.children;
	const api = useApi();

	const collapsed = () => props.collapsed.has(message().id);

	const countAllChildren = (node: CommentNode): number => {
		return node.children.length +
			node.children.reduce((sum, child) => sum + countAllChildren(child), 0);
	};

	function getHtml(): string {
		const cached = contentToHtml.get(message());
		if (cached) return cached;

		const content = message().content ?? "";

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
		contentToHtml.set(message(), twemojified);
		return twemojified;
	}

	let contentEl!: HTMLDivElement;

	createEffect(() => {
		if (contentEl) {
			hydrateMentions(contentEl, props.channel);
		}
	});

	createEffect(() => {
		getHtml();
		import("highlight.js").then(({ default: hljs }) => {
			if (!contentEl) return;
			for (const el of [...contentEl.querySelectorAll("pre")]) {
				el.dataset.highlighted = "";
				hljs.highlightElement(el);
			}
		});
	});

	return (
		<div
			class="comment menu-message"
			data-message-id={message().id}
			classList={{ collapsed: collapsed() }}
		>
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
						{message().content
							? api.stripMarkdownAndResolveMentions(
								message().content!,
								message().channel_id,
							)
							: "(no content)"}
					</div>
				</Show>
			</header>
			<Show when={!collapsed()}>
				<div class="content markdown" ref={contentEl!} innerHTML={getHtml()}>
				</div>
				<Show when={message().attachments?.length}>
					<ul class="attachments">
						<For each={message().attachments}>
							{(att) => <AttachmentView media={att} />}
						</For>
					</ul>
				</Show>
				<Show when={message().reactions?.length}>
					<Reactions message={message()} />
				</Show>
				<menu>
					<button onClick={() => alert("todo")}>
						reply
					</button>
				</menu>
				<Show when={children().length > 0}>
					<ul class="children">
						<For each={children()}>
							{(child) => (
								<li>
									<Comment
										collapsed={props.collapsed}
										channel={props.channel}
										node={child}
									/>
								</li>
							)}
						</For>
					</ul>
				</Show>
			</Show>
		</div>
	);
};

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
