import { useMessages, type MessageListAnchor } from "@/api";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { logger } from "@/utils/logger";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { createSignal, createEffect, on, createMemo, For } from "solid-js";
import { ChatProps } from "./Chat";
import { type TimelineItemT, renderTimeline, TimelineItem } from "./Messages";
import { useSync } from "@/hooks/useSync";
import { MessageSync } from "ts-sdk";
import { MessageToolbarMount } from "./MessageToolbar";

const RELEVANT_SYNC_EVENTS = new Set<MessageSync["type"]>([
	"MessageCreate",
	"MessageUpdate",
	"MessageDelete",
	"MessageDeleteBulk",
	"MessageRemove",
	"MessageRestore",
]);

// type TimelineCommand = { type: "anchor" }
// 	| { type: "highlight" };

export const Timeline = (props: ChatProps) => {
	const log = logger.for("timeline");

	// TEMP: passing current user
	const currentUser = useCurrentUser();

	const [wrapperEl, setWrapperEl] = createSignal<HTMLDivElement | null>(null);

	// TEMP: timeline/message calculation code should be made more explicit
	const messagesService = useMessages();
	const [channelState, setChannelState] = useChannel()!;

	const read_marker_id = () => channelState.read_marker_id;

	const anchor = (): MessageListAnchor => {
		const a = channelState.anchor;
		if (a) return a;

		const r = read_marker_id();
		const last_id = props.channel.last_version_id;

		// If channel has unread messages (and isn't just the very last message)
		if (r && last_id && r !== last_id) {
			return { type: "context", limit: 50, message_id: r };
		}

		return { type: "backwards", limit: 50 };
	};

	// TODO: explicit message/timeline fetching and handling
	const messages = messagesService.useList(() => props.channel.id, anchor);

	useSync((msg) => {
		if (msg.type === "MessageCreate") {
			// msg.message.channel_id;
		}

		if (RELEVANT_SYNC_EVENTS.has(msg.type)) {
			// TODO
		}
	});

	const timelineCache = new Map<string, TimelineItemT>();
	createEffect(
		on(
			() => props.channel.id,
			() => {
				timelineCache.clear();
			},
		),
	);

	const items = createMemo(() => {
		const m = messages();
		const rid = read_marker_id();
		if (!m?.items) return [];

		return renderTimeline({
			items: m.items,
			has_after: m.has_forward,
			has_before: m.has_backwards,
			read_marker_id: rid ?? null,
			cache: timelineCache,
		});
	});

	let scrollEl: HTMLDivElement | null = null;
	const virtualizer = createVirtualizer({
		get count() {
			return items().length;
		},
		getScrollElement: () => scrollEl,
		estimateSize: () => 100, // TODO
		overscan: 5,
	});

	// pagination
	createEffect(
		on(
			() => virtualizer.scrollOffset,
			(pos) => {
				console.log("scrolled", pos);
			},
			{ defer: true },
		),
	);

	const handleScroll = (e: Event) => {
		console.log(e);
		// TODO
	};

	const handleScrollEnd = (e: Event) => {
		console.log(e);
		// TODO: paginate
	};

	// TODO: restore scroll position
	return (
		<div
			class="timeline"
			role="log"
			ref={scrollEl!}
			onScroll={handleScroll}
			onScrollEnd={handleScrollEnd}
		>
			<div
				class="timeline-items"
				style={{ height: `${virtualizer.getTotalSize()}px` }}
			>
				<For each={virtualizer.getVirtualItems()}>
					{(row) => {
						const item = () => items()[row.index];
						let el: HTMLDivElement | null = null;

						createEffect(() => {
							item();
							virtualizer.measureElement(el);
						});

						return (
							<div
								class="timeline-item"
								data-index={row.index}
								style={{ transform: `translateY(${row.start}px)` }}
								ref={el!}
							>
								<TimelineItem
									thread={props.channel}
									item={item()}
									currentUser={currentUser}
								/>
							</div>
						);
					}}
				</For>
				<MessageToolbarMount />
			</div>
		</div>
	);
};

// TODO
// export const TimelineItem2 = (props: {
// 	channel: ChannelT;
// 	item: TimelineItemT;
// }) => {
// 	return (
// 		<Switch>
// 			<Match when={props.item.type === "message" && props.item}>
// 				{(item) => (
// 					<MessageView message={item().message} separate={item().separate} />
// 				)}
// 			</Match>
// 		</Switch>
// 	);
// };
