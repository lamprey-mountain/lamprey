// the context menu for messages

import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { MessageT } from "../types.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

// should i have a separate one for bulk messages?
export function MessageMenu(props: { message: MessageT }) {
	const ctx = useCtx();
	const api = useApi();
	const copyId = () => navigator.clipboard.writeText(props.message.id);
	const setReply = () =>
		ctx.dispatch({
			do: "thread.reply",
			thread_id: props.message.thread_id,
			reply_id: props.message.id,
		});

	function markUnread() {
		const r = api.messages.cacheRanges.get(props.message.thread_id)!;
		const tl = r.find(props.message.id)?.items!;
		const index = tl.findIndex((i) => i.id === props.message.id && !i.is_local);
		const next = tl[index - 1];
		const next_id = next?.id ?? props.message.id;
		ctx.dispatch({
			do: "thread.mark_read",
			thread_id: props.message.thread_id,
			version_id: next_id,
			also_local: true,
		});
	}

	const logToConsole = () =>
		console.log(JSON.parse(JSON.stringify(props.message)));

	return (
		<Menu>
			<Item onClick={markUnread}>mark unread</Item>
			<Item>copy link</Item>
			<Item onClick={setReply}>reply</Item>
			<Item>edit</Item>
			<Item>fork</Item>
			<Item>pin</Item>
			<Item>redact</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
