// TODO: implement

import { createResource, For, Match, Switch } from "solid-js";
import type { MessageVersion, PaginationResponseMessage } from "ts-sdk";
import { useApi } from "@/api";
import { Time } from "@/atoms/Time";
import { MessageView, UserDisplayName } from "../chat/Message";
import { MessageToolbarProvider } from "../chat/message-toolbar-context";

export type ThreadActivityProps = {
	channel_id: string;
	// comments: { items: Array<{ id: string }> } | undefined;
	// commentTree: Array<unknown>;
};

export const ThreadActivity = (props: ThreadActivityProps) => {
	const api = useApi();

	const channel = api.channels.use(() => props.channel_id);

	const [activity] = createResource(
		() => props.channel_id,
		async (channel_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/channel/{channel_id}/activity",
				{ params: { path: { channel_id } } },
			);
			if (!data) return;
			return data as unknown as PaginationResponseMessage;
		},
	);

	// const comments = () => props.comments;
	// const commentTree = () => props.commentTree;

	// <li>
	// 	comments: [{comments()?.items.length ?? 0}] comments ([
	// 	{commentTree().length}] threads/top level comments)
	// </li>

	// <li>[user] renamed to [name]</li>
	// <li>[user] added tag to [name]</li>
	// <li>[user] pinned [a message]</li>
	// <li>[user] added [member] to the thread</li>
	// <li>[user] removed [member] from the thread</li>
	// <li>mentioned in [thread]</li>

	// <ul>
	// 	<li>
	// 		last comment: <a href="#">some time ago</a>
	// 	</li>
	// </ul>
	// <br />
	// <h3 class="dim">thread log</h3>

	return (
		<aside class="forum2-activity">
			<h3 class="dim">thread activity</h3>
			<ul>
				<For each={activity()?.items ?? []}>
					{(item) => {
						function matchesType<T extends string>(
							ty: T,
						): (MessageVersion & { type: T }) | false {
							if (item.latest_version.type === ty) {
								return item.latest_version as MessageVersion & { type: T };
							} else {
								return false;
							}
						}

						// <Match when={matchesType("ChannelMoved")} >
						// 	<MessageToolbarProvider>
						// 		<MessageView message={item} separate />
						// 	</MessageToolbarProvider>
						// </Match>

						// TODO: show time
						// TODO: show system channel icon
						// <Time
						// 	date={new Date(item.created_at)}
						// 	animGroup="message-ts"
						// 	class="full"
						// 	format="full"
						// />

						return (
							<li class="item">
								<Switch>
									<Match when={matchesType("ChannelRename")}>
										{(v) => {
											return (
												<>
													<UserDisplayName
														user_id={item.author_id}
														room_id={channel()?.room_id ?? undefined}
														onClick
													/>{" "}
													renamed this thread from{" "}
													<b class="bright">{v().name_old}</b> to{" "}
													<b class="bright">{v().name_new}</b>
												</>
											);
										}}
									</Match>
									<Match when={matchesType("ChannelMoved")}>
										{(v) => {
											const parentOld = api.channels.use(
												() => v().parent_id_old ?? undefined,
											);
											const parentNew = api.channels.use(
												() => v().parent_id_new ?? undefined,
											);
											return (
												<div>
													channel moved #{parentOld()?.name ?? "loading..."} to
													#{parentNew()?.name ?? "loading..."}
												</div>
											);
										}}
									</Match>
									<Match when={matchesType("MessagePinned")}>
										{(v) => <div>message pinned</div>}
									</Match>
									<Match when={matchesType("MemberAdd")}>
										{(v) => <div>member added</div>}
									</Match>
									<Match when={matchesType("MemberRemove")}>
										{(v) => <div>member removed</div>}
									</Match>
									<Match when={true}>
										unknown action {item.latest_version.type}
									</Match>
								</Switch>
							</li>
						);
					}}
				</For>
			</ul>
		</aside>
	);
};
