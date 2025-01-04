import {
	createEffect,
	createSignal,
	For,
	Match,
	on,
	Show,
	Switch,
	useContext,
} from "solid-js";
import Editor from "./Editor.tsx";
import { TimelineItem } from "./Messages.tsx";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { chatctx } from "./context.ts";
import { createList, createTimeline, TimelineItemT } from "./list.tsx";
import { getTimestampFromUUID, Room, Thread, Timeline } from "sdk";

type ChatProps = {
	thread: Thread,
}

const Item = (props: { item: TimelineItemT }) => {
	return (
		<Switch>
			<Match when={props.item.type === "editor"}>
				<div class="sticky bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[142px] pr-[4px] max-h-50% translate-y-[8px]">
					<Editor onSubmit={props.handleSubmit} placeholder="send a message..." />
				</div>
			</Match>
			<Match when={props.item.type === "editor"}>
				<div style="flex: 1" />
			</Match>
			<Match when={props.item.type === "message"}>
				<TimelineItem msg={props.item.message} />
			</Match>
		</Switch>
	)
}

export const ChatMain = (props: ChatProps) => {
	const ctx = useContext(chatctx)!;
	
	async function handleSubmit({ text }: { text: string }) {
		if (text.startsWith("/")) {
			const [cmd, ...args] = text.slice(1).split(" ");
			if (cmd === "thread") {
				const name = text.slice("/thread ".length);
				await ctx.client.http("POST", `/api/v1/rooms/${ctx.roomId()}/threads`, {
					name,
				});
			} else if (cmd === "archive") {
				await ctx.client.http("PATCH", `/api/v1/threads/${ctx.threadId()}`, {
					is_closed: true,
				});
			} else if (cmd === "unarchive") {
				await ctx.client.http("PATCH", `/api/v1/threads/${ctx.threadId()}`, {
					is_closed: false,
				});
			} else if (cmd === "describe") {
				const description = text.slice("/describe ".length);
				await ctx.client.http("PATCH", `/api/v1/threads/${ctx.threadId()}`, {
					description: description || null,
				});
			} else if (cmd === "describer") {
				const description = text.slice("/describer ".length);
				await ctx.client.http("PATCH", `/api/v1/rooms/${ctx.roomId()}`, {
					description: description || null,
				});
			}
			return;
		}
		props.thread.send({ content: text });
		// await new Promise(res => setTimeout(res, 1000));
	}

	const tl = createTimeline(() => props.thread.timelines);
	const list = createList({
		items: () => tl.items(),
		autoscroll: tl.isAutoscrolling,
    // topPos: () => tl.isAtBeginning() ? 1 : 2,
    topPos: () => 1,
    // bottomPos: () => timel.isAtEnd() ? timel.items().length - 1 : timel.items().length - 2,
    bottomPos: () => tl.items().length - 2,
    onPaginate(dir) {
      if (tl.status() !== "ready") return;
      if (dir === "forwards") {
        tl.forwards();
      } else {
        tl.backwards();
      }
      tl.setIsAutoscrolling(tl.isAtEnd());
    },
	});
	createEffect(() => console.log(tl.items()));

	// translate-y-[8px]
	return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[24px_1fr_0] relative">
			<header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
				{props.thread.data.name} / 
				{props.thread.data.description ?? "(no description)" } /
				<Show when={props.thread.data.is_closed}> (archived)</Show>
			</header>
			<list.List>{item => <TimelineItem item={item} />}</list.List>
			<div class="absolute bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[138px] pr-[4px] max-h-50%">
				<Editor onSubmit={handleSubmit} placeholder="send a message..." />
			</div>
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
													classList={{
														"bg-bg3": ctx.threadId() === thread.id,
														"text-sep": thread.data.is_closed,
													}}
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
