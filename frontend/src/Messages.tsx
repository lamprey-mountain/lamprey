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
			const self = () => api.users.cache.get("@self");
			const room_member = createMemo(() => {
				const me = self();
				if (!me || !thread.room_id) return null;
				return api.room_members.fetch(() => thread.room_id, () => me.id)();
			});

			const is_mentioned = () => {
				const me = self();
				if (!me) return false;

				if (item.message.mentions.users.includes(me.id)) {
					return true;
				}
				if (
					item.message.mentions.everyone
				) {
					return true;
				}
				const rm = room_member();
				if (rm) {
					for (const role_id of item.message.mentions.roles) {
						if (rm.roles.some((r) => r.id === role_id)) {
							return true;
						}
					}
				}
				return false;
			};
			const isSelected = () => {
				const selected = ctx.selectedMessages.get(thread.id);
				return selected?.includes(item.message.id) ?? false;
			};
			return (
				<li
					class="message"
					classList={{
						selected: item.message.id === ctx.channel_reply_id.get(thread.id),
						"message-selected": isSelected(),
						mentioned: is_mentioned(),
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
