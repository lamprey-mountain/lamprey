import type { Message, UserWithRelationship } from "sdk";
import { createMemo, Match, Show, Switch } from "solid-js";
import { createMutable } from "solid-js/store";
import { useFlumes, useRoomMembers } from "@/api";
import { ChannelIcon } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { md } from "@/lib/markdown";
import type { MessageT, ThreadT } from "@/types";
import {
	getMsgTs as get_msg_ts,
	getMessageOverrideName,
} from "@/utils/general";
import { MessageView } from "./Message.tsx";
import { shouldSplit } from "./util.ts";

export type TimelineItemT = { id: string; class?: string; nonce?: string } & (
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

const TimelineMessageItem = (props: {
	thread: ThreadT;
	item: Extract<TimelineItemT, { type: "message" }>;
	currentUser: () => UserWithRelationship | undefined;
}) => {
	const roomMembersService = useRoomMembers();
	const [ch] = useChannel()!;
	const flumes = useFlumes();
	const room_member = roomMembersService.useMember(
		() => props.thread.room_id ?? "",
		() => props.currentUser()?.id ?? "",
	);

	// TODO: move is mentioned calculation into a hook/function
	const is_mentioned = createMemo(() => {
		const me = props.currentUser();
		if (!me) return false;
		const mentions = (props.item.message as Message).mentions as
			| {
					users?: Array<{ id: string }>;
					everyone?: boolean;
					roles?: Array<{ id: string }>;
			  }
			| undefined;
		if (!mentions) return false;

		if (mentions.users?.some((u) => u.id === me.id)) {
			return true;
		}
		if (mentions.everyone) {
			return true;
		}
		const rm = room_member();
		if (rm && mentions.roles) {
			for (const role of mentions.roles) {
				if (rm.roles.some((r) => r === role.id)) {
					return true;
				}
			}
		}
		return false;
	});

	const isSelected = createMemo(() => {
		const selected = ch.selectedMessages;
		return selected?.includes(props.item.message.id) ?? false;
	});

	const hasFlume = createMemo(() => {
		return flumes.cache.has(props.item.id);
	});

	return (
		<li
			classList={{
				selected: props.item.message.id === ch.reply_id,
				"message-selected": isSelected(),
				mentioned: is_mentioned(),
				flume: hasFlume(),
			}}
		>
			<MessageView
				message={props.item.message}
				separate={props.item.separate}
			/>
		</li>
	);
};

export const TimelineItem = (props: {
	thread: ThreadT;
	item: TimelineItemT;
	currentUser: () => UserWithRelationship | undefined;
}) => {
	return (
		<Switch>
			<Match when={props.item.type === "message"}>
				<TimelineMessageItem
					{...props}
					item={props.item as Extract<TimelineItemT, { type: "message" }>}
				/>
			</Match>
			<Match when={props.item.type === "info"}>
				<li class="timeline-header">
					<header>
						<Show when={false}>
							{/* TODO: add channel icon? */}
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
							></span>
						</p>
					</header>
				</li>
			</Match>
			<Match when={props.item.type === "spacer"}>
				<li class="spacer" style="min-height:800px;flex:1"></li>
			</Match>
			<Match when={props.item.type === "spacer-mini2"}>
				<li class="spacer" style="min-height:8rem;flex:1"></li>
			</Match>
			<Match when={props.item.type === "spacer-mini"}>
				<li class="spacer mini"></li>
			</Match>
			<Match when={props.item.type === "divider" && props.item}>
				{(item) => (
					<li
						class="divider timeline-divider"
						classList={{ unread: item().unread, time: !!item().date }}
					>
						<Show when={item().unread}>
							<div class="new">new</div>
						</Show>
						<hr />
						<Show when={item().date}>
							{(d) => (
								<>
									<time datetime={d().toISOString()}>{d().toDateString()}</time>
									<hr />
								</>
							)}
						</Show>
						<Show when={item().unread}>
							<div class="new hidden">new</div>
						</Show>
					</li>
				)}
			</Match>
		</Switch>
	);
};

type RenderTimelineParams = {
	items: Array<Message>;
	read_marker_id: string | null;
	has_before: boolean;
	has_after: boolean;
};

// TODO: message skeletons when loading, has_before, has_after
export function renderTimeline({
	items,
	read_marker_id,
	has_before,
	has_after,
	cache,
}: RenderTimelineParams & {
	cache?: Map<string, TimelineItemT>;
}): Array<TimelineItemT> {
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
		const markerTime =
			prev && get_msg_ts(msg).getDay() !== get_msg_ts(prev).getDay();
		const markerUnread = prev?.id === read_marker_id;
		if (markerTime || markerUnread) {
			const id = `divider-${msg.id}-${markerUnread}`;
			let item: TimelineItemT | undefined = cache?.get(id);
			if (!item || item.type !== "divider") {
				const divider: TimelineItemT = createMutable({
					type: "divider",
					id,
					date: markerTime ? get_msg_ts(msg) : undefined,
					unread: markerUnread,
				});
				item = divider;
				cache?.set(id, item);
			} else {
				item.date = markerTime ? get_msg_ts(msg) : undefined;
				item.unread = markerUnread;
			}
			newItems.push(item);
		}

		const separate = prev ? shouldSplit(msg, prev) : true;
		const cacheKey = msg.id;
		let item: TimelineItemT | undefined = cache?.get(cacheKey);

		if (!item || item.type !== "message" || item.message !== msg) {
			item = createMutable({
				type: "message" as const,
				id: msg.id,
				nonce: msg.nonce,
				message: msg as any,
				separate,
				get class() {
					return this.separate ? "separate" : "";
				},
			}) as TimelineItemT;
			cache?.set(cacheKey, item);
		} else {
			// Update separate without changing object reference
			item.separate = separate;
			item.message = msg as any;
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
