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
import { createList, TimelineItemT } from "./list.tsx";
import { getTimestampFromUUID, Room, Thread, Timeline, TimelineSet } from "sdk";
import { ThreadT, RoomT } from "./types.ts";

type ChatProps = {
	thread: ThreadT,
	room: RoomT,
}

// const Item = (props: { item: TimelineItemT }) => {
// 	return (
// 		<Switch>
// 			<Match when={props.item.type === "editor"}>
// 				<div class="sticky bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[142px] pr-[4px] max-h-50% translate-y-[8px]">
// 					<Editor onSubmit={props.handleSubmit} placeholder="send a message..." />
// 				</div>
// 			</Match>
// 			<Match when={props.item.type === "editor"}>
// 				<div style="flex: 1" />
// 			</Match>
// 			<Match when={props.item.type === "message"}>
// 				<TimelineItem msg={props.item.message} />
// 			</Match>
// 		</Switch>
// 	)
// }

export const ChatMain = (props: ChatProps) => {
	const ctx = useContext(chatctx)!;
	
	async function handleSubmit({ text }: { text: string }) {
		if (text.startsWith("/")) {
			const [cmd, ...args] = text.slice(1).split(" ");
			if (cmd === "thread") {
				const name = text.slice("/thread ".length);
				await ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/threads`, {
					name,
				});
			} else if (cmd === "archive") {
				await ctx.client.http("PATCH", `/api/v1/threads/${props.thread.id}`, {
					is_closed: true,
				});
			} else if (cmd === "unarchive") {
				await ctx.client.http("PATCH", `/api/v1/threads/${props.thread.id}`, {
					is_closed: false,
				});
			} else if (cmd === "describe") {
				const description = text.slice("/describe ".length);
				await ctx.client.http("PATCH", `/api/v1/threads/${props.thread.id}`, {
					description: description || null,
				});
			} else if (cmd === "describer") {
				const description = text.slice("/describer ".length);
				await ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
					description: description || null,
				});
			}
			return;
		}
		// props.thread.send({ content: text });
		// await new Promise(res => setTimeout(res, 1000));
	}

	// if (!ctx.data.timelines[props.thread.id]) {
	// }

	// const tls = new TimelineSet(ctx.client, props.thread.id);
	// const tl = createTimeline(ctx.data, props.thread.id);
	let paginating = false;
	const tl = () => ctx.data.timelines[props.thread.id]?.list.find(i => i.is_at_end);
	if (!tl()) {
    ctx.dispatch({ do: "paginate", dir: "b", thread_id: props.thread.id });
	}
	
	const list = createList({
		items: () => tl()?.messages ?? [],
		autoscroll: () => true,
    // topPos: () => tl.isAtBeginning() ? 1 : 2,
    topPos: () => 1,
    // bottomPos: () => timel.isAtEnd() ? timel.items().length - 1 : timel.items().length - 2,
    bottomPos: () => (tl()?.messages.length ?? 0) - 2,
    onPaginate(dir) {
      if (paginating) return;
      paginating = true;
      if (dir === "forwards") {
	      ctx.dispatch({ do: "paginate", dir: "f", timeline: tl(), thread_id: props.thread.id });
      } else {
	      ctx.dispatch({ do: "paginate", dir: "b", timeline: tl(), thread_id: props.thread.id });
      }
      paginating = false;
      // tl.setIsAutoscrolling(tl.isAtEnd());
    },
	});
	// createEffect(() => console.log(tl.items()));

	// translate-y-[8px]
	return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[24px_1fr_0] relative">
			<header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
				{props.thread.name} / 
				{props.thread.description ?? "(no description)" } /
				<Show when={props.thread.is_closed}> (archived)</Show>
			</header>
			<list.List>{item => <TimelineItem item={{ type: "message", message: item, key: item.id, separate: false }} />}</list.List>
			<div class="absolute bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[138px] pr-[4px] max-h-50%">
				<Editor onSubmit={handleSubmit} placeholder="send a message..." />
			</div>
		</div>
	);
};

export const ChatNav = () => {
	const ctx = useContext(chatctx)!;
	const v = ctx.data.view;
	console.log(ctx, v)
	const roomId = () => (v.view === "room" || v.view === "room-settings" || v.view === "thread") ? v.room.id : null;
	const threadId = () => v.view === "thread" ? v.thread.id : null;
	const isRoomSelected = (id: string) => roomId() === id;
	return (
		<nav class="w-64 bg-bg1 text-fg2 overflow-y-auto">
			<ul class="p-1 flex flex-col">
					<li class="mt-1">
						<button
							class="px-1 py-0.25 w-full text-left hover:bg-bg4"
							classList={{ "bg-bg3": v.view === "home" }}
							onClick={() => ctx.dispatch({ do: "setView", to: { view: "home" } })}
						>home</button>
					</li>
				<For each={Object.values(ctx.data.rooms)}>
					{(room) => (
						<li class="mt-1">
							<button
								class="px-1 py-0.25 w-full text-left hover:bg-bg4"
								classList={{ "bg-bg3": isRoomSelected(room.id) }}
								onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
							>{room.name}</button>
							<Show when={isRoomSelected(room.id)}>
								<ul class="ml-6">
									<button
										class="px-1 py-0.25 w-full text-left hover:bg-bg4"
										classList={{ "bg-bg3": v.view === "room" }}
										onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
									>home</button>
									<button
										class="px-1 py-0.25 w-full text-left hover:bg-bg4"
										classList={{ "bg-bg3": v.view === "room-settings" }}
										onClick={() => ctx.dispatch({ do: "setView", to: { view: "room-settings", room }})}
									>settings</button>
									<For each={Object.values(ctx.data.threads).filter((i) => i.room_id === roomId())}>
										{(thread) => (
											<li class="mt-1">
												<button
													class="px-1 py-0.25 w-full text-left hover:bg-bg4"
													classList={{
														"bg-bg3": threadId() === thread.id,
														"text-sep": thread.is_closed,
													}}
													onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room, thread }})}
												>{thread.name}</button>
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
