import { Channel, getTimestampFromUUID, Message } from "sdk";
import { For, Show } from "solid-js";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "./Time";
import { Author } from "./Message";

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
