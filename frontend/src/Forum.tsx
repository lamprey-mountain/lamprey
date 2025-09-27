import { getTimestampFromUUID, Message, Thread } from "sdk";
import { createResource, For, Show } from "solid-js";
import { useCtx } from "./context";
import { useApi } from "./api";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "./Time";

export const Forum = (props: { thread: Thread }) => {
	const api = useApi();
	const [comments] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/thread/{thread_id}/reply",
			{
				params: { path: { thread_id: props.thread.id } },
			},
		);
		console.log(data);
		return data;
	});
	const collapsed = new ReactiveSet<string>();

	return (
		<div class="forum">
			<div>forum</div>
			<ul>
				<For each={comments()?.items}>
					{(c) => (
						<li class="toplevel">
							<Comment
								collapsed={collapsed}
								thread={props.thread}
								message={c}
							/>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

const Comment = (
	props: { collapsed: ReactiveSet<string>; thread: Thread; message: Message },
) => {
	const api = useApi();

	const collapsed = () => props.collapsed.has(props.message.id);

	const [children] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/thread/{thread_id}/reply/{message_id}",
			{
				params: {
					path: { thread_id: props.thread.id, message_id: props.message.id },
					query: { depth: 2 },
				},
			},
		);
		console.log(props.message, data);
		return data;
	});

	const countChildren = () => children()?.total ?? 0;

	return (
		<div class="comment" classList={{ collapsed: collapsed() }}>
			<header>
				<button
					class="collapse"
					onClick={() =>
						collapsed()
							? props.collapsed.delete(props.message.id)
							: props.collapsed.add(props.message.id)}
				>
					{collapsed() ? "+" : "-"}
				</button>
				<Show when={collapsed()}>
					<span class="childCount dim">[{countChildren()}]</span>
				</Show>
				<div class="author">
					author
				</div>
				<Time date={getTimestampFromUUID(props.message.id)} />
				<Show when={collapsed()}>
					<div class="summary">
						{props.message.content ?? "(no content)"}
					</div>
				</Show>
			</header>
			<Show when={!collapsed()}>
				<div class="content">
					{props.message.content ?? "(no content)"}
				</div>
				<menu>
					<button onClick={() => alert("todo")}>
						reply
					</button>
				</menu>
				<Show when={children()}>
					<ul class="children">
						<For each={children()?.items.slice(1) ?? []}>
							{(child) => (
								<li>
									<Comment
										collapsed={props.collapsed}
										thread={props.thread}
										message={child}
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
// {#if !collapsed}
// {/if}

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
