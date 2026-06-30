import { throttle } from "@solid-primitives/scheduled";
import type { Channel } from "sdk";
import { createEffect, createMemo, createSignal, on, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import { useApi, useMessages } from "@/api";
import type { MessageListAnchor } from "@/api/services/MessagesService.ts";
import { createList2 } from "@/atoms/list.tsx";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useReadTracking } from "@/contexts/read-tracking.tsx";
import { useUploads } from "@/contexts/uploads.tsx";
import { deepEqual } from "@/utils/deepEqual.ts";
import { logger } from "@/utils/logger";
import { Input } from "./Input.tsx";
import { MessageSkeletons } from "./MessageSkeleton.tsx";
import { MessageToolbarContainer } from "./MessageToolbarContainer.tsx";
import { renderTimeline, type TimelineItemT } from "./Messages.tsx";
import { highlight } from "./util.ts";
import { Timeline } from "./Timeline.tsx";
import { MessageToolbarProvider } from "./message-toolbar-context.tsx";

export type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi();
	const messagesService = useMessages();
	const { markChannelRead } = useReadTracking();
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

	const messages = messagesService.useList(() => props.channel.id, anchor);

	const markReadImmediately = () => {
		const version_id = props.channel.last_version_id;
		if (version_id) {
			markChannelRead(props.channel.id, version_id, false, true);
		}
	};

	const markRead = throttle(markReadImmediately, 300);

	const jumpToLastRead = () => {
		const r = read_marker_id();
		if (r) {
			setChannelState("anchor", {
				type: "context",
				limit: 50,
				message_id: r,
			});
		}
	};

	const autoscroll = () =>
		!messages()?.has_forward && anchor().type !== "context";

	const timelineCache = new Map<string, TimelineItemT>();

	createEffect(
		on(
			() => props.channel.id,
			() => {
				timelineCache.clear();
			},
		),
	);

	const tl = createMemo(() => {
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

	const log = logger.for("timeline");

	createEffect(() => {
		log.debug("tl", { tl: [...tl()], msgs: messages() });
	});

	let chatRef: HTMLDivElement | undefined;
	const list = createList2({
		items: tl,
		autoscroll,
		estimateSize: () => 80,
		onPaginate(dir) {
			log.debug(`paginate dir=${dir} loading=${messages.loading}`);

			if (messages.loading) return;

			const MIN_MESSAGES = 50;

			// messages are approx. 20 px high, show 3 pages of messages
			const SLICE_LEN = Math.max(
				MIN_MESSAGES,
				Math.ceil(globalThis.innerHeight / 20) * 3,
			);

			// scroll a page at a time
			const PAGINATE_LEN = Math.floor(SLICE_LEN / 3);

			const msgs = messages()!;
			const old = { ...channelState.anchor } as MessageListAnchor;

			if (dir === "forwards") {
				if (msgs.has_forward) {
					const idx = Math.max(0, msgs.items.length - PAGINATE_LEN);
					setChannelState("anchor", {
						type: "forwards",
						limit: SLICE_LEN,
						message_id: msgs.items[idx]?.id,
					});
				} else {
					// live timeline
					setChannelState("anchor", {
						type: "backwards",
						limit: SLICE_LEN,
						message_id: undefined,
					});

					if (list.isAtBottom()) markRead();
				}
			} else if (dir === "backwards") {
				if (msgs.has_backwards) {
					const idx = Math.min(PAGINATE_LEN, msgs.items.length - 1);
					setChannelState("anchor", {
						type: "backwards",
						limit: SLICE_LEN,
						message_id: msgs.items[idx]?.id,
					});
				}
			}

			const anchor = { ...channelState.anchor };

			if (!deepEqual(old, anchor)) {
				log.debug("set anchor", anchor);
			}
		},
		onRestore() {
			const a = anchor();
			log.info("restore", { ...a });
			if (a.type === "context") {
				const offset = list.getOffset(a.message_id);
				if (offset !== null) {
					const targetOffset = offset - list.getViewportHeight() / 2;
					const distance = Math.abs(list.scrollPos() - targetOffset);
					const shouldSmooth = distance < list.getViewportHeight() * 3;
					list.scrollTo(targetOffset, shouldSmooth);
					return true;
				}
				return false;
			}
			const pos = channelState.scroll_pos;
			if (pos === undefined || pos === -1) {
				list.scrollToBottom();
			} else {
				list.scrollTo(pos);
			}
			return true;
		},
	});

	let jumpingToEnd = false;

	const jumpToEnd = (markRead = false) => {
		const channel_id = props.channel.id;
		const old = { ...channelState.anchor } as MessageListAnchor;

		// messages are approx. 20 px high, show 3 pages of messages
		// TODO: dedupe SLICE_LEN calculation code
		const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

		setChannelState("scroll_pos", -1);

		setChannelState("anchor", {
			type: "backwards",
			limit: SLICE_LEN,
			message_id: undefined,
		});

		if (markRead) {
			// FIXME: use message_id, not version_id
			const version_id =
				messagesService._ranges.get(channel_id)?.live.end ??
				props.channel.last_version_id!;
			markChannelRead(channel_id, version_id, true, false);

			// NOTE: unnecessary with markChannelRead?
			// setChannelState("read_marker_id", undefined);
		}

		const anchor = { ...channelState.anchor };
		if (deepEqual(old, anchor)) {
			list.scrollToBottom();
		} else {
			jumpingToEnd = true;
		}
	};

	setChannelState("timeline", {
		jumpToEnd,
	});

	createEffect(() => {
		tl();
		if (jumpingToEnd) {
			queueMicrotask(() => {
				list.scrollToBottom();
			});
			jumpingToEnd = false;
		}
	});

	// effect to initialize new channels
	createEffect(
		on(
			() => props.channel.id,
			(_channel_id) => {
				const last_read_id =
					props.channel.last_read_id ?? props.channel.last_version_id;
				if (channelState.read_marker_id) return;
				if (!last_read_id) return; // no messages in the channel
				setChannelState("read_marker_id", last_read_id);
			},
		),
	);

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		setChannelState("scroll_pos", pos);
	}, 300);

	// Wait for loading to finish, then jump to the highlight (used for replies)
	createEffect(() => {
		const hl = channelState.highlight;
		if (!hl || messages.loading) return;

		queueMicrotask(() => {
			const offset = list.getOffset(hl);
			if (offset !== null) {
				const targetOffset = offset - list.getViewportHeight() / 2;
				const distance = Math.abs(list.scrollPos() - targetOffset);
				const shouldSmooth = distance < list.getViewportHeight() * 3;
				list.scrollTo(targetOffset, shouldSmooth);

				const target = chatRef?.querySelector(
					`article.message[data-message-id="${hl}"]`,
				);
				if (target) highlight(target.closest("li") ?? target);
				setChannelState("highlight", undefined);
			}
		});
	});

	// Auto-scroll to bottom immediately when the current user sends a message
	let lastLiveEnd = "";
	createEffect(() => {
		messagesService._versions.get(props.channel.id); // track version reactively
		const liveRange = messagesService._ranges.get(props.channel.id)?.live;
		if (!liveRange || liveRange.isEmpty()) return;

		const currentEnd = liveRange.end;
		if (lastLiveEnd && currentEnd !== lastLiveEnd) {
			const newMsg = liveRange.items[liveRange.items.length - 1];
			if (newMsg?.is_local && newMsg.author_id === currentUser()?.id) {
				setChannelState("anchor", { type: "backwards", limit: 50 });
				setTimeout(() => list.scrollToBottom(), 0);
			}
		}
		lastLiveEnd = currentEnd;
	});

	createEffect(on(list.scrollPos, setPos));

	const [dragging, setDragging] = createSignal(false);
	let dragCounter = 0;

	const currentUser = useCurrentUser();
	const getTyping = () => {
		const user_id = currentUser()?.id;
		const user_ids = [
			...(api2.typing.get(props.channel.id)?.values() ?? []),
		].filter((i) => i !== user_id);
		return user_ids;
	};

	const uploads = useUploads();

	return (
		<MessageToolbarProvider>
			<div
				ref={chatRef}
				class="chat"
				classList={{ "has-typing": !!getTyping().length }}
				data-channel-id={props.channel.id}
				onKeyDown={(e) => {
					if (e.key === "Escape") {
						jumpToEnd(true);
					} else if (e.key === "PageDown") {
						list.scrollBy(globalThis.innerHeight * 0.8, true);
					} else if (e.key === "PageUp") {
						list.scrollBy(-globalThis.innerHeight * 0.8, true);
					}
				}}
				onDragEnter={(e) => {
					e.preventDefault();
					dragCounter++;
					setDragging(true);
				}}
				onDragOver={(e) => {
					e.preventDefault();
					setDragging(true);
				}}
				onDragLeave={(e) => {
					e.preventDefault();
					dragCounter--;
					if (dragCounter === 0) setDragging(false);
				}}
				onDrop={(e) => {
					e.preventDefault();
					dragCounter = 0;
					setDragging(false);
					for (const file of Array.from(e.dataTransfer?.files ?? [])) {
						const local_id = uuidv7();
						uploads.init(local_id, props.channel.id, file);
					}
				}}
			>
				<Show
					when={
						messages()?.has_forward &&
						props.channel.last_version_id !== channelState.read_marker_id
					}
				>
					<div class="new-messages">
						<button type="button" class="jump-read" onClick={jumpToLastRead}>
							jump to unread
						</button>
						<button
							type="button"
							class="mark-read"
							onClick={markReadImmediately}
						>
							mark as read
						</button>
					</div>
				</Show>
				<Timeline channel={props.channel} />
				<Input channel={props.channel} />
				<Portal>
					<Show when={dragging()}>
						<div class="dnd-upload-message">
							<div class="inner">drop to upload</div>
						</div>
					</Show>
				</Portal>
			</div>
		</MessageToolbarProvider>
	);
};
