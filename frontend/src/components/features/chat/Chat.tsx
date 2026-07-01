import type { Channel } from "sdk";
import { createSignal, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import { useApi } from "@/api";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useUploads } from "@/contexts/uploads.tsx";
import { useReadTracking } from "@/contexts/read-tracking";
import { throttle } from "@solid-primitives/scheduled";
import { Input } from "./Input.tsx";
import { Timeline } from "./Timeline.tsx";
import { MessageToolbarProvider } from "./message-toolbar-context.tsx";
import { TimelineProvider } from "./timeline-context.tsx";

export type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi();
	const [channelState] = useChannel()!;
	const readTracking = useReadTracking();

	const markReadFn = throttle(() => {
		const version_id = props.channel.last_version_id;
		const read_id = props.channel.last_read_id;
		if (version_id && version_id !== read_id) {
			readTracking.markChannelRead(props.channel.id, version_id, false, true);
		}
	}, 300);

	// ack channel when scrolled to bottom
	channelState.timeline.events.on("scrollBottom", markReadFn);

	const jumpToEnd = (markRead = false) => {
		channelState.timeline.jumpToBottom();
		if (markRead) {
			markReadFn();
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

	// PERF: see if i can avoid using <Show keyed>?

	return (
		<MessageToolbarProvider>
			<Show when={props.channel} keyed>
				{(chan) => (
					<TimelineProvider channel={chan}>
						<div
							class="chat"
							classList={{ "has-typing": !!getTyping().length }}
							data-channel-id={props.channel.id}
							onClick={(e) => {
								console.log(e.target.closest(".avatar[data-user-id]"));
								// TODO: open user view
							}}
							onKeyDown={(e) => {
								if (e.key === "Escape") {
									jumpToEnd(true);
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
							{/*
							// TODO: impl timeline controls
							// TODO: show controls when new messages are received while not at the end of timeline
							<Show
								when={
									timeline.messages?.has_forward &&
									props.channel.last_version_id !== timeline.last_read_message_id
								}
							>
								<div class="new-messages">
									<button
										type="button"
										class="jump-read"
										onClick={() =>
											timeline.controller.jumpToMessage(
												timeline.last_read_message_id!,
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
							*/}
							<Timeline channel={chan} />
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
				)}
			</Show>
		</MessageToolbarProvider>
	);
};
