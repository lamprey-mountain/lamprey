import { useMessages, type MessageListAnchor } from "@/api";
import { TimelineController, useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { throttle } from "@solid-primitives/scheduled";
import { useReadTracking } from "@/contexts/read-tracking";
import { logger } from "@/utils/logger";
import { createVirtualizer } from "@tanstack/solid-virtual";
import {
	createSignal,
	createEffect,
	on,
	createMemo,
	For,
	Show,
} from "solid-js";
import { ChatProps } from "./Chat";
import { renderTimeline, TimelineItem, TimelineItemT } from "./Messages";
import { MessageToolbarMount } from "./MessageToolbar";
import { MessageRange } from "@/api/services/MessagesService";
import { createEmitter, Emitter } from "@solid-primitives/event-bus";
import { createStore, reconcile } from "solid-js/store";

// TODO: add logging
const log = logger.for("timeline");

const PAGINATE_THRESHOLD = 200;
const OVERSCAN = 20;

export type TimelineEvents = {
	/** scroll position updated */
	scrollPosition: number;

	/** scrolled to top of channel */
	scrollTop: void;

	/** scrolled to bottom of channel */
	scrollBottom: void;
};

// TODO: use this for Timeline
// save TimelineState in channel context, swap it out when switching channels
export type TimelineState = {
	controller: TimelineController;
	events: Emitter<TimelineEvents>;
}

export type TimelineProps = {
	tl: TimelineState;
}

export function createTimeline(): TimelineState {
	// TODO

	return {
		// TODO
	}
}

// how should i handle state management with Timeline? should i make Chat.tsx swap out TimelineState and store it in contexts? or make Timeline reactively respond to the current channel?

// TODO: remove highlight from ChannelState

export const Timeline = (props: ChatProps) => {
	const messagesService = useMessages();
	const [channelState, setChannelState] = useChannel()!;
	const currentUser = useCurrentUser();
	const { markChannelRead } = useReadTracking();

	const getInitialAnchor = (): MessageListAnchor => {
		const readMarker = channelState.read_marker_id;
		const hasReadMarker =
			readMarker && readMarker !== props.channel.last_version_id;
		if (hasReadMarker) {
			return { type: "context", limit: 50, message_id: readMarker };
		} else {
			return { type: "backwards", limit: 50 };
		}
	};

	const [anchor, setAnchor] = createSignal<MessageListAnchor>(
		getInitialAnchor(),
	);

	// TODO: add event emitter
	// const events = createEmitter<TimelineEvents>();
	// events.emit("scrollPosition", 123);
	// events.emit("scrollTop");
	// events.on("scrollPosition", (n) => {});

	const [messages, setMessages] = createSignal<MessageRange | null>(null);
	const [loading, setLoading] = createSignal(false);

	const fetchMessages = async (a: MessageListAnchor) => {
		if (loading()) return;
		setLoading(true);
		try {
			const range = await messagesService.fetchSlice(props.channel.id, a);
			setMessages(range);
		} finally {
			setLoading(false);
		}
	};

	// reactively refetch messages whenever anchor changes
	createEffect(
		on(anchor, (a) => {
			fetchMessages(a);
		}),
	);

	// reactively refetch messages whenever a channel's message version is bumped
	createEffect(
		on(
			() => messagesService._versions.get(props.channel.id),
			() => {
				const m = messages();
				if (m) fetchMessages(anchor());
			},
			{ defer: true },
		),
	);

	// reset when channel changes
	createEffect(
		on(
			() => props.channel.id,
			(channelId) => {
				// TODO: reset
			},
			{ defer: true },
		),
	);

	const [timeline, updateTimeline] = createStore([] as TimelineItemT[]);
	const items = () => timeline;

	// reactively update timeline when messages are received
	// FIXME: rerender when channelState.read_marker_id updates
	createEffect(() => {
		const m = messages();
		const rendered = m?.items ? renderTimeline({
			items: m.items,
			has_after: m.has_forward,
			has_before: m.has_backwards,
			read_marker_id: channelState.read_marker_id ?? null,
		}) : [];
		log.debug("update timeline", rendered);
		updateTimeline(reconcile(rendered));
	});

	let scrollEl = null as HTMLDivElement | null;
	const virtualizer = createVirtualizer({
		get count() {
			return items().length;
		},
		anchorTo: "end",
		getScrollElement: () => scrollEl,
		estimateSize: () => 80, // TODO: more accurate size estimation
		overscan: OVERSCAN,
	});

	// TODO: move outside of timeline
	const markRead = throttle(() => {
		const version_id = props.channel.last_version_id;
		if (version_id !== props.channel.last_read_id) {
			markChannelRead(props.channel.id, version_id, false, true);
		}
	}, 300);

	const calculateSliceLen = () =>
		Math.max(50, Math.ceil(globalThis.innerHeight / 20) * 3);
	const calculatePaginateLen = () => Math.floor(calculateSliceLen() / 3);

	const handleScrollEnd = () => {
		if (loading()) return;
		const el = scrollEl;
		if (!el) return;

		const msgs = messages();
		if (!msgs) return;

		// PERF: use IntersectionObserver; using .scrollHeight/.scrollTop forces a sync layout recalc
		const atTop = el.scrollTop < PAGINATE_THRESHOLD;
		const atBottom =
			el.scrollHeight - el.scrollTop - el.clientHeight < PAGINATE_THRESHOLD;

		if (atTop && msgs.has_backwards) {
			const len = calculatePaginateLen();
			const idx = Math.min(len, msgs.items.length - 1);
			setAnchor({
				type: "backwards",
				limit: calculateSliceLen(),
				message_id: msgs.items[idx]?.id,
			});
		} else if (atBottom && msgs.has_forward) {
			const len = calculatePaginateLen();
			const idx = Math.max(0, msgs.items.length - len);
			setAnchor({
				type: "forwards",
				limit: calculateSliceLen(),
				message_id: msgs.items[idx]?.id,
			});
		} else if (atBottom && !msgs.has_forward) {
			markRead();
		}

		setPos();
	};

	createEffect(
		on(items, (items) => {
			if (items.length === 0) return;
			// HACK: delay to allow items to render/measure
			setTimeout(() => {
				const a = anchor();
				if (a.type === "context") {
					const idx = items.findIndex(
						(x) => x.type === "message" && x.message.id === a.message_id,
					);
					if (idx !== -1) {
						const offset = virtualizer.getOffsetForIndex(idx);
						if (offset !== null) {
							const targetOffset = offset - (scrollEl?.clientHeight ?? 0) / 2;
							const distance = Math.abs(
								virtualizer.scrollOffset - targetOffset,
							);
							const shouldSmooth = distance < (scrollEl?.clientHeight ?? 0) * 3;
							virtualizer.scrollToOffset(targetOffset, {
								behavior: shouldSmooth ? "smooth" : "auto",
							});
						}
					} else {
						// fallback
						virtualizer.scrollToOffset(virtualizer.getTotalSize());
					}
				} else {
					const pos = channelState.scroll_pos;
					if (pos === undefined || pos === -1) {
						virtualizer.scrollToOffset(virtualizer.getTotalSize());
					} else {
						virtualizer.scrollToOffset(pos);
					}
				}
			}, 0);
		}),
	);

	// PERF: is constantly setting pos too heavy?
	const setPos = throttle(() => {
		if (!scrollEl) return;
		console.log("setPos")
		// use virtualizer offset instead of scrollTop since it's more accurate
		const offset = virtualizer.scrollOffset;
		const atBottom =
			scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < 50;
		const pos = atBottom ? -1 : offset;
		setChannelState("scroll_pos", pos);
	}, 300);

	// reactively update has_forward
	createEffect(() => {
		const m = messages();
		if (m) {
			setChannelState("has_forward", m.has_forward);
		}
	});

	// register timeline controller
	setChannelState("timeline", {
		jumpToEnd(markAsRead = false) {
			setAnchor({ type: "backwards", limit: calculateSliceLen() });
			if (markAsRead) markRead();
			queueMicrotask(() =>
				virtualizer.scrollToOffset(virtualizer.getTotalSize()),
			);
		},
		jumpToMessage(message_id: string, highlight = false) {
			setAnchor({ type: "context", limit: 50, message_id });
			if (highlight) setChannelState("highlight", message_id);
		},
		scrollBy(px: number, smooth: boolean) {
			virtualizer.scrollBy(px, { behavior: smooth ? "smooth" : "auto" });
		},
		isAtBottom() {
			if (!scrollEl) return true;
			const bottom = scrollEl.scrollHeight - scrollEl.clientHeight;
			return virtualizer.scrollOffset >= bottom - 64;
		},
		scrollToBottom(smooth = false) {
			virtualizer.scrollToEnd({ behavior: smooth ? "smooth" : "auto" });
		},
	});

	return (
		<>
			{/* TODO: move jump to unread/mark as read buttons into Chat.tsx */}
			<Show
				when={
					messages()?.has_forward &&
					props.channel.last_version_id !== channelState.read_marker_id
				}
			>
				<div class="new-messages">
					<button
						type="button"
						class="jump-read"
						onClick={() =>
							channelState.timeline.jumpToMessage(
								channelState.read_marker_id!,
								true,
							)
						}
					>
						jump to unread
					</button>
					<button type="button" class="mark-read" onClick={markRead}>
						mark as read
					</button>
				</div>
			</Show>
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
									style={{
										position: "absolute",
										top: 0,
										left: 0,
										width: "100%",
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

// TODO: rewrite TimelineItem
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

// TODO: extract timeline logic out of component
// export type TimelineProps = {
// 	tl: TimelineController;
// };
//
// export const createTimeline = (): TimelineController => {
// 	return { ... }
// }
