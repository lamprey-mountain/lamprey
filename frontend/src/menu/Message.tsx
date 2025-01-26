// the context menu for messages

import { useCtx } from "../context.ts";
import { MessageT } from "../types.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

// should i have a separate one for bulk messages?
export function MessageMenu(props: { message: MessageT }) {
	const ctx = useCtx();
	const copyId = () => navigator.clipboard.writeText(props.message.id);
	const setReply = () =>
		ctx.dispatch({
			do: "thread.reply",
			thread_id: props.message.thread_id,
			reply_id: props.message.id,
		});

	function markUnread() {
		const thread = ctx.data.timelines[props.message.thread_id];
		const index = thread.findIndex((i) =>
			i.type === "remote" && i.message.id === props.message.id
		);
		const next = thread[index - 1];
		const next_id = next?.type === "remote"
			? next.message.id
			: props.message.id;
		ctx.dispatch({
			do: "thread.mark_read",
			thread_id: props.message.thread_id,
			version_id: next_id,
			also_local: true,
		});
	}

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
			<Item
				onClick={() => console.log(JSON.parse(JSON.stringify(props.message)))}
			>
				log to console
			</Item>
		</Menu>
	);
}
