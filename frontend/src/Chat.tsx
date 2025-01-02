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

	function refresh() {
		setTl(tl());
	}

	let oldLive: Timeline;
	createEffect(() => {
		if (oldLive) oldLive.events.off("append", refresh);
		oldLive = props.thread.timelines.live;
		oldLive.events.on("append", refresh);
	});
	
	createEffect(() => {
		console.log("set thread");
		setTl(props.thread.timelines.live);
	});

	const messages = () => {
		const t = tl();
		if (!t) return null;
		return t.messages.map((msg) => {
			return {
				id: msg.id,
				body: msg.data.content,
				origin_ts: /^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(msg.id) ? getTimestampFromUUID(msg.id) : Date.now(),
				type: "message",
				sender: "tester",
				is_local: msg.data.is_local,
			};
		});
	};

	async function handleSubmit({ text }: { text: string }) {
		if (text.startsWith("/thread")) {
			const name = text.slice("/thread ".length);
			await ctx.client.http("POST", `/api/v1/rooms/${ctx.threadId()}/threads`, {
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

	createEffect(on(ctx.threadId, async () => {
		if (!messages()?.length) {
			paginating = true;
			setTl(await tl()?.paginate("b"));
			paginating = false;
    }
	}));

	return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[24px_1fr] relative">
			<header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">Here is a header.
				<button onClick={loadAllMessages}>load all</button>
			</header>
			<list.List>
				{i => 
						i === "editor" ? (
						<div class="sticky bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[142px] pr-[4px] max-h-50% translate-y-[8px]">
							<Editor onSubmit={handleSubmit} placeholder="send a message..." />
						</div>
					)
					: i === "spacer" ? (<div style="flex: 1" />)
					: <TimelineItem msg={i} />}
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
		<nav class="w-64 bg-bg1 text-fg2 overflow-y-auto">
			<ul class="p-1 flex flex-col">
				<For each={props.rooms}>
					{(room) => (
						<li class="mt-1">
							<button
								class="px-1 py-0.25 w-full text-left hover:bg-bg4"
								classList={{ "bg-bg3": ctx.roomId() === room.id }}
								onClick={() => ctx.setRoomId(room.id)}
							>{room.data.name}</button>
							<Show when={ctx.roomId() === room.id}>
								<ul class="ml-6">
									<For each={props.threads.filter((i) => i.data.room_id === ctx.roomId())}>
										{(thread) => (
											<li class="mt-1">
												<button
													class="px-1 py-0.25 w-full text-left hover:bg-bg4"
													classList={{ "bg-bg3": ctx.threadId() === thread.id }}
													onClick={() => ctx.setThreadId(thread.id)}
												>{thread.data.name}</button>
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
