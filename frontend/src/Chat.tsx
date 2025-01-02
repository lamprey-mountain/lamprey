import {
Accessor,
	createEffect,
	createSignal,
	For,
	on,
	Show,
	useContext,
} from "solid-js";
import Editor from "./Editor.tsx";
import { TimelineItem } from "./Messages.tsx";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { chatctx } from "./context.ts";
import { createList } from "./list.tsx";
import { getTimestampFromUUID, Room, Thread, Timeline } from "sdk";

type ChatProps = {
	thread: Thread,
}

export const ChatMain = (props: ChatProps) => {
	const ctx = useContext(chatctx)!;
	const [tl, setTl] = createSignal<Timeline>(props.thread.timelines.live, { equals: false });

	const messages = () => {
		const t = tl();
		if (!t) return null;
		return t.messages.map((msg) => {
			return {
				id: msg.id,
				body: msg.data.content,
				origin_ts: getTimestampFromUUID(msg.id),
				type: "message",
				sender: "tester",
				is_local: msg.data.is_local,
			};
		});
	};

	async function handleSubmit({ text }: { text: string }) {
		if (text.startsWith("/thread")) {
			const name = text.slice("/thread ".length);
			await ctx.client.http("POST", `/api/v1/rooms/${ctx.roomId()}/threads`, {
				name,
			});
			return;
		}
		props.thread.send({ content: text });
		list.scrollBy(9999);
		// await new Promise(res => setTimeout(res, 1000));
	}

	async function loadAllMessages() {
		setTl(await tl().paginate("b"));
	}
	
	let paginating = false;
	const list = createList({
		items: () => ["spacer", ...messages() ?? [], "editor"],
		autoscroll: () => true,
		onUpdate() {
    	console.log("UPDATE!")
    },
		async onPaginate(dir) {
			if (paginating) return;
    	console.log("PAGNIATE!", dir)
			const t = tl();
			if (!t) return;
			paginating = true;
			if (!t.messages.length) {
				setTl(await t.paginate("b"));
			} else if (dir === "backwards") {
				if (!t.isAtBeginning) setTl(await t.paginate("b"));
			} else {
				if (!t.isAtEnd) setTl(await t.paginate("f"));
			}
			paginating = false;
    },
    topPos: () => 1,
    bottomPos: () => (messages() ?? []).length - 1, // negative pos?
	});

	createEffect(() => {
		setTl(props.thread.timelines.live);
	});

	createEffect(on(ctx.threadId, async () => {
		if (!messages()?.length) {
			paginating = true;
			setTl(await tl()?.paginate("b"));
			paginating = false;
    }
	}));

	return (
		<div class="thread">
			<header>Here is a header.
				<button onClick={loadAllMessages}>load all</button>
			</header>
			<list.List>
				{i => 
				i === "editor" ? (
				<div class="editorwrap">
					<Editor onSubmit={handleSubmit} placeholder="send a message..." />
				</div>
				) : 
				i === "spacer" ? (<div style="flex: 1" />) :
				<TimelineItem msg={i} />}
			</list.List>
		</div>
	);
};

type ChatNavProps = {
	rooms: Array<Room>,
	threads: Array<Thread>,
}

export const ChatNav = (props: ChatNavProps) => {
	const ctx = useContext(chatctx)!;
	return (
		<nav class="bgf2 w256">
			<ul class="lsn">
				<For each={props.rooms}>
					{(room) => (
						<li style={`color:${ctx.roomId() === room.id ? "red" : "white"}`}>
							<button class="bgf3 bgf4-h" onClick={() => ctx.setRoomId(room.id)}>{room.data.name}</button>
							<Show when={ctx.roomId() === room.id}>
								<ul class="ml16 lsn">
									<For
										each={props.threads.filter((i) => i.data.room_id === ctx.roomId())}
									>
										{(thread) => (
											<li style={`color:${ctx.threadId() === thread.id ? "blue" : ""}`}>
												<button class="bgf3 bgf4-h" onClick={() => ctx.setThreadId(thread.id)}>{thread.data.name}</button>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</li>
					)}
				</For>
			</ul>
		</nav>
	);
};
