import { createEffect, on, Show } from "solid-js";
import { ThreadState, useCtx } from "./context.ts";
import { createList } from "./list.tsx";
import { RoomT, ThreadT } from "./types.ts";
import { renderTimelineItem, TimelineItemT } from "./Messages.tsx";
import { Input } from "./Input.tsx";
import { useApi } from "./api.tsx";
import { createSignal } from "solid-js";
import { MessageListAnchor } from "./api/messages.ts";
import { reconcile } from "solid-js/store";
import { Message } from "sdk";

type ChatProps = {
	thread: ThreadT;
	room: RoomT;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();

	// const slice = () => ctx.data.slices[props.thread.id];
	// const tl = () => ctx.data.timelines[props.thread.id];
	const ts = () =>
		ctx.data.thread_state[props.thread.id] as ThreadState | undefined;
	// const hasSpaceTop = () => tl()?.[0]?.type === "hole" || slice()?.start > 0;
	// const hasSpaceBottom = () =>
	// 	tl()?.at(-1)?.type === "hole" || slice()?.end < tl()?.length;

	const [anchor, setAnchor] = createSignal<MessageListAnchor>({
		type: "backwards",
		limit: 50, // TODO: calculate dynamically
	});

	const messages = api.messages.list(() => props.thread.id, anchor);

	const [tl, setTl] = createSignal<Array<TimelineItemT>>([]);

	createEffect(() => {
		const m = messages();
		if (m?.items.length) {
			console.log("render timeline", m.items);
			console.time("rendertimeline");
			const rendered = renderTimeline({
				items: m.items,
				has_after: m.has_forward,
				has_before: m.has_backwards,
				read_marker_id: ts()?.read_marker_id ?? null,
				// slice: { start: 0, end: 50 },
			});
			setTl((old) => [...reconcile(rendered)(old)]);
			console.timeEnd("rendertimeline");
		} else {
			setTl([]);
		}
	});

	function init() {
		const thread_id = props.thread.id;
		const read_id = props.thread.last_read_id ?? undefined;
		ctx.dispatch({ do: "thread.init", thread_id, read_id });
	}

	init();
	createEffect(init);

	const list = createList({
		items: tl,
		autoscroll: () => !messages()?.has_forward,
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
		onPaginate(dir) {
			if (messages.loading) return;
			// const thread_id = props.thread.id;

			// messages are approx. 20 px high, show 3 pages of messages
			const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

			// scroll a page at a time
			const PAGINATE_LEN = SLICE_LEN / 3;

			const msgs = messages()!;
			if (dir === "forwards") {
				if (msgs.has_forward) {
					setAnchor({
						type: "forwards",
						limit: SLICE_LEN,
						message_id: messages()?.items.at(-PAGINATE_LEN)?.id,
					});
				} else {
					setAnchor({
						type: "backwards",
						limit: SLICE_LEN,
					});
					// ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
				}
			} else {
				setAnchor({
					type: "backwards",
					limit: SLICE_LEN,
					message_id: messages()?.items[PAGINATE_LEN]?.id,
				});
			}
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
					message: api.messages.cache.get(message_id)!,
				},
			});
		},
	});

	// createEffect(() => {
	// 	list.scrollPos();
	// 	throttle(() => {
	// 		init(); // FIXME: don't init on all scroll
	// 		ctx.dispatch({
	// 			do: "thread.scroll_pos",
	// 			thread_id: props.thread.id,
	// 			pos: list.scrollPos(),
	// 			is_at_end: list.isAtBottom(),
	// 		});
	// 	});
	// });

	// createEffect(() => {
	// 	if (slice()?.start === undefined) {
	// 		ctx.dispatch({
	// 			do: "paginate",
	// 			dir: "b",
	// 			thread_id: props.thread.id,
	// 		});
	// 	}
	// });

	createEffect(on(() => props.thread, () => {
		// TODO: restore scroll position
		queueMicrotask(() => {
			const pos = ts()!.scroll_pos;
			// console.log({ pos });
			if (pos === null) return list.scrollTo(999999);
			list.scrollTo(pos);
		});
	}));

	return (
		<div class="chat">
			<list.List>
				{(item) => renderTimelineItem(props.thread, item)}
			</list.List>
			<Show when={ts()}>
				<Input ts={ts()!} thread={props.thread} />
			</Show>
		</div>
	);
};

type RenderTimelineParams = {
	items: Array<Message>;
	read_marker_id: string | null;
	has_before: boolean;
	has_after: boolean;
};

export function renderTimeline(
	{ items, read_marker_id, has_before, has_after }: RenderTimelineParams,
): Array<TimelineItemT> {
	const newItems: Array<TimelineItemT> = [];
	if (items.length === 0) throw new Error("no items");
	if (has_before) {
		newItems.push({
			type: "info",
			id: "info",
			header: false,
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
			header: true,
		});
	}
	for (let i = 0; i < items.length; i++) {
		const msg = items[i];
		newItems.push({
			type: "message",
			id: msg.version_id,
			message: msg,
			separate: true,
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
		if (msg.id === read_marker_id && i !== items.length - 1) {
			newItems.push({
				type: "unread-marker",
				id: "unread-marker",
			});
		}
	}
	if (has_after) {
		newItems.push({
			type: "spacer",
			id: "spacer-bottom",
		});
	} else {
		newItems.push({
			type: "spacer-mini",
			id: "spacer-bottom-mini",
		});
	}
	return newItems;
}
