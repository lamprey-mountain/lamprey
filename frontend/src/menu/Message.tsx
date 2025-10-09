// the context menu for messages

import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

// should i have a separate one for bulk messages?

type MessageMenuProps = {
	thread_id: string;
	message_id: string;
	version_id: string;
};

export function MessageMenu(props: MessageMenuProps) {
	const ctx = useCtx();
	const api = useApi();
	const message = api.messages.fetch(
		() => props.thread_id,
		() => props.message_id,
	);

	const copyId = () => navigator.clipboard.writeText(props.message_id);

	const setReply = () => {
		ctx.thread_reply_id.set(props.thread_id, props.message_id);
	};

	function markUnread() {
		const r = api.messages.cacheRanges.get(props.thread_id)!;
		const tl = r.find(props.message_id)?.items!;
		const index = tl.findIndex((i) => i.id === props.message_id && !i.is_local);
		const next = tl[index - 1];
		const next_id = next?.id ?? props.message_id;
		const next_version_id = next?.version_id ?? props.version_id;
		ctx.dispatch({
			do: "thread.mark_read",
			thread_id: props.thread_id,
			version_id: next_version_id,
			message_id: next_id,
			also_local: true,
		});
	}

	const togglePin = () => {
		if (message()?.pinned) {
			api.messages.unpin(props.thread_id, props.message_id);
		} else {
			api.messages.pin(props.thread_id, props.message_id);
		}
	};

	function redact() {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really delete?",
			cont: (conf) => {
				if (!conf) return;
				api.client.http.DELETE(
					"/api/v1/thread/{thread_id}/message/{message_id}",
					{
						params: {
							path: {
								thread_id: props.thread_id,
								message_id: props.message_id,
							},
						},
					},
				);
			},
		});
	}

	const edit = () => {
		ctx.editingMessage.set(props.thread_id, props.message_id);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(message())));

	return (
		<Menu>
			<Item onClick={markUnread}>mark unread</Item>
			<Item>copy link</Item>
			<Item onClick={setReply}>reply</Item>
			<Item onClick={edit}>edit</Item>
			<Item>fork</Item>
			<Item onClick={togglePin}>{message()?.pinned ? "unpin" : "pin"}</Item>
			<Item onClick={redact}>redact</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
