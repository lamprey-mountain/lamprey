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
	createSignal,
} from "solid-js";
import { ChatProps } from "./Chat";
import { renderTimeline, TimelineItem, TimelineItemT } from "./Messages";
import { highlight } from "./util";
import { MessageToolbarMount } from "./MessageToolbar";
import { createStore, reconcile } from "solid-js/store";
import { TimelineState, useTimeline } from "./timeline-context";
import { Queue } from "@/utils/queue";
import { ChannelT } from "@/types";
import { MessageView } from "./Message";

const log = logger.for("timeline");

const PAGINATE_THRESHOLD = 200;
const OVERSCAN = 20;

type TimelineTask =
	| { type: "SET_ANCHOR"; anchor: MessageListAnchor }
	| { type: "HIGHLIGHT"; message_id: string }
	| { type: "SCROLL_TOP"; smooth?: boolean }
	| { type: "SCROLL_BOTTOM"; smooth?: boolean }
	| { type: "SCROLL_MESSAGE"; message_id: string; smooth?: boolean };

export const Timeline = (props: ChatProps) => {
	const messagesService = useMessages();
	const [timeline, updateTimeline] = useTimeline();
	const currentUser = useCurrentUser(); // TEMP: compat
	const [pendingHighlight, setPendingHighlight] = createSignal<string | null>(
		null,
	);
	let highlightTimer: number | undefined;

	const resetHighlight = () => {
		setPendingHighlight(null);
		clearTimeout(highlightTimer);
	};

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

	let scrollEl = null as HTMLDivElement | null;
	const virtualizer = createVirtualizer({
		get count() {
			return timeline.items.length;
		},
		anchorTo: "end",
		followOnAppend: true,
		scrollEndThreshold: 80,
		getScrollElement: () => scrollEl,
		estimateSize: () => 80, // TODO: more accurate size estimation
		overscan: OVERSCAN,
		getItemKey(index) {
			return timeline.items[index]?.id;
		},
	});

	const queue = new Queue(async (task: TimelineTask) => {
		log.debug("execute task", task);
		switch (task.type) {
			case "SET_ANCHOR": {
				resetHighlight();
				updateTimeline("anchor", task.anchor);
				await fetchMessages(task.anchor);

				// update timeline
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
				await new Promise<void>((r) => queueMicrotask(r));
				break;
			}
			case "HIGHLIGHT": {
				clearTimeout(highlightTimer);
				setPendingHighlight(task.message_id);
				highlightTimer = window.setTimeout(
					() => setPendingHighlight(null),
					3000, // TODO: extract 3000 into a const
				);
				break;
			}
			case "SCROLL_TOP": {
				virtualizer.scrollToOffset(0, {
					behavior: task.smooth ? "smooth" : "auto",
				});
				break;
			}
			case "SCROLL_BOTTOM": {
				virtualizer.scrollToEnd({
					behavior: task.smooth ? "smooth" : "auto",
				});
				break;
			}
			case "SCROLL_MESSAGE": {
				const idx = timeline.items.findIndex(
					(x) => x.type === "message" && x.message.id === task.message_id,
				);
				if (idx !== -1) {
					virtualizer.scrollToIndex(idx, {
						align: "center",
						behavior: task.smooth ? "smooth" : "auto",
					});
				} else {
					log.warn("couldn't find message for SCROLL_MESSAGE", task);
				}
				break;
			}
		}
	});

	// fetch initial messages
	const init = () => {
		const a = timeline.anchor;
		const scroll: TimelineTask =
			a.type === "context"
				? { type: "SCROLL_MESSAGE", message_id: a.message_id }
				: a.type === "backwards"
					? { type: "SCROLL_BOTTOM" }
					: { type: "SCROLL_TOP" };

		queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor }, scroll);
	};

	init();

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

	// FIXME: rerender when channelState.read_marker_id updates

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
				type: "SCROLL_BOTTOM",
				smooth: data.smooth,
			},
		);
	});

	timeline.commands.on("jumpToTop", (data) => {
		queue.push(
			{ type: "SET_ANCHOR", anchor: { type: "forwards", limit: 50 } },
			{
				type: "SCROLL_TOP",
				smooth: data.smooth,
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
				type: "SCROLL_MESSAGE",
				message_id: data.message_id,
				smooth: data.smooth,
			},
		);
		if (data.highlight) {
			queue.push({
				type: "HIGHLIGHT",
				message_id: data.message_id,
			});
		}
	});

	timeline.commands.on("ackMessage", (data) => {
		updateTimeline("last_read_message_id", data.message_id);

		// re-render to update read markers
		// PERF: only re-render if last_read_message_id is inside the current message range
		// m?.contains(data.message_id)
		const m = timeline.messages;
		if (m) {
			const rendered = renderTimeline({
				items: m.items,
				has_after: m.has_forward,
				has_before: m.has_backwards,
				read_marker_id: data.message_id,
			});
			updateTimeline("items", reconcile(rendered));
		}
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
		resetHighlight();
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
								const it = item();
								virtualizer.measureElement(el);

								if (
									it?.type === "message" &&
									it.message.id === pendingHighlight() &&
									el
								) {
									highlight(el);
									resetHighlight();
								}
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
