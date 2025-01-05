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
import { createList, SliceInfo, TimelineItemT, TimelineStatus } from "./list.tsx";
import { ThreadT, RoomT } from "./types.ts";
import { reconcile } from "solid-js/store";

type ChatProps = {
	thread: ThreadT,
	room: RoomT,
}

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

	let paginating = false;
	const slice = () => ctx.data.slices[props.thread.id];
	
	if (!slice()) {
    ctx.dispatch({ do: "paginate", dir: "b", thread_id: props.thread.id });
	}

  const [items, setItems] = createSignal<Array<TimelineItemT>>([]);

	createEffect(() => slice() && updateItems());
	createEffect(() => console.log(slice()));
	createEffect(() => console.log(items()));

  function updateItems() {
    const items: Array<TimelineItemT> = [];
    items.push({
      type: "info",
      key: "info" + slice().is_at_beginning,
      header: slice().is_at_beginning,
      class: "header",
    });
    if (!slice().is_at_beginning) {
      items.push({ type: "spacer", key: "space-begin" });
    } else {
      items.push({ type: "spacer", key: "space-begin" });
    }
    const messages = slice().messages;

    for (let i = 0; i < messages.length; i++) {
      const msg = messages[i]
      items.push({
        type: "message",
        key: msg.id,
        message: msg,
        separate: true,
        // separate: shouldSplit(messages[i], messages[i - 1]),
      });
      // if (msg.id - prev.originTs > 1000 * 60 * 5) return true;
      // items.push({
      //   type: "message",
      //   key: messages[i].id,
      //   message: messages[i],
      //   separate: true,
      //   // separate: shouldSplit(messages[i], messages[i - 1]),
      // });
      // if (events[i].id === lastAck) {
      //   items.push({
      //     type: "unread-marker",
      //     key: "unread-marker",
      //   });
      // }
    }
    if (slice().is_at_end) {
      items.push({ type: "spacer-mini", key: "space-end-mini" });
    } else {
      items.push({ type: "spacer", key: "space-end" });
    }
    // items.push({ type: "editor", key: "editor" });
    console.time("perf::updateItems");
    setItems((old) => [...reconcile(items, { key: "key" })(old)]);
    console.timeEnd("perf::updateItems");
  }
	
	const list = createList({
		items: () => items(),
		autoscroll: () => slice()?.is_at_end,
    // topPos: () => tl.isAtBeginning() ? 1 : 2,
    topPos: () => 0,
    // bottomPos: () => timel.isAtEnd() ? timel.items().length - 1 : timel.items().length - 2,
    bottomPos: () => (slice()?.messages.length ?? 0) - 2,
    onPaginate(dir) {
      if (paginating) return;
      paginating = true;
      console.log({ dir })
      if (dir === "forwards") {
	      ctx.dispatch({ do: "paginate", dir: "f", thread_id: props.thread.id });
      } else {
	      ctx.dispatch({ do: "paginate", dir: "b", thread_id: props.thread.id });
      }
      paginating = false;
      // tl.setIsAutoscrolling(tl.isAtEnd());
    }, 
	}); 

	// translate-y-[8px]
	return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[24px_1fr_0] relative">
			<header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
				{props.thread.name} / 
				{props.thread.description ?? "(no description)" } /
				<Show when={props.thread.is_closed}> (archived)</Show>
			</header>
			<list.List>{item => <TimelineItem item={item} />}</list.List>
			<div class="absolute bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[138px] pr-[4px] max-h-50%">
				<Editor onSubmit={handleSubmit} placeholder="send a message..." />
			</div>
		</div>
	);
};
