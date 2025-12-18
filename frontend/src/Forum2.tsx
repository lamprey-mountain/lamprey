import { Channel, getTimestampFromUUID, Message } from "sdk";
import { createMemo, createResource, createSignal, For, Show } from "solid-js";
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

	return (
		<div class="room-home" style="display:flex">
			<div style="display:flex;flex-direction:column;border:solid red 1px">
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
			<div style="border:solid blue 1px">
				<Show when={threadId()}>
					{(tid) => <Forum2View channel={api.channels.cache.get(tid())!} />}
				</Show>
			</div>
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
		<div style="display:flex;">
			<div style="flex:1">
				<div>
					<h2>{props.channel.name}</h2>
				</div>
				<div style="display:flex">
					<div style="flex:1">
						n comments
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
			<div style="width:144px">
				<h3 class="dim">topic info</h3>
				<ul>
					<li>tags: [foo] [bar] [baz]</li>
					<li>comments: [n] comments ([m] threads/top level comments)</li>
					<li>
						last comment: <a href="#">some time ago</a>
					</li>
				</ul>
				<br />
				<h3 class="dim">topic log</h3>
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

const Comment = (
	props: {
		collapsed: ReactiveSet<string>;
		channel: Channel;
		node: CommentNode;
	},
) => {
	const message = () => props.node.message;
	const children = () => props.node.children;

	const collapsed = () => props.collapsed.has(message().id);

	const countAllChildren = (node: CommentNode): number => {
		return node.children.length +
			node.children.reduce((sum, child) => sum + countAllChildren(child), 0);
	};

	return (
		<div class="comment" classList={{ collapsed: collapsed() }}>
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
						{message().content ?? "(no content)"}
					</div>
				</Show>
			</header>
			<Show when={!collapsed()}>
				<div class="content">
					{message().content ?? "(no content)"}
				</div>
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