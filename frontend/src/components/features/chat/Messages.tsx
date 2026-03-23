import { createMemo, Match, Show, Switch } from "solid-js";
import { useApi, useMessages2, useRoomMembers2 } from "../../../api.tsx";
import type { MessageT, ThreadT } from "../../../types.ts";
import { useCtx } from "../../../context.ts";
import { md } from "../../../markdown_utils.tsx";
import { MessageView } from "./Message.tsx";
import { useChannel } from "../../../channelctx.tsx";
import { Message, UserWithRelationship } from "sdk";
import {
	getMessageOverrideName,
	getMsgTs as get_msg_ts,
} from "../../../utils/general";
import { ChannelIcon } from "../../../User.tsx";

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
			class: string;
		}
	);

export const TimelineItem = (props: {
	thread: ThreadT;
	item: TimelineItemT;
	currentUser: () => UserWithRelationship | undefined;
}) => {
	switch (props.item.type) {
		case "message": {
			const ctx = useCtx();
			const roomMembersService = useRoomMembers2();
			const [ch] = useChannel()!;
			const room_member = roomMembersService.useMember(
				() => props.thread.room_id ?? "",
				() => props.currentUser()?.id ?? "",
			);

			const is_mentioned = createMemo(() => {
				const me = props.currentUser();
				if (!me || props.item.type !== "message") return false;
				const mentions = (props.item.message as any).mentions as any;
				if (!mentions) return false;

				if (mentions.users.some((u: any) => u.id === me.id)) {
					return true;
				}
				if (mentions.everyone) {
					return true;
				}
				const rm = room_member();
				if (rm) {
					for (const role of mentions.roles) {
						if (rm.roles.some((r: any) => r.id === (role as any).id)) {
							return true;
						}
					}
				}
				return false;
			});

			const isSelected = createMemo(() => {
				if (props.item.type !== "message") return false;
				const selected = ch.selectedMessages;
				return selected?.includes(props.item.message.id) ?? false;
			});

			return (
				<li
					class="message"
					classList={{
						selected: props.item.message.id === ch.reply_id,
						"message-selected": isSelected(),
						mentioned: is_mentioned(),
					}}
				>
					<MessageView
						message={props.item.message}
						separate={props.item.separate}
					/>
				</li>
			);
		}
		case "info": {
			return (
				<li class="header">
					<header>
						<Show when={false}>
							<div style="display:flex;align-items:center;gap:4px;">
								<div style="background:red;border-radius:50%;display:grid;place-items:center;height:32px;width:32px;">
									<ChannelIcon
										style="height:24px;width:24px"
										channel={props.thread}
									/>
								</div>
								<h1>{props.thread.name}</h1>
							</div>
						</Show>
						<h1>{props.thread.name}</h1>
						<p>
							This is the start of {props.thread.name}.{" "}
							<span
								class="markdown"
								innerHTML={md(props.thread.description ?? "") as string}
							>
							</span>
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
			return <li class="spacer mini"></li>;
		}
		case "divider": {
			return (
				<li
					class="divider"
					classList={{ unread: props.item.unread, time: !!props.item.date }}
				>
					<Show when={props.item.unread}>
						<div class="new">new</div>
					</Show>
					<hr />
					<Show when={props.item.date}>
						{(d) => (
							<>
								<time datetime={d().toISOString()}>
									{d().toDateString()}
								</time>
								<hr />
							</>
						)}
					</Show>
					<Show when={props.item.unread}>
						<div class="new hidden">new</div>
					</Show>
				</li>
			);
		}
		default:
			return null;
	}
};

/** @deprecated use TimelineItem component instead */
export function renderTimelineItem(
	thread: ThreadT,
	item: TimelineItemT,
	currentUser: () => UserWithRelationship | undefined,
) {
	return <TimelineItem thread={thread} item={item} currentUser={currentUser} />;
}

type RenderTimelineParams = {
	items: Array<Message>;
	read_marker_id: string | null;
	has_before: boolean;
	has_after: boolean;
};

export function renderTimeline(
	{ items, read_marker_id, has_before, has_after, cache }:
		& RenderTimelineParams
		& { cache?: Map<string, TimelineItemT> },
): Array<TimelineItemT> {
	const newItems: Array<TimelineItemT> = [];
	if (has_before) {
		newItems.push({
			type: "spacer",
			id: "spacer-top",
		});
	} else {
		newItems.push({
			type: "info",
			id: "thread-header",
			header: true,
		});
	}
	for (let i = 0; i < items.length; i++) {
		const msg = items[i];
		const prev = items[i - 1] as Message | undefined;
		const markerTime = prev &&
			get_msg_ts(msg).getDay() !== get_msg_ts(prev).getDay();
		const markerUnread = prev?.id === read_marker_id;
		if (markerTime || markerUnread) {
			newItems.push({
				type: "divider",
				id: `divider-${msg.id}-${markerUnread}`,
				date: markerTime ? get_msg_ts(msg) : undefined,
				unread: markerUnread,
			});
		}

		const separate = prev ? shouldSplit(msg, prev) : true;
		const cacheKey = `${msg.id}:${separate}`;
		let item = cache?.get(cacheKey);
		if (!item) {
			item = {
				type: "message",
				id: msg.id,
				message: msg as any,
				separate,
				class: separate ? "separate" : "",
			};
			cache?.set(cacheKey, item);
		}
		newItems.push(item);
	}
	if (has_after) {
		newItems.push({
			type: "spacer",
			id: "spacer-bottom",
		});
	} else {
		newItems.push({
			type: "spacer-mini",
			id: "spacer-bottom-mini",
		});
	}
	return newItems;
}

const shouldSplitMemo = new WeakMap();
function shouldSplit(a: Message, b: Message) {
	const s1 = shouldSplitMemo.get(a);
	if (s1) return s1;
	const s2 = shouldSplitInner(a, b);
	shouldSplitMemo.set(a, s2);
	return s2;
}

function shouldSplitInner(a: Message, b: Message) {
	if (a.latest_version.type !== "DefaultMarkdown") return true;
	if (b.latest_version.type !== "DefaultMarkdown") return true;
	if (a.author_id !== b.author_id) return true;
	if (getMessageOverrideName(a) !== getMessageOverrideName(b)) return true;
	const ts_a = get_msg_ts(a);
	const ts_b = get_msg_ts(b);
	if (+ts_a - +ts_b > 1000 * 60 * 5) return true;
	return false;
}
