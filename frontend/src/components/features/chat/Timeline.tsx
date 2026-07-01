import { useMessages, type MessageListAnchor } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { throttle } from "@solid-primitives/scheduled";
import { useReadTracking } from "@/contexts/read-tracking";
import { logger } from "@/utils/logger";
import { createVirtualizer } from "@tanstack/solid-virtual";
import {
	createEffect,
	on,
	For,
	Show,
	onCleanup,
	Switch,
	Match,
} from "solid-js";
import { ChatProps } from "./Chat";
import { renderTimeline, TimelineItem, TimelineItemT } from "./Messages";
import { MessageToolbarMount } from "./MessageToolbar";
import { MessageRange } from "@/api/services/MessagesService";
import { createStore, reconcile } from "solid-js/store";
import { TimelineState, useTimeline } from "./timeline-context";
import { Queue } from "@/utils/queue";
import { ChannelT } from "@/types";
import { MessageView } from "./Message";

// TODO: add logging
const log = logger.for("timeline");

const PAGINATE_THRESHOLD = 200;
const OVERSCAN = 20;

// how should i handle state management with Timeline? should i make Chat.tsx swap out TimelineState and store it in contexts? or make Timeline reactively respond to the current channel?

type TimelineTask =
	| { type: "SET_ANCHOR"; anchor: MessageListAnchor }
	| { type: "CALLBACK"; fn: () => void }
	// | { type: "SCROLL" }
	| { type: "HIGHLIGHT" };

export const Timeline = (props: ChatProps) => {
	const messagesService = useMessages();
	const [timeline, updateTimeline] = useTimeline();
	const currentUser = useCurrentUser(); // TEMP: compat

	const fetchMessages = async (a: MessageListAnchor) => {
		if (timeline.loading) return;
		updateTimeline("loading", true);
		try {
			const range = await messagesService.fetchSlice(props.channel.id, a);
			updateTimeline("messages", range);
		} finally {
			updateTimeline("loading", false);
		}
	};

	const queue = new Queue(async (task: TimelineTask) => {
		log.debug("execute task", task);
		switch (task.type) {
			case "SET_ANCHOR": {
				updateTimeline("anchor", task.anchor);
				await fetchMessages(task.anchor);
				break;
			}
			case "CALLBACK": {
				task.fn();
				break;
			}
			case "HIGHLIGHT": {
				// TODO
				break;
			}
		}
	});

	// fetch initial messages
	queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor });

	// reactively refetch messages whenever a channel's message version is bumped
	// TODO: merge into queue handler
	createEffect(
		on(
			() => messagesService._versions.get(props.channel.id),
			() => {
				// queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor });
				// const m = timeline.messages;
				// if (m) fetchMessages(timeline.anchor);
			},
			{ defer: true },
		),
	);

	// reactively update timeline when messages are received
	// FIXME: rerender when channelState.read_marker_id updates
	createEffect(() => {
		const m = timeline.messages;
		const rendered = m?.items
			? renderTimeline({
					items: m.items,
					has_after: m.has_forward,
					has_before: m.has_backwards,
					read_marker_id: timeline.last_read_message_id ?? null,
				})
			: [];
		log.debug("update timeline", rendered);
		updateTimeline("items", reconcile(rendered));
	});

	let scrollEl = null as HTMLDivElement | null;
	const virtualizer = createVirtualizer({
		get count() {
			return timeline.items.length;
		},
		anchorTo: "end",
		getScrollElement: () => scrollEl,
		estimateSize: () => 80, // TODO: more accurate size estimation
		overscan: OVERSCAN,
	});

	timeline.commands.on("scrollBy", (data) => {
		const newOffset = (virtualizer.scrollOffset ?? 0) + data.px;
		virtualizer.scrollToOffset(newOffset, {
			behavior: data.smooth ? "smooth" : "auto",
		});
	});

	timeline.commands.on("jumpToBottom", (data) => {
		queue.push(
			{ type: "SET_ANCHOR", anchor: { type: "backwards", limit: 50 } },
			{
				type: "CALLBACK",
				fn: () => {
					virtualizer.scrollToEnd({
						behavior: data.smooth ? "smooth" : "auto",
					});
				},
			},
		);
	});

	timeline.commands.on("jumpToTop", (data) => {
		queue.push(
			{ type: "SET_ANCHOR", anchor: { type: "forwards", limit: 50 } },
			{
				type: "CALLBACK",
				fn: () => {
					virtualizer.scrollToOffset(0, {
						behavior: data.smooth ? "smooth" : "auto",
					});
				},
			},
		);
	});

	timeline.commands.on("jumpToMessage", (data) => {
		queue.push(
			{
				type: "SET_ANCHOR",
				anchor: {
					type: "context",
					limit: 50,
					message_id: data.message_id,
				},
			},
			{
				type: "CALLBACK",
				fn: () => {
					const idx = timeline.items.findIndex(
						(x) => x.type === "message" && x.message.id === data.message_id,
					);
					if (idx !== -1) {
						virtualizer.scrollToIndex(idx, {
							align: "center",
							behavior: data.smooth ? "smooth" : "auto",
						});
					} else {
						log.warn("couldn't find message after scrolling", data);
					}
				},
			},
		);
	});

	timeline.commands.listen((e) => {
		log.debug(e.name, "command", e.details ?? null);
	});

	timeline.events.listen((e) => {
		log.debug(e.name, "event", e.details ?? null);
	});

	const calculateSliceLen = () =>
		Math.max(50, Math.ceil(globalThis.innerHeight / 20) * 3);
	const calculatePaginateLen = () => Math.floor(calculateSliceLen() / 3);

	const handleScrollEnd = () => {
		if (timeline.loading) return;
		const el = scrollEl;
		if (!el) return;

		const msgs = timeline.messages;
		if (!msgs) return;

		// PERF: use IntersectionObserver; using .scrollHeight/.scrollTop forces a sync layout recalc
		const atTop = el.scrollTop < PAGINATE_THRESHOLD;
		const atBottom =
			el.scrollHeight - el.scrollTop - el.clientHeight < PAGINATE_THRESHOLD;

		if (atTop) {
			if (msgs.has_backwards) {
				const len = calculatePaginateLen();
				const idx = Math.min(len, msgs.items.length - 1);
				const anchor: MessageListAnchor = {
					type: "backwards",
					limit: calculateSliceLen(),
					message_id: msgs.items[idx]?.id,
				};
				queue.push({ type: "SET_ANCHOR", anchor });
			} else {
				timeline.events.emit("scrollTop");
			}
		} else if (atBottom) {
			if (msgs.has_forward) {
				const len = calculatePaginateLen();
				const idx = Math.max(0, msgs.items.length - len);
				const anchor: MessageListAnchor = {
					type: "forwards",
					limit: calculateSliceLen(),
					message_id: msgs.items[idx]?.id,
				};
				queue.push({ type: "SET_ANCHOR", anchor });
			} else {
				timeline.events.emit("scrollBottom");
			}
		}

		setPos();
	};

	// TODO: merge with jumpToMessage command
	// createEffect(
	// 	on(
	// 		() => timeline.items,
	// 		(items) => {
	// 			if (items.length === 0) return;
	// 			// HACK: delay to allow items to render/measure
	// 			setTimeout(() => {
	// 				const a = timeline.anchor;
	// 				if (a.type === "context") {
	// 					const idx = items.findIndex(
	// 						(x) => x.type === "message" && x.message.id === a.message_id,
	// 					);
	// 					if (idx !== -1) {
	// 						const offset = virtualizer.getOffsetForIndex(idx);
	// 						if (offset !== null) {
	// 							const targetOffset = offset - (scrollEl?.clientHeight ?? 0) / 2;
	// 							const distance = Math.abs(
	// 								virtualizer.scrollOffset - targetOffset,
	// 							);
	// 							const shouldSmooth =
	// 								distance < (scrollEl?.clientHeight ?? 0) * 3;
	// 							virtualizer.scrollToOffset(targetOffset, {
	// 								behavior: shouldSmooth ? "smooth" : "auto",
	// 							});
	// 						}
	// 					} else {
	// 						// fallback
	// 						virtualizer.scrollToOffset(virtualizer.getTotalSize());
	// 					}
	// 				} else {
	// 					const pos = timeline.scroll_pos;
	// 					if (pos === undefined || pos === -1) {
	// 						virtualizer.scrollToOffset(virtualizer.getTotalSize());
	// 					} else {
	// 						virtualizer.scrollToOffset(pos);
	// 					}
	// 				}
	// 			}, 0);
	// 		},
	// 	),
	// );

	// PERF: is constantly setting pos too heavy?
	const setPos = throttle(() => {
		if (!scrollEl) return;
		// use virtualizer offset instead of scrollTop since it's more accurate
		// NOTE: verify this claim
		const offset = virtualizer.scrollOffset ?? -1; // NOTE: what do i do when scrollOffset is null?
		const atBottom =
			scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < 50;
		const pos = atBottom ? -1 : offset;
		timeline.events.emit("scrollPosition", pos);
		updateTimeline("scroll_pos", pos);
	}, 300);

	// TODO: remove? and remove has_forward?
	// // reactively update has_forward
	// createEffect(() => {
	// 	const m = timeline.messages;
	// 	if (m) {
	// 		updateTimeline("has_forward", m.has_forward);
	// 	}
	// });

	return (
		<>
			<div
				class="timeline"
				role="log"
				ref={scrollEl!}
				onScrollEnd={handleScrollEnd}
			>
				<div
					class="timeline-items"
					style={{
						height: `${virtualizer.getTotalSize()}px`,
						width: "100%",
						position: "relative",
					}}
				>
					<For each={virtualizer.getVirtualItems()}>
						{(row) => {
							const item = () => timeline.items[row.index];
							let el: HTMLDivElement | null = null;

							createEffect(() => {
								item();
								virtualizer.measureElement(el);
							});

							return (
								<div
									class="timeline-item"
									data-index={row.index}
									style={{
										transform: `translateY(${row.start}px)`,
									}}
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
		</>
	);
};

export const TimelineItem2 = (props: {
	channel: ChannelT;
	item: TimelineItemT;
}) => {
	// TODO: rename spacer, spacer-mini?
	// TODO: remove spacer-mini2
	// TODO: render message skeletons for spacer

	return (
		<li
			classList={{
				mentioned: false,
				flume: false,
				selected: false,
				"reply-target": false,
			}}
		>
			<Switch>
				<Match when={props.item.type === "message" && props.item}>
					{(item) => (
						<MessageView message={item().message} separate={item().separate} />
					)}
				</Match>
				<Match when={props.item.type === "info" && props.item}>
					{(item) => <header>todo</header>}
				</Match>
				<Match when={props.item.type === "divider" && props.item}>
					{(item) => <div class="timeline-divider">todo</div>}
				</Match>
				<Match when={props.item.type === "spacer"}>
					<div class="spacer"></div>
				</Match>
				<Match when={props.item.type === "spacer-mini"}>
					<div class="spacer-mini"></div>
				</Match>
			</Switch>
		</li>
	);
};
