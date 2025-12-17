import { Channel, getTimestampFromUUID, Message } from "sdk";
import { createMemo, For, Show } from "solid-js";
import { useApi } from "./api";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "./Time";

interface CommentNode {
	message: Message;
	children: CommentNode[];
}

export const Forum2Comments = (props: { channel: Channel }) => {
	const api = useApi();
	const comments = api.messages.listReplies(
		() => props.channel.id,
		() => undefined,
		() => ({ depth: 8, breadth: 9999 }),
	);

	const commentTree = createMemo(() => {
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

	return (
		<div class="forum">
			<div>forum</div>
			<ul>
				<For each={commentTree()}>
					{(node) => (
						<li class="toplevel">
							<Comment
								collapsed={collapsed}
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
				<div class="author">
					author
				</div>
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
