import { createResource, For, Match, Switch } from "solid-js";
import type { MessageVersion, PaginationResponseMessage } from "ts-sdk";
import { useApi } from "@/api";
import { Time } from "@/atoms/Time";
import { MessageView, UserDisplayName } from "../chat/Message";
import { MessageToolbarProvider } from "../chat/message-toolbar-context";

export type ThreadActivityProps = {
	channel_id: string;
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

	// TODO: list most recently created comments?

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

						// TODO: show time
						// TODO: show system message icon
						// <Time
						// 	date={new Date(item.created_at)}
						// 	animGroup="message-ts"
						// 	class="full"
						// 	format="full"
						// />

						// TODO: finish rendering logic, maybe reuse parts of SystemMessage

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
										{(_v) => <>message pinned</>}
									</Match>
									<Match when={matchesType("MemberAdd")}>
										{(_v) => <>member added</>}
									</Match>
									<Match when={matchesType("MemberRemove")}>
										{(_v) => <>member removed</>}
									</Match>
									<Match when={matchesType("ChannelPingback")}>
										{(_v) => <>channel mentioned</>}
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
