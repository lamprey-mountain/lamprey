import { throttle } from "@solid-primitives/scheduled";
import type { Channel, ChannelType } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	Match,
	on,
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
import { usePermissions } from "@/hooks/usePermissions";
import { Input } from "./Input.tsx";
import { MessageToolbarProvider } from "./message-toolbar-context.tsx";
import { Timeline } from "./Timeline.tsx";
import { TimelineProvider, useTimeline } from "./timeline-context.tsx";

export type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi();
	const [channelState] = useChannel()!;
	const readTracking = useReadTracking();

	const markReadFn = throttle(() => {
		const message_id = props.channel.last_message_id;
		const read_id = props.channel.last_read_id;
		if (message_id && message_id !== read_id) {
			readTracking.ack(props.channel.id, message_id, false, true);
		}
	}, 300);

	// ack channel when scrolled to bottom
	channelState.timeline.events.on("scrollBottom", markReadFn);

	// when esc pressed, jump to end of timeline and mark channel as read
	const jumpToEnd = () => {
		channelState.timeline.jumpToBottom();
		const message_id = props.channel.last_message_id;
		const read_id = props.channel.last_read_id;
		if (message_id && message_id !== read_id) {
			readTracking.ack(props.channel.id, message_id, true, false);
		}
	};

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
						class="chat"
						classList={{ "has-typing": !!getTyping().length }}
						data-channel-id={props.channel.id}
						onClick={(e) => {
							// console.log(e.target.closest(".avatar[data-user-id]"));
							// TODO: open user view
						}}
						onKeyDown={(e) => {
							if (e.key === "Escape") {
								jumpToEnd();
							} else if (e.key === "PageDown") {
								channelState.timeline.scrollBy(
									globalThis.innerHeight * 0.8,
									true,
								);
							} else if (e.key === "PageUp") {
								channelState.timeline.scrollBy(
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

	const isAtBottom = () => {
		const m = timeline.messages;
		if (!m) return true;
		if (m.has_forward) return true; // FIXME: try to merge with live timeline, avoid gaps

		const si = scrollIndex();
		const itemsBelow = m.items.length - si;
		return itemsBelow > 50;
	};

	const hasUnread = () => {
		const id = props.channel.last_message_id;
		return !!id && id !== props.channel.last_read_id;
	};

	// TODO: always show new messages bar when not autoscrolling and a message is received
	// TODO: extract new messages bar logic into a state machine or object/class?
	const updateShow = () => {
		setShowNewMessages(hasUnread() && isAtBottom());
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
		return message ? new Date(message.created_at) : new Date();
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
			<Show when={showNewMessages()}>
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
