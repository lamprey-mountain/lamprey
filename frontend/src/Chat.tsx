import { createEffect, createSignal, on, onMount, Show, useContext, } from "solid-js";
import Editor from "./Editor.tsx";
import { TimelineItem } from "./Messages.tsx";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { chatctx } from "./context.ts";
import { createList, TimelineItemT } from "./list.tsx";
import { ThreadT, RoomT } from "./types.ts";
import { reconcile } from "solid-js/store";
import { CLASS_BUTTON } from "./styles.ts";

type ChatProps = {
	thread: ThreadT,
	room: RoomT,
}

export const ChatMain = (props: ChatProps) => {
	const ctx = useContext(chatctx)!;
	
	let paginating = false;
  const [items, setItems] = createSignal<Array<TimelineItemT>>([]);
	const slice = () => ctx.data.slices[props.thread.id];
  const tl = () => ctx.data.timelines[props.thread.id];
	const hasSpaceTop = () => tl()?.[0]?.type === "hole" || slice()?.start > 0;
	const hasSpaceBottom = () => tl()?.at(-1)?.type === "hole" || slice()?.end < tl()?.length;
	createEffect(on(() => (slice()?.start, slice()?.end), () => updateItems()));

  function updateItems() {
  	console.log("update items", slice())
  	if (!slice()) return;
    const rawItems = tl()?.slice(slice().start, slice().end) ?? [];
    const items: Array<TimelineItemT> = [];
    items.push({
      type: "spacer",
      key: "spacer-top",
    });
    items.push({
      type: "info",
      key: "info",
      header: !hasSpaceTop(),
    });

    for (let i = 0; i < rawItems.length; i++) {
      const msg = rawItems[i];
      if (msg.type === "hole") continue;
      items.push({
        type: "message",
        key: msg.message.id,
        message: msg.message,
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
    
  	if (hasSpaceBottom()) {
      items.push({
        type: "spacer",
        key: "spacer-bottom"
      });
  	} else {
      items.push({
        type: "spacer-mini",
        key: "spacer-bottom-mini"
      });
  	}
  	
  	console.log("new items", items);
    console.time("perf::updateItems");
    setItems((old) => [...reconcile(items, { key: "key" })(old)]);
    console.timeEnd("perf::updateItems");
  }
	
	const list = createList({
		items: () => items(),
		autoscroll: () => !hasSpaceBottom(),
		// topPos: () => hasSpaceTop() ? 1 : 0,
		// topPos: () => hasSpaceTop() ? 1 : 2,
		topPos: () => 2,
		bottomPos: () => items().length - 2,
    async onPaginate(dir) {
      if (paginating) return;
      paginating = true;
      if (dir === "forwards") {
	      await ctx.dispatch({ do: "paginate", dir: "f", thread_id: props.thread.id });
      } else {
	      await ctx.dispatch({ do: "paginate", dir: "b", thread_id: props.thread.id });
      }
      paginating = false;
    },
	  onContextMenu(e: MouseEvent) {
	  	const target = e.target as HTMLElement;
	  	const message_el = target.closest("li[data-message-id]") as HTMLElement;
	  	const message_id = message_el?.dataset.messageId;
	  	if (!message_id) return;
	  	e.preventDefault();
	  	ctx.dispatch({
	  		do: "menu",
				menu: {
					type: "message",
					x: e.x,
					y: e.y,
					message: ctx.data.messages[message_id],
				}
	  	});
	  },
	  onScroll(pos) {
	  	ctx.dispatch({
	  		do: "thread.scroll_pos",
	  		thread_id: props.thread.id,
	  		pos,
	  	});
	  },
	});

	createEffect(async () => {
		if (slice()?.start === undefined) {
      if (paginating) return;
      paginating = true;
      await ctx.dispatch({ do: "paginate", dir: "b", thread_id: props.thread.id });
      list.scrollTo(999999);
      paginating = false;
		}
	});

	createEffect(on(() => props.thread, () => {
		// TODO: restore scroll position
		queueMicrotask(() => {
			const pos = ts().scroll_pos;
			if (!pos) return list.scrollTo(999999);
			list.scrollTo(pos);
		});
	}));

	ctx.dispatch({ do: "thread.init", thread_id: props.thread.id });
	const ts = () => ctx.data.thread_state[props.thread.id];
	const reply = () => ctx.data.messages[ts().reply_id!];

	// translate-y-[8px]
	
	return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[48px_1fr_0] relative">
			<header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
				{props.thread.name} / 
				{props.thread.description ?? "(no description)" } /
				<Show when={props.thread.is_closed}> (archived)</Show>
			</header>
			<list.List>{item => <TimelineItem item={item} />}</list.List>
			<div class="absolute bottom-0 w-full bg-gradient-to-t from-bg2 from-25% flex py-[4px] pl-[138px] pr-[4px] max-h-50% flex-col">
				<Show when={ts().reply_id}>
					<div class="bg-bg2 m-0 flex relative mb-[-1px]">
						<button
							class={CLASS_BUTTON + " my-0 mr-[-1px] border-[1px] border-sep absolute right-[100%]"}
							onClick={() => ctx.dispatch({ do: "thread.reply", thread_id: props.thread.id, reply_id: null })}
						>
							cancel
						</button>
						<div class="px-[4px] bg-bg1/80 flex-1 border-[1px] border-sep">
							{ts().reply_id}
							replying to {reply()?.override_name ?? reply()?.author.name}: {reply()?.content}
						</div>
					</div>
				</Show>
				<Editor state={ts().state} class="shadow-asdf shadow-[#1114]" placeholder="send a message..." />
			</div>
		</div>
	);
};
