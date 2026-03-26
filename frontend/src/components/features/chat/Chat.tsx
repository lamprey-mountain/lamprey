import { createEffect, createMemo, createSignal, on, Show } from "solid-js";
import { useCtx } from "../../../context.ts";
import { createList2 } from "../../../atoms/list.tsx";
import type { Channel } from "sdk";
import {
	renderTimeline,
	TimelineItem,
	type TimelineItemT,
} from "./Messages.tsx";
import { Input } from "./Input.tsx";
import { useApi2, useMessages2 } from "@/api";
import { throttle } from "@solid-primitives/scheduled";
import type { MessageListAnchor } from "@/api/services/MessagesService.ts";
import { uuidv7 } from "uuidv7";
import { Portal } from "solid-js/web";
import { useChannel } from "../../../channelctx.tsx";
import { useReadTracking } from "../../../contexts/read-tracking.tsx";
import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import { useUploads } from "../../../contexts/uploads.tsx";
import { MessageSkeleton } from "./MessageSkeleton.tsx";
import { logger } from "../../../logger.ts";
import { deepEqual } from "../../../utils/deepEqual.ts";

type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi2();
	const messagesService = useMessages2();
	const { t } = useCtx();
	const { markChannelRead } = useReadTracking();
	const [channelState, setChannelState] = useChannel()!;

	const read_marker_id = () => channelState.read_marker_id;

	const anchor = (): MessageListAnchor => {
		const a = channelState.anchor;
		if (a) return a;

		const r = read_marker_id();
		if (r) return { type: "context", limit: 50, message_id: r };

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
			setChannelState("anchor", { type: "context", limit: 50, message_id: r });
		}
	};

	const autoscroll = () =>
		!messages()?.has_forward && anchor().type !== "context";

	const timelineCache = new Map<string, TimelineItemT>();

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

	let last_thread_id: string | undefined;
	let chatRef: HTMLDivElement | undefined;
	const list = createList2({
		items: tl,
		autoscroll,
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
			const old = { ...channelState.anchor };

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
					});

					if (list.isAtBottom()) markRead();
				}
			} else {
				const idx = Math.min(PAGINATE_LEN, msgs.items.length - 1);
				setChannelState("anchor", {
					type: "backwards",
					limit: SLICE_LEN,
					message_id: msgs.items[idx]?.id,
				});
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
					list.scrollTo(offset - list.getViewportHeight() / 2);
					return true;
				}
				return false;
			}
			const pos = channelState.scroll_pos;
			list.scrollTo(pos === undefined || pos === -1 ? 99999999 : pos);
			return true;
		},
	});

	// TODO: re-add jumping to context
	// const list = createList2({
	// 	onRestore() {
	// 		const a = anchor();
	// 		if (a.type === "context") {
	// 			// TODO: is this safe and performant?
	// 			const target = chatRef?.querySelector(
	// 				`article[data-message-id="${a.message_id}"]`,
	// 			);
	// 			console.log("scroll restore: to anchor", a.message_id, target);
	// 			if (target) {
	// 				last_thread_id = props.channel.id;
	// 				target.scrollIntoView({
	// 					behavior: "instant",
	// 					block: "center",
	// 				});
	// 				const hl = channelState.highlight;
	// 				if (hl) scrollAndHighlight(hl);
	// 				return true;
	// 			} else {
	// 				console.warn("couldn't find target to scroll to");
	// 				return false;
	// 			}
	// 		} else if (last_thread_id !== props.channel.id) {
	// 			const pos = channelState.scroll_pos;
	// 			console.log("scroll restore: load pos", pos);
	// 			if (pos === undefined || pos === -1) {
	// 				list.scrollTo(999999);
	// 			} else {
	// 				list.scrollTo(pos);
	// 			}
	// 			last_thread_id = props.channel.id;
	// 			return true;
	// 		} else {
	// 			console.log("nothing special");
	// 			return false;
	// 		}
	// 	},
	// });

	// effect to initialize new channels
	createEffect(on(() => props.channel.id, (_channel_id) => {
		const last_read_id = props.channel.last_read_id ??
			props.channel.last_version_id;
		if (channelState.read_marker_id) return;
		if (!last_read_id) return; // no messages in the channel
		setChannelState("read_marker_id", last_read_id);
	}));

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		setChannelState("scroll_pos", pos);
	}, 300);

	// called both during reanchor and when thread_highlight changes
	function scrollAndHighlight(hl?: string) {
		if (!hl) return;
		const target = chatRef?.querySelector(
			`li:has(article.message[data-message-id="${hl}"])`,
		);
		if (!target) return;
		target.scrollIntoView({
			behavior: "instant",
			block: "center",
		});
		highlight(target);
		setChannelState("highlight", undefined);
	}

	// // TODO: replace with this
	// function scrollAndHighlight(hl?: string) {
	// 	if (!hl) return;
	// 	const offset = list.getOffset(hl);
	// 	if (offset === null) return;
	// 	list.scrollTo(offset - list.getViewportHeight() / 2);
	// 	// highlight the rendered el if it exists
	// 	const el = document.querySelector(`article.message[data-message-id="${hl}"]`);
	// 	if (el) highlight(el.closest('li') ?? el);
	// 	setChannelState("highlight", undefined);
	// }

	createEffect(
		on(() => channelState.highlight, scrollAndHighlight),
	);

	createEffect(on(() => channelState.anchor, (a) => {
		if (a && a.type === "backwards" && !a.message_id) {
			setTimeout(() => {
				list.scrollTo(99999999);
			});
		}
	}));

	createEffect(on(list.scrollPos, setPos));

	const [dragging, setDragging] = createSignal(false);

	const currentUser = useCurrentUser();
	const getTyping = () => {
		const user_id = currentUser()?.id;
		const user_ids = [...api2.typing.get(props.channel.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return user_ids;
	};

	const uploads = useUploads();

	return (
		<div
			ref={chatRef}
			class="chat"
			classList={{ "has-typing": !!getTyping().length }}
			data-channel-id={props.channel.id}
			role="log"
			onKeyDown={(e) => {
				if (e.key === "Escape") {
					const channel_id = props.channel.id;
					const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

					setChannelState("scroll_pos", -1);
					setChannelState("read_marker_id", undefined);

					setChannelState("anchor", {
						type: "backwards",
						limit: SLICE_LEN,
					});

					const version_id =
						messagesService._ranges.get(channel_id)?.live.end ??
							props.channel.last_version_id;

					if (version_id) {
						markChannelRead(channel_id, version_id, true, false);
					}

					setTimeout(() => {
						list.scrollTo(99999999);
					});
					// }

					// 					const channel_id = props.channel.id;
					// 					const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

					// 					// clear stored scroll to guarantee jump on restore
					// 					setChannelState("scroll_pos", -1);

					// 					setChannelState("anchor", {
					// 						type: "backwards",
					// 						limit: SLICE_LEN,
					// 					});

					// 					const version_id =
					// 						messagesService.cacheRanges.get(channel_id)?.live.end ??
					// 							props.channel.last_version_id;

					// 					if (version_id) {
					// 						markChannelRead(channel_id, version_id, true, false);
					// 					}

					// 					setTimeout(() => {
					// 						list.scrollTo(99999999);
					// 					});
				} else if (e.key === "PageDown") {
					list.scrollBy(globalThis.innerHeight * .8, true);
				} else if (e.key === "PageUp") {
					list.scrollBy(-globalThis.innerHeight * .8, true);
				}
			}}
			onDragEnter={(e) => {
				e.preventDefault();
				setDragging(true);
			}}
			onDragOver={(e) => {
				e.preventDefault();
				setDragging(true);
			}}
			onDragLeave={(e) => {
				e.preventDefault();
				setDragging(false);
			}}
			onDrop={(e) => {
				e.preventDefault();
				setDragging(false);
				for (const file of Array.from(e.dataTransfer?.files ?? [])) {
					const local_id = uuidv7();
					uploads.init(local_id, props.channel.id, file);
				}
			}}
		>
			<Show
				when={messages()?.has_forward &&
					(props.channel.last_version_id !== channelState.read_marker_id)}
			>
				<div class="new-messages">
					<button class="jump-read" onClick={jumpToLastRead}>
						jump to unread
					</button>
					<button class="mark-read" onClick={markReadImmediately}>
						mark as read
					</button>
				</div>
			</Show>
			<Show
				when={messages.loading && tl().length === 0}
				fallback={
					<list.List>
						{(item) => (
							<TimelineItem
								thread={props.channel}
								item={item}
								currentUser={currentUser}
							/>
						)}
					</list.List>
				}
			>
				<ul class="skeleton-message-list">
					<MessageSkeleton />
				</ul>
			</Show>
			<Input channel={props.channel} />
			<Portal>
				<Show when={dragging()}>
					<div class="dnd-upload-message">
						<div class="inner">
							drop to upload
						</div>
					</div>
				</Show>
			</Portal>
		</div>
	);
};

function highlight(el: Element) {
	el.animate([
		{
			boxShadow: "4px 0 0 -1px inset #cc1856",
			backgroundColor: "#cc185622",
			offset: 0,
		},
		{
			boxShadow: "4px 0 0 -1px inset #cc1856",
			backgroundColor: "#cc185622",
			offset: .8,
		},
		{
			boxShadow: "none",
			backgroundColor: "transparent",
			offset: 1,
		},
	], {
		duration: 1000,
	});
}
