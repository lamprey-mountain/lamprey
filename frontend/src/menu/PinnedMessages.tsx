import { For, Show } from "solid-js";
import type { Thread } from "sdk";
import { useApi } from "../api.tsx";
import { MessageView } from "../Message.tsx";
import type { Message } from "sdk";

type PinnedMessagesProps = {
	thread: Thread;
};

export function PinnedMessages(props: PinnedMessagesProps) {
	const api = useApi();
	const pinnedMessages = api.messages.listPinned(() => props.thread.id);

	return (
		<div class="pinned-messages-list" data-thread-id={props.thread.id}>
			<Show
				when={pinnedMessages.loading}
				fallback={
					<Show
						when={pinnedMessages()?.items && pinnedMessages()!.items.length > 0}
						fallback={
							<div class="dim" style="text-align: center; margin-top: 8px">
								no pinned messages
							</div>
						}
					>
						<header>{pinnedMessages()?.total} pinned messages</header>
						<ul>
							<For each={pinnedMessages()?.items}>
								{(message: Message) => (
									<li class="pinned-message-item">
										<MessageView message={message} separate={true} />
									</li>
								)}
							</For>
						</ul>
					</Show>
				}
			>
				<div class="dim" style="text-align: center; margin-top: 8px">
					loading...
				</div>
			</Show>
		</div>
	);
}
