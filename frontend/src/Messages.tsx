// import { Tooltip } from "./Atoms.tsx";
import { Match, Show, Switch } from "solid-js";
import { MessageT, ThreadT } from "./types.ts";
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
		| { type: "time-split", date: Date }
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
			return (
				<li
					class="message has-menu"
					classList={{
						"selected": item.message.id === ctx.thread_reply_id.get(thread.id),
						// "context": a()?.type === "context" &&
						// 	item.message.id === a()!.message_id,
					}}
					data-message-id={item.message.id}
				>
					<MessageView message={item.message} />
				</li>
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
					<time datetime={item.date.toISOString()}>{item.date.toDateString()}</time>
					<hr />
				</li>
			);
		}
	}
}
