// import { Tooltip } from "./Atoms.tsx";
import { createMemo, Match, Show, Switch } from "solid-js";
import { useApi } from "./api.tsx";
import type { MessageT, ThreadT } from "./types.ts";
import { useCtx } from "./context.ts";
import { MessageView } from "./Message.tsx";

// const Tooltip = (props: ParentProps<{ tip: any, attrs: any }>) => props.children;

export type TimelineItemT =
	& { id: string; class?: string }
	& (
		| { type: "info"; header: boolean }
		| { type: "editor" }
		| { type: "spacer" }
		| { type: "spacer-mini" }
		| { type: "spacer-mini2" }
		| { type: "unread-marker" }
		| { type: "time-split"; date: Date }
		| {
			type: "message";
			message: MessageT;
			separate: boolean;
		}
	);

export function renderTimelineItem(thread: ThreadT, item: TimelineItemT) {
	switch (item.type) {
		case "message": {
			const ctx = useCtx();
			const api = useApi();
			const message = createMemo(() => api.messages.cache.get(item.message.id));
			return (
				<Show when={message()}>
					{(message) => (
						<li
							class="message"
							classList={{
								"selected": message().id === ctx.thread_reply_id.get(thread.id),
							}}
						>
							<MessageView message={message()} separate={item.separate} />
						</li>
					)}
				</Show>
			);
		}
		case "info": {
			return (
				<li class="header">
					<header>
						<h1>{thread.name}</h1>
						<p>
							{thread.description ?? "(no description)"}
							{" / "}
							<Switch>
								<Match when={thread.state === "Archived"}>(archived)</Match>
								<Match when={thread.state === "Deleted"}>(deleted)</Match>
							</Switch>
						</p>
					</header>
				</li>
			);
		}
		case "spacer": {
			return <li class="spacer" style="min-height:800px;flex:1"></li>;
		}
		case "spacer-mini2": {
			return <li class="spacer" style="min-height:8rem;flex:1"></li>;
		}
		case "spacer-mini": {
			return <li class="spacer" style="min-height:2rem"></li>;
		}
		case "unread-marker": {
			return (
				<li class="unread-marker">
					<div class="content">new messages</div>
				</li>
			);
		}
		case "time-split": {
			return (
				<li class="time-split">
					<hr />
					<time datetime={item.date.toISOString()}>
						{item.date.toDateString()}
					</time>
					<hr />
				</li>
			);
		}
	}
}
