import { createMemo, createSignal, For, Show } from "solid-js";
import type { Channel } from "sdk";
import { useApi } from "./api.tsx";
import { MessageView } from "./Message.tsx";
import type { Message } from "sdk";

type PinnedMessagesProps = {
	channel: Channel;
};

export function PinnedMessages(props: PinnedMessagesProps) {
	const api = useApi();
	const pinnedMessages = api.messages.listPinned(() => props.channel.id);

	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(
		null,
	);

	const getMessageId = (e: DragEvent) =>
		(e.currentTarget as HTMLLIElement).dataset.messageId;

	const handleDragStart = (e: DragEvent) => {
		const id = getMessageId(e);
		if (id) setDragging(id);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		const id = getMessageId(e);
		if (!id || id === dragging()) {
			return;
		}
		const rect = (e.currentTarget as HTMLLIElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;

		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) {
			return;
		}

		const messages = pinnedMessages()?.items;
		if (!messages) return;

		const fromIndex = messages.findIndex((m) => m.id === fromId);
		let toIndex = messages.findIndex((m) => m.id === toId);

		if (fromIndex === -1 || toIndex === -1) {
			return;
		}

		if (after) toIndex++;
		if (fromIndex < toIndex) toIndex--;

		const reordered = [...messages];
		const [moved] = reordered.splice(fromIndex, 1);
		reordered.splice(toIndex, 0, moved);

		if (
			JSON.stringify(messages.map((m) => m.id)) ===
				JSON.stringify(reordered.map((m) => m.id))
		) {
			return;
		}

		const body = reordered.map((m, i) => ({
			id: m.id,
			position: i,
		}));

		api.messages.reorderPins(props.channel.id, body);

		// optimistic update
		const current = pinnedMessages();
		if (current) {
			api.messages._pinnedListings.get(props.channel.id)?.mutate({
				...current,
				items: reordered,
			});
		}
	};

	const previewedMessages = createMemo(() => {
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;
		const messages = pinnedMessages()?.items;

		if (!messages || !fromId || !toId || fromId === toId) {
			return messages ?? [];
		}

		const fromIndex = messages.findIndex((m) => m.id === fromId);
		let toIndex = messages.findIndex((m) => m.id === toId);

		if (fromIndex === -1 || toIndex === -1) {
			return messages;
		}

		if (after) toIndex++;
		if (fromIndex < toIndex) toIndex--;

		const reordered = [...messages];
		const [moved] = reordered.splice(fromIndex, 1);
		reordered.splice(toIndex, 0, moved);

		return reordered;
	});

	return (
		<div class="pinned-messages-list" data-channel-id={props.channel.id}>
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
							<For each={previewedMessages()}>
								{(message: Message) => (
									<li
										class="pinned-message-item"
										data-message-id={message.id}
										draggable="true"
										onDragStart={handleDragStart}
										onDragOver={handleDragOver}
										onDrop={handleDrop}
										onDragEnd={() => {
											setDragging(null);
											setTarget(null);
										}}
										classList={{
											dragging: dragging() === message.id,
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
