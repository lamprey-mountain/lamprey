// the context menu for messages

import { useApi, useMessages2 } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator } from "./Parts.tsx";
import { useModals } from "../contexts/modal";
import { usePermissions } from "../hooks/usePermissions.ts";
import { Show } from "solid-js";
import { useReadTracking } from "../contexts/read-tracking.tsx";
import { useCurrentUser } from "../contexts/currentUser.tsx";

// should i have a separate one for bulk messages?

type MessageMenuProps = {
	channel_id: string;
	message_id: string;
	version_id: string;
};

export function MessageMenu(props: MessageMenuProps) {
	const ctx = useCtx();
	const api = useApi();
	const { markThreadRead } = useReadTracking();
	const message = api.messages.fetch(
		() => props.channel_id,
		() => props.message_id,
	);
	const [ch, chUpdate] = ctx.channel_contexts.get(props.channel_id)!;
	const [, modalCtl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const channel = api.channels.fetch(() => props.channel_id);
	const { has: hasPermission } = usePermissions(
		self_id,
		() => channel()?.room_id ?? undefined,
		() => props.channel_id,
	);

	const copyId = () => navigator.clipboard.writeText(props.message_id);

	const copyLink = () => {
		const url = new URL(location.origin);
		url.pathname = `/channel/${props.channel_id}/message/${props.message_id}`;
		navigator.clipboard.writeText(url.toString());
	};

	const setReply = () => {
		chUpdate("reply_id", props.message_id);
	};

	const messagesService = useMessages2();

	function markUnread() {
		const r = messagesService.cacheRanges.get(props.channel_id);
		if (!r) return;
		const tl = r.find(props.message_id)?.items;
		if (!tl) return;
		const index = tl.findIndex((i) => i.id === props.message_id && !i.is_local);
		if (index === -1) return;

		const prev = tl[index - 1];
		if (prev) {
			const prev_version_id = prev.latest_version.version_id;
			markThreadRead(props.channel_id, prev_version_id, true);
		} else {
			// If no previous message, we mark everything as unread
			// In our current system, setting it to undefined or a very old ID might work.
			// Clearing the local marker makes it look unread until next sync.
			chUpdate("read_marker_id", undefined);
			// We might need an API call to truly clear it on server, but 'ack' usually takes an ID.
		}
	}

	const togglePin = () => {
		if (message()?.pinned) {
			api.messages.unpin(props.channel_id, props.message_id);
		} else {
			api.messages.pin(props.channel_id, props.message_id);
		}
	};

	function deleteMessage() {
		modalCtl.confirm("really delete?", (conf) => {
			if (!conf) return;
			api.client.http.DELETE(
				"/api/v1/channel/{channel_id}/message/{message_id}",
				{
					params: {
						path: {
							channel_id: props.channel_id,
							message_id: props.message_id,
						},
					},
				},
			);
		});
	}

	const edit = () => {
		chUpdate("editingMessage", {
			message_id: props.message_id,
			selection: "end",
		});
	};

	const selectMessage = () => {
		chUpdate("selectMode", true);
		chUpdate("selectedMessages", [props.message_id]);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(message())));

	const addReaction = () => {
		// HACK: open reaction picker next to toolbar button by clicking it
		const messageEl = document.querySelector(
			`[data-message-id="${props.message_id}"]`,
		);
		if (!messageEl) return;
		const button = messageEl.querySelector(
			'.message-toolbar button[title="Add reaction"]',
		) as HTMLElement;
		if (!button) return;
		button.click();
	};

	const viewReactions = () => {
		modalCtl.open({
			type: "view_reactions",
			channel_id: props.channel_id,
			message_id: props.message_id,
		});
	};

	const canAddReaction = () => hasPermission("ReactionAdd");
	const hasReactions = () => {
		const msg = message();
		return msg?.reactions && msg.reactions.length > 0;
	};
	const canReply = () => hasPermission("MessageCreate");
	const canEdit = () => {
		const msg = message();
		return hasPermission("MessageCreate") && msg?.author_id === self_id();
	};
	const canPin = () => hasPermission("MessagePin");
	const canSelect = () =>
		hasPermission("MessageDelete") || hasPermission("MessageRemove");
	const canDelete = () => {
		const msg = message();
		return hasPermission("MessageDelete") || msg?.author_id === self_id();
	};
	const canCreateThread = () => {
		const msg = message();
		const parentChannel = channel();
		// can create threads in text channels, forums, and similar
		const parentThreadable = parentChannel && [
			"Text",
			"Announcement",
			"Forum",
			"Forum2",
		].includes(parentChannel.type);
		return hasPermission("ThreadCreatePublic") && parentThreadable &&
			!msg?.thread;
	};

	const createThread = () => {
		modalCtl.prompt("thread name?", (name) => {
			if (!name) return;
			api.channels.createThreadFromMessage(
				props.channel_id,
				props.message_id,
				{ name, type: "ThreadPublic" },
			);
		});
	};

	return (
		<Menu>
			<Item onClick={markUnread}>mark unread</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Separator />
			<Show when={canAddReaction()}>
				<Item onClick={addReaction}>add reaction</Item>
			</Show>
			<Show when={hasReactions()}>
				<Item onClick={viewReactions}>view reactions</Item>
			</Show>
			<Show when={canReply()}>
				<Item onClick={setReply}>reply</Item>
			</Show>
			<Show when={canCreateThread()}>
				<Item onClick={createThread}>create thread</Item>
			</Show>
			<Show when={canEdit()}>
				<Item onClick={edit}>edit</Item>
			</Show>
			<Show when={canPin()}>
				<Item onClick={togglePin}>{message()?.pinned ? "unpin" : "pin"}</Item>
			</Show>
			<Show when={canSelect()}>
				<Item onClick={selectMessage}>select</Item>
			</Show>
			<Show when={canDelete()}>
				<Item onClick={deleteMessage} color="danger">delete</Item>
			</Show>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
