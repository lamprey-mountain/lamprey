import {
	useFlumes,
	useMessages,
	useRoomMembers,
	type MessageListAnchor,
} from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { logger } from "@/utils/logger";
import {
	createEffect,
	on,
	Show,
	onCleanup,
	Switch,
	Match,
	createSignal,
	createMemo,
	onMount,
} from "solid-js";
import { ChatProps } from "./Chat";
import { highlight, TimelineItemT2 } from "./util";
import { MessageToolbarMount } from "./MessageToolbar";
import { useTimeline } from "./timeline-context";
import { ChannelT } from "@/types";
import { MessageView } from "./Message";
import { MessageSkeletons } from "./MessageSkeleton";
import { Key } from "@solid-primitives/keyed";
import { md } from "@/lib/markdown";
import { useChannel } from "@/contexts/mod";
import {
	createTimelineVirtualizer,
	PAGINATE_THRESHOLD,
	STICKY_THRESHOLD,
	TimelineTask,
	TimelineVirtualizer,
} from "./virtualizer";

const log = logger.for("timeline");

export const Timeline = (props: ChatProps) => {
	const messages = useMessages();
	const timeline = useTimeline();

	const [scrollEl, setScrollEl] = createSignal(null as HTMLDivElement | null);

	// TODO: store this in `timeline` (timeline context)
	const virt = createTimelineVirtualizer({
		scrollEl,
		channel: props.channel,
	});

	const hl = virt.highlighter;
	const queue = virt.queue;

	const ro = new ResizeObserver((entries) => {
		let scrollElResized = false;
		let delta = 0;

		for (const entry of entries) {
			if (entry.target === scrollEl()) {
				scrollElResized = true;
				continue;
			}

			const key = (entry.target as HTMLElement).dataset.key;
			if (!key) continue;

			const heightOld = virt.measurements.get(key);
			// if (!heightOld) return; // TODO: handle this

			const heightNew =
				entry.borderBoxSize?.[0]?.blockSize ?? entry.contentRect.height;
			if (heightOld !== heightNew) {
				delta += heightNew - (heightOld ?? 0);
				virt.measurements.set(key, heightNew);
			}
		}

		if (delta !== 0 || scrollElResized) {
			queue.push({
				type: "RESIZE",
				delta,
			});
		}
	});

	onMount(() => {
		const el = scrollEl();
		if (!el) return;
		ro.observe(el);
	});

	onCleanup(() => ro.disconnect());

	// refetch messages whenever a channel's message version is bumped
	// PERF: MessagesService.handleMessageCreate() is called when MessagesService.send() completes AND when MessageCreate is received. i should deduplicate these events.
	// PERF: ChannelsService.ack seems to retrigger this too for some reason?
	createEffect(
		on(
			() => messages._versions.get(props.channel.id),
			(vNew, vOld) => {
				if (queue.active) return;
				if (vNew === vOld) return;

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

	// handle pagination on scroll
	// return true if pagination was triggered
	const attemptPagination = () => {
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
				return true;
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
				return true;
			}
		}

		return false;
	};

	// debounce/throttle handleScroll to only recalculate once per
	// requestAnimationFrame, since theres no point in doing more calculations that the
	// browser cant render
	let redrawRequested = false;
	const handleScroll = () => {
		const el = scrollEl();
		if (!el) return;

		timeline.scrollTop = el.scrollTop;

		if (!queue.active) {
			attemptPagination();
		}

		if (!redrawRequested) {
			requestAnimationFrame(() => {
				virt.refreshVisibleRows();
				redrawRequested = false;
			});
			redrawRequested = true;
		}
	};

	// handle terminal events on scrollend event.
	const handleScrollEnd = () => {
		hl.reset();

		const el = scrollEl();
		if (!el) return;
		timeline.events.emit("scrollPosition", el.scrollTop);

		const msgs = timeline.messages;
		if (!msgs) return;

		const atTop = el.scrollTop < PAGINATE_THRESHOLD;
		const atBottom =
			el.scrollHeight - el.scrollTop - el.clientHeight < PAGINATE_THRESHOLD;

		if (atTop) {
			if (!msgs.has_backwards) {
				timeline.events.emit("scrollTop");
			}
		} else if (atBottom) {
			if (!msgs.has_forward) {
				timeline.events.emit("scrollBottom");
			}
		}
	};

	// TODO: queue.push SCROLL_BY
	timeline.commands.on("scrollBy", (data) => {
		scrollEl()?.scrollBy({
			top: data.px,
			behavior: data.smooth ? "smooth" : "auto",
		});
	});

	forwardCommands(virt);

	timeline.commands.on("ackMessage", (data) => {
		timeline.readMarkerId = data.message_id;

		if (!timeline.messages?.contains(data.message_id)) return;

		// update anchor to rerender
		// NOTE: maybe i should have a dedicated RERENDER task (with option to skip message fetch?)
		queue.push({ type: "SET_ANCHOR", anchor: timeline.anchor });
	});

	timeline.commands.listen((e) => {
		log.debug("command", e.name, e.details ?? "(null)");
	});

	timeline.events.listen((e) => {
		log.debug("event", e.name, e.details ?? "(null)");
	});

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
					height: `${virt.accessTotalSize()}px`,
				}}
			>
				<Key each={virt.accessVisibleRows()} by={(row) => row.item.key}>
					{(row) => {
						const handleRef = (ref: HTMLDivElement) => {
							ro.observe(ref);
							onCleanup(() => ro.unobserve(ref));

							createEffect(() => {
								const item = row().item;
								if (item.type === "message" && item.message.id === hl.pending) {
									highlight(ref.children[0]);
									hl.reset();
								}
							});
						};

						return (
							<div
								class="timeline-item"
								data-index={row().index}
								data-key={row().item.key}
								data-size={row().size}
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

// listen for timeline commands and map them to tasks
// TODO: handle the rest of the commands
function forwardCommands(virt: TimelineVirtualizer) {
	const timeline = useTimeline();

	timeline.commands.on("jumpToBottom", (data) => {
		virt.queue.push(
			{ type: "SET_ANCHOR", anchor: { type: "backwards", limit: 50 } },
			{
				type: "SCROLL_BOTTOM",
				smooth: data.smooth,
			},
		);
	});

	timeline.commands.on("jumpToTop", (data) => {
		virt.queue.push(
			{ type: "SET_ANCHOR", anchor: { type: "forwards", limit: 50 } },
			{
				type: "SCROLL_TOP",
				smooth: data.smooth,
			},
		);
	});

	timeline.commands.on("jumpToMessage", (data) => {
		virt.queue.push(
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
			virt.queue.push({
				type: "HIGHLIGHT",
				message_id: data.message_id,
			});
		}
	});
}

export const TimelineItem2 = (props: {
	channel: ChannelT;
	item: TimelineItemT2;
}) => {
	const roomMembersService = useRoomMembers();
	const [ch] = useChannel()!;
	const flumes = useFlumes();
	const currentUser = useCurrentUser();
	const room_member = roomMembersService.useMember(
		() => props.channel.room_id ?? "",
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
