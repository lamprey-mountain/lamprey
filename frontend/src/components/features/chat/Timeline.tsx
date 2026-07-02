import {
	useFlumes,
	useMessages,
	useRoomMembers,
	type MessageListAnchor,
} from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { throttle } from "@solid-primitives/scheduled";
import { useReadTracking } from "@/contexts/read-tracking";
import { logger } from "@/utils/logger";
import {
	createEffect,
	on,
	For,
	Show,
	onCleanup,
	Switch,
	Match,
	createSignal,
	batch,
	onMount,
	createMemo,
} from "solid-js";
import { ChatProps } from "./Chat";
import {
	estimateSize,
	highlight,
	renderTimeline2,
	TimelineItemT2,
} from "./util";
import { MessageToolbarMount } from "./MessageToolbar";
import { createStore, reconcile } from "solid-js/store";
import { TimelineState, useTimeline } from "./timeline-context";
import { Queue } from "@/utils/queue";
import { ChannelT, MessageT } from "@/types";
import { MessageView } from "./Message";
import { MessageSkeletons } from "./MessageSkeleton";
import { MessageRange } from "@/api/services/MessagesService";
import { Key } from "@solid-primitives/keyed";
import { useSync } from "@/hooks/useSync";
import { md } from "@/lib/markdown";
import { useChannel } from "@/contexts/mod";
import { ReactiveMap } from "@solid-primitives/map";
import { Layout, TimelineTask, VirtualItem } from "./timeline-utils";

const log = logger.for("timeline");

const PAGINATE_THRESHOLD = 800;
const OVERSCAN = 20;
const HIGHLIGHT_TIMEOUT = 3000;
const STICKY_THRESHOLD = 80;

// FIXME: flash of empty/blank timeline when paginating

// FIXME: deduplicate SET_ANCHOR tasks
// this happens when sending a message (_versions bump, js scroll event)
// then after the remote echo is received this happens again
// maybe it would be better to make duplicate SET_ANCHORs a no-op

export const Timeline = (props: ChatProps) => {
	const messages = useMessages();
	const timeline = useTimeline();

	const [scrollEl, setScrollEl] = createSignal(null as HTMLDivElement | null);
	const [viewportHeight, setViewportHeight] = createSignal(0);
	const [totalSize, setTotalSize] = createSignal(0);

	// NOTE: reuse ro instead of creating a new ResizeObserver
	const containerRo = new ResizeObserver((entries) => {
		for (const entry of entries) {
			setViewportHeight(entry.contentRect.height);
		}
	});

	// NOTE: use onMount?
	createEffect(() => {
		const el = scrollEl();
		if (el) {
			containerRo.observe(el);
		}
		onCleanup(() => {
			if (el) containerRo.unobserve(el);
		});
	});

	// TODO: encapsulate highlight logic?
	// const highlight = { timer: null as null | number, pending: ..., reset() { ... } };
	const [pendingHighlight, setPendingHighlight] = createSignal<string | null>(
		null,
	);
	let highlightTimer: number | undefined;

	const resetHighlight = () => {
		setPendingHighlight(null);
		clearTimeout(highlightTimer);
	};

	// TODO: extract virtualization logic into a separate file?
	// maybe write a hook for this?

	// TODO: move to timeline context
	const measurements = new Map<string, number>();

	// TODO(?): maybe move this to timeline context?
	const [visibleRows, setVisibleRows] = createStore([] as VirtualItem[]);

	/** get or estimate the height of an item */
	const getSize = (item: TimelineItemT2) => {
		let s = measurements.get(item.key);

		if (!s) {
			s = estimateSize(item);
			measurements.set(item.key, s);
		}

		return s;
	};

	let layout: Layout;
	let range: Array<VirtualItem>;

	const calculateLayout = (): Layout => {
		const items = timeline.items;

		// calculate sizes and offsets
		const sizes = new Float64Array(items.length);
		const offsets = new Float64Array(items.length);
		let totalSize = 0;
		for (let i = 0; i < items.length; i++) {
			const s = getSize(items[i]);
			sizes[i] = s;
			offsets[i] = totalSize;
			totalSize += s;
		}

		return { sizes, offsets, totalSize };
	};

	const calculateRange = (layout: Layout): Array<VirtualItem> => {
		const items = timeline.items;
		const { sizes, offsets } = layout;

		// calculate visible items
		const st = timeline.scrollTop;
		const vh = viewportHeight();

		// PERF: use binary search
		let start = offsets.findIndex((o) => o > st);
		start = start === -1 ? items.length - 1 : start - 1;

		let end = offsets.findIndex((o) => o > st + vh);
		end = end === -1 ? items.length - 1 : end - 1;

		start = Math.max(0, start - OVERSCAN);
		end = Math.min(items.length - 1, end + OVERSCAN);

		const visible: Array<VirtualItem> = [];
		for (let i = start; i <= end; i++) {
			visible.push({
				index: i,
				item: items[i],
				offset: offsets[i],
				size: sizes[i],
			});
		}

		return visible;
	};

	const ro = new ResizeObserver((entries) => {
		let delta = 0;

		for (const entry of entries) {
			const key = (entry.target as HTMLElement).dataset.key;
			if (!key) continue;

			const heightOld = measurements.get(key);
			if (!heightOld) return; // TODO: handle this

			const heightNew =
				entry.borderBoxSize?.[0]?.blockSize ?? entry.contentRect.height;
			if (heightOld !== heightNew) {
				delta += heightNew - heightOld;
				measurements.set(key, heightNew);
			}
		}

		scrollEl()?.scrollBy({ top: delta, behavior: "instant" });

		// runs before the browser paints
		requestAnimationFrame(() => {
			// TODO: update scroll position, visible items, virtual item offsets
			// handle elements being resized above the viewport and below the viewport
			// maybe overflow-anchor can keep stuff stable
			// maybe i'd need to disable overflow-anchor during SET_ANCHOR
			// scrollEl()?.scrollBy({ top: -delta, behavior: "instant" });
		});

		queue.push({
			type: "RESIZE",
			delta,
		});
	});

	// onMount(() => {
	// 	const el = scrollEl();
	//  if (!el) log warning
	//
	// 	ro.observe(el);
	//  onCleanup(() => {});
	// });

	onCleanup(() => {
		ro.disconnect();
		containerRo.disconnect();
	});

	const queue = new Queue(async (task: TimelineTask) => {
		log.debug("execute task", task);

		switch (task.type) {
			case "SET_ANCHOR": {
				// 1. fetch messages
				// 2. recalculate/update layout
				// 3. recalculate/update range
				// 4. wait for solidjs to update the dom
				// 5. stabilize scroll position

				resetHighlight();

				const el = scrollEl();

				// TODO: show skeletons while fetching messages?
				const messageRange = await messages.fetchSlice(
					props.channel.id,
					task.anchor,
				);

				// update timeline
				const rendered = renderTimeline2(
					messageRange,
					timeline.readMarkerId ?? null,
				);

				log.debug("update timeline", rendered);

				// pick a reference item: prefer the anchor's message_id, else first visible item
				// const refKey =
				// 	("message_id" in task.anchor &&
				// 		`message-${task.anchor.message_id}`) ||
				// 	range()[0]?.item.key;
				const refKey =
					("message_id" in task.anchor &&
						`message-${task.anchor.message_id}`) ??
					range?.find((i) => i.item.type === "message")?.item.key;
				// FIXME: anchor backwards with no message id should probably use range().lastIndexOf
				// or maybe i should pick the center item if possible?

				// PERF: use binary search
				const refOffsetOld = refKey
					? layout?.offsets[timeline.items.findIndex((x) => x.key === refKey)]
					: undefined;

				timeline.anchor = task.anchor;
				timeline.messages = messageRange;
				timeline.itemsSignal[1](rendered); // TODO: don't make this a signal

				layout = calculateLayout();
				range = calculateRange(layout);
				setTotalSize(layout.totalSize);
				setVisibleRows(reconcile(range, { key: "key" }));

				// wait for solidjs to update dom
				await new Promise<void>((r) => queueMicrotask(r));

				// wait for layout but before paint
				// await new Promise((r) => requestAnimationFrame(r));

				if (el && refOffsetOld) {
					// PERF: use binary search
					const newIdx = timeline.items.findIndex((x) => x.key === refKey);
					if (newIdx !== -1) {
						const refOffsetNew = calculateLayout().offsets[newIdx];
						log.debug("stabilize scroll", {
							refKey,
							refOffsetOld,
							refOffsetNew,
						});
						el.scrollTop += refOffsetNew - refOffsetOld;
					}
				}

				break;
			}
			case "RESIZE": {
				// TODO: move scroll stabilization logic here?
				layout = calculateLayout();
				range = calculateRange(layout);
				setTotalSize(layout.totalSize);
				setVisibleRows(reconcile(range, { key: "key" }));
				break;
			}
			case "HIGHLIGHT": {
				clearTimeout(highlightTimer);
				setPendingHighlight(task.message_id);
				highlightTimer = window.setTimeout(
					() => setPendingHighlight(null),
					HIGHLIGHT_TIMEOUT,
				);
				break;
			}
			case "SCROLL_TOP": {
				scrollEl()?.scrollTo({
					top: 0,
					behavior: task.smooth ? "smooth" : "auto",
				});
				break;
			}
			case "SCROLL_BOTTOM": {
				const el = scrollEl();
				if (el) {
					el.scrollTo({
						top: el.scrollHeight,
						behavior: task.smooth ? "smooth" : "auto",
					});
				}
				break;
			}
			case "SCROLL_MESSAGE": {
				// PERF: binary search, check timeline.messages (MessageRange)
				const idx = timeline.items.findIndex(
					(x) => x.type === "message" && x.message.id === task.message_id,
				);
				if (idx !== -1) {
					const { offsets, sizes } = calculateLayout();
					// align message to center of view
					const vh = viewportHeight();
					const itemOffset = offsets[idx];
					const itemSize = sizes[idx];
					const targetTop = Math.max(0, itemOffset - vh / 2 + itemSize / 2);

					scrollEl()?.scrollTo({
						top: targetTop,
						behavior: task.smooth ? "smooth" : "auto",
					});
				} else {
					log.warn("couldn't find message for SCROLL_MESSAGE", task);
				}
				break;
			}
		}
	});

	// refetch messages whenever a channel's message version is bumped
	createEffect(
		on(
			() => messages._versions.get(props.channel.id),
			() => {
				console.log("AAA messages._versions bumped", queue.active);
				if (queue.active) return;

				// update anchor to rerender
				queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor });

				// attempt to autoscroll
				const el = scrollEl();
				if (
					el &&
					el.scrollHeight - el.scrollTop - el.clientHeight < STICKY_THRESHOLD
				) {
					queue.push({ type: "SCROLL_BOTTOM" });
				}
			},
			{ defer: true },
		),
	);

	// NOTE: maybe i could use this? but using _versions is easier
	// useSync((msg) => {
	// 	if (msg.type === "MessageCreate") {
	// 		if (msg.message.channel_id === props.channel.id) {
	// 			// maybe update timeline, rerender
	// 		}
	// 	}
	// });

	// TODO: use scroll element height instead of globalThis
	const calculateSliceLen = () =>
		Math.max(50, Math.ceil(globalThis.innerHeight / 20) * 3);
	const calculatePaginateLen = () => Math.floor(calculateSliceLen() / 3);

	// debounce/throttle handleScroll to only recalculate once per
	// requestAnimationFrame, since theres no point in doing more calculations that the
	// browser cant render
	let ticking = false;
	const handleScroll = () => {
		if (!ticking) {
			requestAnimationFrame(() => {
				const el = scrollEl();
				if (el) {
					timeline.scrollTop = el.scrollTop;
					range = calculateRange(layout);
					setVisibleRows(reconcile(range, { key: "key" }));
				}
				ticking = false;
			});
			ticking = true;
		}
	};

	// handle pagination on scrollend event. IntersectionObserver wouldn't help
	// performance here. using it would prevent forcing a reflow, but after scrollend
	// everything is already laid out.
	// TODO: fetch before scroll end so stuff starts loading before user hits a wall
	// maybe do use IntersectionObserver anyways, not for perf but as an impl detail?
	const handleScrollEnd = () => {
		resetHighlight();
		if (queue.active) return;
		const el = scrollEl();
		if (!el) return;

		const msgs = timeline.messages;
		if (!msgs) return;

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

		timeline.scrollTop = el.scrollTop;
		timeline.events.emit("scrollPosition", el.scrollTop);
	};

	// ===== handle commands =====

	timeline.commands.on("scrollBy", (data) => {
		scrollEl()?.scrollBy({
			top: data.px,
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
		timeline.readMarkerId = data.message_id;

		if (!timeline.messages?.contains(data.message_id)) return;

		// TODO: rerender timeline (in a queue task?)
		// const m = timeline.messages;
		// if (m) {
		// 	const rendered = renderTimeline({
		// 		items: m.items,
		// 		has_after: m.has_forward,
		// 		has_before: m.has_backwards,
		// 		read_marker_id: data.message_id,
		// 	});
		// 	updateTimeline("items", reconcile(rendered));
		// }
	});

	timeline.commands.listen((e) => {
		log.debug("command", e.name, e.details ?? "(null)");
	});

	timeline.events.listen((e) => {
		log.debug("event", e.name, e.details ?? "(null)");
	});

	// ===== other stuff =====

	// fetch initial messages
	const init = () => {
		const a = timeline.anchor;

		// TODO: use a second queue.push instead of this horrible nested ternary
		const scroll: TimelineTask =
			a.type === "context"
				? { type: "SCROLL_MESSAGE", message_id: a.message_id }
				: a.type === "backwards"
					? { type: "SCROLL_BOTTOM" }
					: { type: "SCROLL_TOP" };

		queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor }, scroll);

		// TODO: restore scroll position
		// timeline.scrollTop;
	};

	init();

	// PERF: use { passive: true } for onscroll event?

	return (
		<div
			class="timeline"
			role="log"
			ref={setScrollEl}
			onScrollEnd={handleScrollEnd}
			onScroll={handleScroll}
		>
			<div
				class="timeline-items"
				style={{
					height: `${totalSize()}px`,
				}}
			>
				<Key each={visibleRows} by={(row) => row.item.key}>
					{(row) => {
						const handleRef = (ref: HTMLDivElement) => {
							ro.observe(ref);
							onCleanup(() => ro.unobserve(ref));

							createEffect(() => {
								const item = row().item;
								if (
									item.type === "message" &&
									item.message.id === pendingHighlight()
								) {
									highlight(ref.children[0]);
									resetHighlight();
								}
							});
						};

						return (
							<div
								class="timeline-item"
								data-index={row().index}
								data-key={row().item.key}
								style={{
									transform: `translateY(${row().offset}px)`,
								}}
								ref={handleRef}
							>
								<TimelineItem2 channel={props.channel} item={row().item} />
							</div>
						);
					}}
				</Key>
				<MessageToolbarMount />
			</div>
		</div>
	);
};

export const TimelineItem2 = (props: {
	channel: ChannelT;
	item: TimelineItemT2;
}) => {
	const roomMembersService = useRoomMembers();
	const [ch] = useChannel()!;
	const flumes = useFlumes();
	const currentUser = useCurrentUser();
	const room_member = roomMembersService.useMember(
		() => props.channel.id,
		() => currentUser()?.id ?? "",
	);

	// TODO: create a hook for this
	// PERF: cache globally (maybe store directly in message object?)
	const is_mentioned = createMemo(() => {
		if (props.item.type !== "message") return false;
		const me = currentUser();
		if (!me) return false;
		const mentions = (props.item.message as any).mentions as
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
		if (props.item.type !== "message") return false;
		const selected = ch.selectedMessages;
		return selected?.includes(props.item.message.id) ?? false;
	});

	const hasFlume = createMemo(() => {
		if (props.item.type !== "message") return false;
		return flumes.cache.has(props.item.key);
	});

	return (
		<li
			classList={{
				mentioned: is_mentioned(),
				flume: hasFlume(),
				selected: isSelected(),
				"reply-target":
					props.item.type === "message" &&
					props.item.message.id === ch.reply_id,
			}}
		>
			<Switch>
				<Match when={props.item.type === "message" && props.item}>
					{(item) => (
						<MessageView message={item().message} separate={item().separate} />
					)}
				</Match>
				<Match when={props.item.type === "info"}>
					<div class="timeline-header">
						<header>
							<h1>{props.channel.name}</h1>
							<p>
								This is the start of {props.channel.name}.{" "}
								<span
									class="markdown"
									innerHTML={md(props.channel.description ?? "") as string}
								></span>
							</p>
						</header>
					</div>
				</Match>
				<Match when={props.item.type === "divider" && props.item}>
					{(item) => (
						<div
							class="timeline-divider"
							classList={{ unread: item().unread, time: !!item().date }}
						>
							<Show when={item().unread}>
								<div class="new">new</div>
							</Show>
							<hr />
							<Show when={item().date}>
								{(d) => (
									<>
										<time datetime={d().toISOString()}>
											{d().toDateString()}
										</time>
										<hr />
									</>
								)}
							</Show>
							<Show when={item().unread}>
								<div class="new hidden">new</div>
							</Show>
						</div>
					)}
				</Match>
				<Match when={props.item.type === "skeletons"}>
					<div class="spacer">
						<MessageSkeletons />
					</div>
				</Match>
				<Match when={props.item.type === "spacer-mini"}>
					<div class="spacer-mini"></div>
				</Match>
			</Switch>
		</li>
	);
};
