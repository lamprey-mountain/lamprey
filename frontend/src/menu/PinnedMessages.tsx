import { createSignal, For, Show } from "solid-js";
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

	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<string | null>(null);

	const getMessageId = (e: DragEvent) =>
		(e.currentTarget as HTMLLIElement).dataset.messageId;

	const handleDragStart = (e: DragEvent) => {
		const id = getMessageId(e);
		if (id) setDragging(id);
	};

	const handleDragEnter = (e: DragEvent) => {
		e.preventDefault();
		setTarget(getMessageId(e) ?? null);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target();

		if (!fromId || !toId || fromId === toId) {
			setDragging(null);
			setTarget(null);
			return;
		}

		const messages = pinnedMessages()?.items;
		if (!messages) return;

		const fromIndex = messages.findIndex((m) => m.id === fromId);
		const toIndex = messages.findIndex((m) => m.id === toId);

		if (fromIndex === -1 || toIndex === -1) {
			setDragging(null);
			setTarget(null);
			return;
		}

		const reordered = [...messages];
		const [moved] = reordered.splice(fromIndex, 1);
		reordered.splice(toIndex, 0, moved);

		const body = reordered.map((m, i) => ({
			id: m.id,
			position: i,
		}));

		api.messages.reorderPins(props.thread.id, body);

		// optimistic update
		const current = pinnedMessages();
		if (current) {
			api.messages._pinnedListings.get(props.thread.id)?.mutate({
				...current,
				items: reordered,
			});
		}

		setDragging(null);
		setTarget(null);
	};

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
									<li
										class="pinned-message-item"
										data-message-id={message.id}
										draggable="true"
										onDragStart={handleDragStart}
										onDragEnter={handleDragEnter}
										onDragOver={handleDragOver}
										onDrop={handleDrop}
										classList={{
											dragging: dragging() === message.id,
											over: target() === message.id,
										}}
									>
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
