import type { Channel } from "sdk";
import { createSignal, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import { useApi } from "@/api";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useUploads } from "@/contexts/uploads.tsx";
import { Input } from "./Input.tsx";
import { Timeline } from "./Timeline.tsx";
import { MessageToolbarProvider } from "./message-toolbar-context.tsx";

export type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const api2 = useApi();
	const [channelState] = useChannel()!;

	const jumpToEnd = (markRead = false) => {
		channelState.timeline.jumpToEnd(markRead);
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

	return (
		<MessageToolbarProvider>
			<div
				class="chat"
				classList={{ "has-typing": !!getTyping().length }}
				data-channel-id={props.channel.id}
				onKeyDown={(e) => {
					if (e.key === "Escape") {
						jumpToEnd(true);
					} else if (e.key === "PageDown") {
						channelState.timeline.scrollBy(globalThis.innerHeight * 0.8, true);
					} else if (e.key === "PageUp") {
						channelState.timeline.scrollBy(-globalThis.innerHeight * 0.8, true);
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
