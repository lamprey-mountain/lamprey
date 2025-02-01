import { useCtx } from "../context.ts";
import { ThreadT } from "../types.ts";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

// the context menu for threads
export function ThreadMenu(props: { thread: ThreadT }) {
	const ctx = useCtx();
	const copyId = () => navigator.clipboard.writeText(props.thread.id);
	const markRead = () => {
		ctx.dispatch({
			do: "thread.mark_read",
			thread_id: props.thread.id,
			also_local: true,
		});
	};

	const deleteThread = () => {
		ctx.client.http.DELETE("/api/v1/thread/{thread_id}", {
			params: {
				path: { thread_id: props.thread.id },
			},
		});
	};

	const copyLink = () => {
		const url = `${ctx.client.opts.baseUrl}/thread/${props.thread.id}`;
		navigator.clipboard.writeText(url);
	};

	return (
		<Menu>
			<Item onClick={markRead}>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<ThreadNotificationMenu />
			<Separator />
			<Submenu content={"edit"}>
				<Item>info</Item>
				<Item>permissions</Item>
				<Submenu content={"tags"}>
					<Item>foo</Item>
					<Item>bar</Item>
					<Item>baz</Item>
				</Submenu>
			</Submenu>
			<Item>pin</Item>
			<Item>close</Item>
			<Item>lock</Item>
			<Item onClick={deleteThread}>delete</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item>view source</Item>
		</Menu>
	);
}

function ThreadNotificationMenu() {
	return (
		<>
			<Submenu content={"notifications"}>
				<Item>
					<div>default</div>
					<div class="subtext">
						Uses the room's default notification setting.
					</div>
				</Item>
				<Item>
					<div>everything</div>
					<div class="subtext">
						You will be notified of all new messages in this thread.
					</div>
				</Item>
				<Item>
					<div>watching</div>
					<div class="subtext">
						Messages in this thread will show up in your inbox.
					</div>
				</Item>
				<Item>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
				</Item>
				<Separator />
				<Item>bookmark</Item>
				<Submenu content={"remind me"}>
					<Item>in 15 minutes</Item>
					<Item>in 3 hours</Item>
					<Item>in 8 hours</Item>
					<Item>in 1 day</Item>
					<Item>in 1 week</Item>
				</Submenu>
			</Submenu>
			<Submenu content={"mute"}>
				<Item>for 15 minutes</Item>
				<Item>for 3 hours</Item>
				<Item>for 8 hours</Item>
				<Item>for 1 day</Item>
				<Item>for 1 week</Item>
				<Item>forever</Item>
			</Submenu>
		</>
	);
}
