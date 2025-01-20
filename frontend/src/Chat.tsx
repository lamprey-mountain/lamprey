import { createEffect, createSignal, For, on, onMount, Show, untrack, useContext, } from "solid-js";
import Editor from "./Editor.tsx";
import { TimelineItemT, TimelineItem, getAttachment } from "./Messages.tsx";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { chatctx, ThreadState } from "./context.ts";
import { createList } from "./list.tsx";
import { ThreadT, RoomT, AttachmentT } from "./types.ts";
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
	const ts = () => ctx.data.thread_state[props.thread.id] as ThreadState | undefined;
	const reply = () => ctx.data.messages[ts()?.reply_id!];
	const hasSpaceTop = () => tl()?.[0]?.type === "hole" || slice()?.start > 0;
	const hasSpaceBottom = () => tl()?.at(-1)?.type === "hole" || slice()?.end < tl()?.length;

	const thread_id = props.thread.id;
	const read_id = props.thread.last_read_id;
	ctx.dispatch({ do: "thread.init", thread_id, read_id });
	
	createEffect(() => {
		const thread_id = props.thread.id;
		const read_id = props.thread.last_read_id;
		ctx.dispatch({ do: "thread.init", thread_id, read_id });
	});
	createEffect(() => {
		console.log(props.thread.id);
	});

	createEffect(on(() => (slice()?.start, slice()?.end, ts()?.read_marker_id, tl()), () => updateItems()));

  function updateItems() {
  	console.log("update items", {
  		slice: untrack(slice),
  		tl: untrack(tl),
  		ts: untrack(ts),
  	});
  	if (!slice()) return;
    const rawItems = tl()?.slice(slice().start, slice().end) ?? [];
    const newItems: Array<TimelineItemT> = [];
    const { read_marker_id } = ts()!;

    if (hasSpaceTop()) {
	    newItems.push({
	      type: "info",
	      id: "info",
	      header: !hasSpaceTop(),
	    });
	    newItems.push({
	      type: "spacer",
	      id: "spacer-top",
	    });
    } else {
	    newItems.push({
	      type: "spacer-mini2",
	      id: "spacer-top2",
	    });
	    newItems.push({
	      type: "info",
	      id: "info",
	      header: !hasSpaceTop(),
	    });
    }

    for (let i = 0; i < rawItems.length; i++) {
      const msg = rawItems[i];
      if (msg.type === "hole") continue;
      newItems.push({
        type: "message",
        id: msg.message.version_id,
        message: msg.message,
        separate: true,
        is_local: msg.type === "local",
        // separate: shouldSplit(messages[i], messages[i - 1]),
      });
      // if (msg.id - prev.originTs > 1000 * 60 * 5) return true;
      // items.push({
      //   type: "message",
      //   id: messages[i].id,
      //   message: messages[i],
      //   separate: true,
      //   // separate: shouldSplit(messages[i], messages[i - 1]),
      // });
      if (msg.message.id === read_marker_id && i !== rawItems.length - 1) {
        newItems.push({
          type: "unread-marker",
          id: "unread-marker",
        });
      }
    }
    
  	if (hasSpaceBottom()) {
      newItems.push({
        type: "spacer",
        id: "spacer-bottom"
      });
  	} else {
      newItems.push({
        type: "spacer-mini",
        id: "spacer-bottom-mini"
      });
  	}

  	const old = untrack(items)
  	console.log("new items", newItems);
    console.time("perf::updateItems");
    setItems((old) => [...reconcile(newItems)(old)]);
    console.timeEnd("perf::updateItems");
  	console.log("update items", {
  		old,
	  	new: newItems,
	  	diff: untrack(items).filter((i, x) => i !== old[x]),
	  });
  }
	
	const list = createList({
		items: () => items(),
		autoscroll: () => !hasSpaceBottom(),
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
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
      // setTimeout(() => paginating = false, 1000);
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
			const pos = ts()!.scroll_pos;
			if (!pos) return list.scrollTo(999999);
			list.scrollTo(pos);
		});
	}));

	// TODO: handle this with onSubmit if possible
	async function handleUpload(f: File) {
		console.log(f);
		const { media_id, upload_url } = await ctx.client.http("POST", "/api/v1/media", {
			filename: f.name,
			size: f.size,
		});
		const r = await fetch(upload_url, {
			method: "PATCH",
			headers: {
				authorization: ctx.client.token,
				"upload-offset": "0",
			},
			body: f,
		});
		if (!r.ok) {
			ctx.dispatch({ do: "modal.alert", text: "failed to upload: " + await r.text() });
			return;
		}
		const json = await r.json();
		ctx.dispatch({
			do: "thread.attachments",
			thread_id: props.thread.id,
			attachments: [...ts()!.attachments, json],
		});
		console.log(ts()!.attachments);
	}

	function removeAttachment(id: string) {
		ctx.dispatch({
			do: "thread.attachments",
			thread_id: props.thread.id,
			attachments: [...ts()!.attachments].filter(i => i.id !== id),
		});
	}

	// translate-y-[8px]
	// <header class="bg-bg3 border-b-[1px] border-b-sep flex items-center px-[4px]">
	// 	{props.thread.name} / 
	// </header>
	return (
		<div class="chat">
			<list.List>{item => <TimelineItem thread={props.thread} item={item} />}</list.List>
			<div class="input">
				<Show when={ts()?.reply_id}>
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
				<Show when={ts()?.attachments.length}>
					<ul class="attachments">
						<For each={ts()!.attachments}>{media => (
							<li onClick={() => removeAttachment(media.id)}>{getAttachment(media)}</li>
						)}</For>
					</ul>
				</Show>
				<Show when={ts()}>
					<Editor state={ts()!.editor_state} onUpload={handleUpload} placeholder="send a message..." />
				</Show>
			</div>
		</div>
	);
};

type MediaPreviewProps = {
	media: AttachmentT;
}
