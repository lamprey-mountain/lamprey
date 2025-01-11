import { createEffect, createSignal, on, onMount, Show, useContext, } from "solid-js";
import Editor from "./Editor.tsx";
import { TimelineItem } from "./Messages.tsx";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { chatctx } from "./context.ts";
import { createList, TimelineItemT } from "./list.tsx";
import { ThreadT, RoomT } from "./types.ts";
import { reconcile } from "solid-js/store";

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
	const ts = () => ctx.data.thread_state[props.thread.id];
	const reply = () => ctx.data.messages[ts().reply_id!];
	const hasSpaceTop = () => tl()?.[0]?.type === "hole" || slice()?.start > 0;
	const hasSpaceBottom = () => tl()?.at(-1)?.type === "hole" || slice()?.end < tl()?.length;

	ctx.dispatch({ do: "thread.init", thread_id: props.thread.id });
	createEffect(on(() => (slice()?.start, slice()?.end, ts().read_marker_id, tl()), () => updateItems()));

  function updateItems() {
  	console.log("update items", slice(), tl())
  	if (!slice()) return;
    const rawItems = tl()?.slice(slice().start, slice().end) ?? [];
    const items: Array<TimelineItemT> = [];
    const { read_marker_id } = ts();

    if (hasSpaceTop()) {
	    items.push({
	      type: "info",
	      key: "info",
	      header: !hasSpaceTop(),
	    });
	    items.push({
	      type: "spacer",
	      key: "spacer-top",
	    });
    } else {
	    items.push({
	      type: "spacer-mini2",
	      key: "spacer-top2",
	    });
	    items.push({
	      type: "info",
	      key: "info",
	      header: !hasSpaceTop(),
	    });
    }

    for (let i = 0; i < rawItems.length; i++) {
      const msg = rawItems[i];
      if (msg.type === "hole") continue;
      items.push({
        type: "message",
        key: msg.message.version_id,
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
      if (msg.message.id === read_marker_id && i !== rawItems.length - 1) {
        items.push({
          type: "unread-marker",
          key: "unread-marker",
        });
      }
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
      const thread_id = props.thread.id;
      if (dir === "forwards") {
	      await ctx.dispatch({ do: "paginate", dir: "f", thread_id });
				const isAtEnd = ctx.data.slices[thread_id].end === ctx.data.timelines[thread_id].length;
				if (isAtEnd) {
		    	ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
				}
      } else {
	      await ctx.dispatch({ do: "paginate", dir: "b", thread_id });
      }
      paginating = false;
    },
	  onContextMenu(e: MouseEvent) {
	  	e.stopPropagation();
	  	const target = e.target as HTMLElement;
	  	const media_el = target.closest("a, img, video, audio") as HTMLElement;
	  	const message_el = target.closest("li[data-message-id]") as HTMLElement;
	  	const message_id = message_el?.dataset.messageId;
	  	if (!message_id || (media_el && message_el.contains(media_el))) {
		  	ctx.dispatch({
		  		do: "menu",
					menu: null,
		  	});
	  		return;
  		}
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

	// translate-y-[8px]
	// <header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
	// 	{props.thread.name} / 
	// </header>
	
	return (
		<div class="chat">
			<list.List>{item => <TimelineItem thread={props.thread} item={item} />}</list.List>
			<div class="input">
				<Show when={ts().reply_id}>
					<div class="reply">
						<button
							class="cancel"
							onClick={() => ctx.dispatch({ do: "thread.reply", thread_id: props.thread.id, reply_id: null })}
						>
							cancel
						</button>
						<div class="info">
							replying to {reply()?.override_name ?? reply()?.author.name}: {reply()?.content}
						</div>
					</div>
				</Show>
				<Show when={ts().attachments.length && false}>
					<div class="attachments">
						<div>foo</div>
						<div>bar</div>
						<div>baz</div>
					</div>
				</Show>
				<Editor state={ts().state} placeholder="send a message..." />
			</div>
		</div>
	);
};
