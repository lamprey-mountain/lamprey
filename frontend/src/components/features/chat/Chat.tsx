import { throttle } from "@solid-primitives/scheduled";
import type { Channel, ChannelType } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	Match,
	on,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import { useApi, useChannels, useMessages } from "@/api";
import { Time } from "@/atoms/Time.tsx";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useReadTracking } from "@/contexts/read-tracking";
import { useUploads } from "@/contexts/uploads.tsx";
import { useMessageNav } from "@/hooks/useMessageNav";
import { usePermissions } from "@/hooks/usePermissions";
import { getDate } from "@/utils/general.tsx";
import { Input } from "./Input.tsx";
import { MessageToolbarProvider } from "./message-toolbar-context.tsx";
import { Timeline } from "./Timeline.tsx";
import { TimelineProvider, useTimeline } from "./timeline-context.tsx";

export type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi();
	const [channelState, setChannelState] = useChannel()!;
	const messageNav = useMessageNav(() => props.channel.id);
	let chatRef!: HTMLDivElement;

	const onFocusIn = (e: FocusEvent) => {
		if (!channelState.messageNavActive) return;
		const target = e.target as HTMLElement | null;
		const messageEl = target?.closest("[data-message-id]");
		if (messageEl instanceof HTMLElement) {
			const id = messageEl.getAttribute("data-message-id");
			if (id) setChannelState("focusedMessageId", id);
			return;
		}
		setChannelState("focusedMessageId", undefined);
	};

	const onFocusOut = (e: FocusEvent) => {
		if (!channelState.messageNavActive) return;
		const next = e.relatedTarget;
		if (next instanceof Node && chatRef.contains(next)) return;
		if (next instanceof Element && next.closest("#overlay")) return;
		messageNav.deactivate();
	};

	const onPointerDown = () => {
		if (!channelState.messageNavActive) return;
		messageNav.deactivate();
	};

	onMount(() => {
		const el = chatRef;
		el.addEventListener("focusin", onFocusIn);
		el.addEventListener("focusout", onFocusOut);
		el.addEventListener("pointerdown", onPointerDown);
		onCleanup(() => {
			el.removeEventListener("focusin", onFocusIn);
			el.removeEventListener("focusout", onFocusOut);
			el.removeEventListener("pointerdown", onPointerDown);
		});
	});

	// move DOM focus onto the selected message
	createEffect(() => {
		const id = channelState.focusedMessageId;
		if (!id) return;

		let cancelled = false;
		const tryFocus = (attempt: number) => {
			if (cancelled) return;
			const el = chatRef.querySelector(
				`[data-message-id="${CSS.escape(id)}"]`,
			) as HTMLElement | null;
			if (el) {
				el.focus({ preventScroll: true });
				el.scrollIntoView({ block: "nearest" });
				return;
			}
			// HACK: message isn't rendered yet, keep retrying until it mounts
			if (attempt === 0) {
				channelState.timelineState.controller.jumpToMessage(id, false, false);
			}
			if (attempt < 60) {
				requestAnimationFrame(() => tryFocus(attempt + 1));
			}
		};
		queueMicrotask(() => tryFocus(0));
		onCleanup(() => {
			cancelled = true;
		});
	});

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

	// FIXME: don't use <Show keyed>

	return (
		<MessageToolbarProvider>
			<Show when={props.channel.id} keyed>
				<TimelineProvider channel={props.channel}>
					<div
						ref={chatRef}
						class="chat"
						classList={{ "has-typing": !!getTyping().length }}
						data-channel-id={props.channel.id}
						onClick={(e) => {
							// console.log(e.target.closest(".avatar[data-user-id]"));
							// TODO: open user view
						}}
						onKeyDown={(e) => {
							if (e.defaultPrevented) return;

							if (channelState.focusedMessageId) {
								// FIXME: tab sometimes navigates to other messages' stuff?
								if (e.key === "Escape") {
									e.preventDefault();
									messageNav.exit();
									return;
								}
								if (e.key === "ArrowUp") {
									e.preventDefault();
									messageNav.navigate(-1);
									return;
								}
								if (e.key === "ArrowDown") {
									e.preventDefault();
									messageNav.navigate(1);
									return;
								}
								if (e.key === "r" && !e.ctrlKey && !e.metaKey && !e.altKey) {
									e.preventDefault();
									messageNav.reply();
									return;
								}
								if (e.key === "Enter" && e.altKey) {
									e.preventDefault();
									messageNav.markUnread();
									return;
								}
								if (e.key === "Backspace" || e.key === "Delete") {
									e.preventDefault();
									messageNav.deleteMessage();
									return;
								}
								if (e.key === "p" && !e.ctrlKey && !e.metaKey && !e.altKey) {
									e.preventDefault();
									messageNav.togglePin();
									return;
								}
								if (e.key === "e" && !e.ctrlKey && !e.metaKey && !e.altKey) {
									e.preventDefault();
									messageNav.edit();
									return;
								}
								if (e.key === "c" && (e.ctrlKey || e.metaKey)) {
									if (messageNav.copyContent()) {
										e.preventDefault();
										return;
									}
								}
								if (e.key === "+") {
									e.preventDefault();
									messageNav.openReactionPicker();
									return;
								}
							}

							if (e.key === "Escape") {
								messageNav.jumpToEnd();
							} else if (e.key === "PageDown") {
								channelState.timelineState.controller.scrollBy(
									globalThis.innerHeight * 0.8,
									true,
								);
							} else if (e.key === "PageUp") {
								channelState.timelineState.controller.scrollBy(
									-globalThis.innerHeight * 0.8,
									true,
								);
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
						<TimelineControls channel={props.channel} />
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
				</TimelineProvider>
			</Show>
		</MessageToolbarProvider>
	);
};

const isThread = (ty: ChannelType) =>
	ty === "ThreadPublic" || ty === "ThreadPrivate" || ty === "ThreadForum2";

export const TimelineControls = (props: ChatProps) => {
	const timeline = useTimeline();
	const rt = useReadTracking();
	const channels = useChannels();
	const messagesService = useMessages();
	const getMe = useCurrentUser();
	const { has: hasPermission } = usePermissions(
		() => getMe()?.id,
		() => props.channel.room_id ?? "",
		() => props.channel.id,
	);
	const [showNewMessages, setShowNewMessages] = createSignal(false);
	const [scrollIndex, setScrollIndex] = createSignal(0);

	// ack channel when scrolled to bottom
	const markReadFn = throttle(() => {
		const message_id = props.channel.last_message_id;
		const read_id = props.channel.last_read_id;
		if (message_id && message_id !== read_id) {
			rt.ack(props.channel.id, message_id, false, true);
		}
	}, 300);
	timeline.events.on("scrollBottom", markReadFn);

	// TODO: move this into some utiity thing?
	const isAtBottom = () => {
		const si = scrollIndex();
		const m = timeline.messages;
		if (!m) return true;
		if (m.has_forward) return false; // FIXME: try to merge with live timeline, avoid gaps

		const itemsBelow = m.items.length - si;
		return itemsBelow <= 50;
	};

	const hasUnread = () => {
		const id = props.channel.last_message_id;
		return !!id && id !== props.channel.last_read_id;
	};

	// TODO: always show new messages bar when not autoscrolling and a message is received
	// TODO: extract new messages bar logic into a state machine or object/class?
	const updateShow = () => {
		// setShowNewMessages(hasUnread() && !isAtBottom());
		setShowNewMessages(hasUnread());
	};

	createEffect(
		on(
			[
				() => props.channel.last_message_id,
				() => props.channel.last_read_id,
				scrollIndex,
			],
			updateShow,
		),
	);

	timeline.events.on("scrollIndex", setScrollIndex);
	timeline.events.on("paginate", updateShow);

	const jumpToRead = () => {
		timeline.controller.jumpToMessage(timeline.readMarkerId!, true);
	};

	const markRead = () => {
		rt.ack(props.channel.id, props.channel.last_message_id!, true, false);
	};

	const unreadMessageCount = createMemo(() => {
		// depend on the version tracker for reactivity
		messagesService._versions.get(props.channel.id);

		const lastReadId = props.channel.last_read_id;
		if (!lastReadId) return { count: 0, approximate: false };

		const ranges = messagesService._ranges.get(props.channel.id);
		if (!ranges) return { count: 0, approximate: true };

		let count = 0;
		for (const range of ranges.ranges) {
			count += range.items.filter((m) => m.id > lastReadId).length;
		}

		return {
			count,
			approximate: ranges.live.has_forward,
		};
	});

	const lastReadTimestamp = createMemo(() => {
		const lastReadId = props.channel.last_read_id;
		if (!lastReadId) return new Date();
		const message = messagesService.cache.get(lastReadId);
		return message ? getDate(message.created_at) : new Date();
	});

	return (
		<div class="timeline-controls">
			<Show when={props.channel.locked || props.channel.archived_at}>
				<div class="channel-locked">
					<Switch>
						<Match when={props.channel.locked && props.channel.archived_at}>
							This channel has been locked and archived.
						</Match>
						<Match when={props.channel.locked}>
							This channel has been locked.
						</Match>
						<Match when={props.channel.archived_at}>
							This channel has been archived.
						</Match>
					</Switch>
					<Show
						when={
							(hasPermission("ChannelManage") ||
								(isThread(props.channel.type) &&
									hasPermission("ThreadManage"))) &&
							props.channel.locked
						}
					>
						<button
							type="button"
							class="button"
							onClick={() => channels.unlock(props.channel.id)}
						>
							Unlock
						</button>
					</Show>
				</div>
			</Show>
			<Show when={showNewMessages() && unreadMessageCount().count > 0}>
				<div class="new-messages">
					<button type="button" class="jump-read" onClick={jumpToRead}>
						{(() => {
							const { count, approximate } = unreadMessageCount();
							return `${count}${approximate ? "+" : ""}`;
						})()} new messages since <Time date={lastReadTimestamp()} />
					</button>
					<button type="button" class="mark-read" onClick={markRead}>
						mark as read
					</button>
				</div>
			</Show>
		</div>
	);
};
