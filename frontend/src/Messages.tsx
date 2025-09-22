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
		| { type: "divider"; unread: boolean; date?: Date }
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
			return (
				<li
					class="message"
					classList={{
						"selected": item.message.id === ctx.thread_reply_id.get(thread.id),
					}}
				>
					<MessageView message={item.message} separate={item.separate} />
				</li>
			);
		}
		case "info": {
			return (
				<li class="header">
					<header>
						<h1>{thread.name}</h1>
						<p>This is the start of {thread.name}. {thread.description}</p>
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
			return <li class="spacer mini"></li>;
		}
		case "divider": {
			return (
				<li
					class="divider"
					classList={{ unread: item.unread, time: !!item.date }}
				>
					<Show when={item.unread}>
						<div class="new">new</div>
					</Show>
					<hr />
					<Show when={item.date}>
						{(d) => (
							<>
								<time datetime={d().toISOString()}>
									{d().toDateString()}
								</time>
								<hr />
							</>
						)}
					</Show>
					<Show when={item.unread}>
						<div class="new hidden">new</div>
					</Show>
				</li>
			);
		}
	}
}
