import { createEffect, on, Show, useContext } from "solid-js";
import { chatctx, ThreadState, TimelineItem, useCtx } from "./context.ts";
import { createList } from "./list.tsx";
import { RoomT, ThreadT } from "./types.ts";
import { throttle } from "@solid-primitives/scheduled";
import { renderTimelineItem } from "./Messages.tsx";
import { Input } from "./Input.tsx";
import { useApi } from "./api.tsx";
import { createSignal } from "solid-js";
import { MessageListAnchor } from "./api/messages.ts";
import { renderTimeline } from "./dispatch/messages.ts";

type ChatProps = {
	thread: ThreadT;
	room: RoomT;
};

const SLICE_LEN = 50;

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
		limit: SLICE_LEN,
	});

	const messages = api.messages.list(() => props.thread.id, anchor);

	createEffect(() => {
		console.log("update messages", messages())
		console.log("update messages error", messages.error)
	})

	function tl() {
		if (messages()?.length) {
			return renderTimeline({
				items: messages()!.map((i) => ({
					type: "remote",
					message: i,
				})) as Array<TimelineItem>,
				has_after: false,
				has_before: false,
				read_marker_id: ts()?.read_marker_id ?? null,
				slice: { start: 0, end: 50 },
			});
		} else {
			return [];
		}
	}

	function init() {
		const thread_id = props.thread.id;
		const read_id = props.thread.last_read_id ?? undefined;
		ctx.dispatch({ do: "thread.init", thread_id, read_id });
	}

	init();
	createEffect(init);

	const list = createList({
		items: tl,
		// autoscroll: () => !hasSpaceBottom(),
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
		onPaginate(dir) {
			if (messages.loading) return;
			
			const thread_id = props.thread.id;
			if (dir === "forwards") {
				// ctx.dispatch({ do: "paginate", dir: "f", thread_id });
				// const isAtEnd = ctx.data.slices[thread_id].end ===
				// 	ctx.data.timelines[thread_id].length;
				// if (isAtEnd) {
				// 	ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
				// }
				console.log("paginate forwards", messages.loading);
				// setAnchor({
				// 	type: "forwards",
				// 	limit: SLICE_LEN,
				// 	message_id: messages()?.at(-20)?.id,
				// });
			} else {
				// ctx.dispatch({ do: "paginate", dir: "b", thread_id });
				console.log("paginate backwards", messages.loading);
				setAnchor({
					type: "backwards",
					limit: SLICE_LEN,
					message_id: messages()?.[20]?.id,
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
					message: ctx.data.messages[message_id],
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
			console.log({ pos });
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
