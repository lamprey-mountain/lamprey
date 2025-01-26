// import { Tooltip } from "./Atoms.tsx";
import { Show } from "solid-js";
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
		| { type: "time-split" }
		| {
			type: "message";
			message: MessageT;
			separate: boolean;
			is_local: boolean;
		}
	);

function renderTimelineItem(thread: ThreadT, item: TimelineItemT) {
	switch (item.type) {
		case "message": {
			const ctx = useCtx();
			return (
				<li
					class="message"
					classList={{
						"selected":
							item.message.id === ctx.data.thread_state[thread.id]?.reply_id,
					}}
					data-message-id={item.message.id}
				>
					<MessageView message={item.message} is_local={item.is_local} />
				</li>
			);
		}
		case "info": {
			return (
				<li class="header">
					<header>
						<h1>{thread.name}</h1>
						<p>
							{thread.description ?? "(no description)"} /
							<Show when={thread.is_closed}>(archived)</Show>
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
	}
}

export const TimelineItem = (
	props: { thread: ThreadT; item: TimelineItemT },
) => {
	return <>{renderTimelineItem(props.thread, props.item)}</>;
};
