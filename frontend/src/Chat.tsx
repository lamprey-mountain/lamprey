import { createEffect, createRenderEffect, on, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { createList } from "./list.tsx";
import { RoomT, ThreadT } from "./types.ts";
import { renderTimelineItem, TimelineItemT } from "./Messages.tsx";
import { Input } from "./Input.tsx";
import { useApi } from "./api.tsx";
import { createSignal } from "solid-js";
import { reconcile } from "solid-js/store";
import { Message } from "sdk";
import { throttle } from "@solid-primitives/scheduled";

type ChatProps = {
	thread: ThreadT;
	room: RoomT;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();

	const ts = () => ctx.data.thread_state[props.thread.id];
	const anchor = () =>
		ctx.thread_anchor.get(props.thread.id) ?? { type: "backwards", limit: 50 };
	const messages = api.messages.list(() => props.thread.id, anchor);
	const [tl, setTl] = createSignal<Array<TimelineItemT>>([]);

	const markRead = throttle(
		() =>
			ctx.dispatch({
				do: "thread.mark_read",
				thread_id: props.thread.id,
				delay: true,
			}),
		300,
	);

	const list = createList({
		items: tl,
		autoscroll: () => !messages()?.has_forward && anchor().type !== "context",
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
		onPaginate(dir) {
			// FIXME: this tends to fire an excessive number of times
			// it's not a problem when *actually* paginating, but is for eg. marking threads read or scrolling to replies
			if (messages.loading) return;
			const thread_id = props.thread.id;

			// messages are approx. 20 px high, show 3 pages of messages
			const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

			// scroll a page at a time
			const PAGINATE_LEN = SLICE_LEN / 3;

			const msgs = messages()!;
			if (dir === "forwards") {
				if (msgs.has_forward) {
					ctx.thread_anchor.set(thread_id, {
						type: "forwards",
						limit: SLICE_LEN,
						message_id: messages()?.items.at(-PAGINATE_LEN)?.id,
					});
				} else {
					ctx.thread_anchor.set(thread_id, {
						type: "backwards",
						limit: SLICE_LEN,
					});
					if (list.isAtBottom()) markRead();
				}
			} else {
				ctx.thread_anchor.set(thread_id, {
					type: "backwards",
					limit: SLICE_LEN,
					message_id: messages()?.items[PAGINATE_LEN]?.id,
				});
			}
		},
	});

	// effect to update timeline
	createRenderEffect(on(messages, (m) => {
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
			console.log("tried to render empty timeline");
		}
	}));

	// effect to initialize new threads
	createEffect(on(() => props.thread.id, (thread_id) => {
		const read_id = props.thread.last_read_id ?? undefined;
		ctx.dispatch({ do: "thread.init", thread_id, read_id });
	}));

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		console.log("set pos", pos);
		ctx.thread_scroll_pos.set(props.thread.id, pos);
	}, 300);

	createEffect(() => {
		list.scrollPos();
		setPos();
	});

	// effect to restore saved scroll position or scroll to selected message
	let last_thread_id: string | undefined;
	createEffect(on(() => [tl(), anchor()] as const, ([_tl, anchor]) => {
		// make sure this runs after tl renders
		if (messages.loading) return;
		queueMicrotask(() => {
			if (anchor.type === "context") {
				console.log("scroll to anchor");
				highlight(anchor.message_id);
			} else if (last_thread_id !== props.thread.id) {
				const pos = ctx.thread_scroll_pos.get(props.thread.id);
				console.log("get pos", pos);
				if (pos === undefined || pos === -1) {
					list.scrollTo(999999);
				} else {
					list.scrollTo(pos);
				}
				last_thread_id = props.thread.id;
			}
		});
	}));

	return (
		<div class="chat">
			<Show when={messages.loading}>
				<div class="loading">loading...</div>
			</Show>
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

function highlight(message_id: string) {
	// TODO: is this safe and performant?
	const target = document.querySelector(
		`li[data-message-id="${message_id}"]`,
	);
	if (target) {
		console.log("scroll into view + animate", target);
		target.scrollIntoView({
			// behavior: "smooth",
			behavior: "instant",
			block: "center",
		});
		target.animate([
			{
				boxShadow: "4px 0 0 -1px inset #cc1856",
				backgroundColor: "#cc185622",
				offset: 0,
			},
			{
				boxShadow: "4px 0 0 -1px inset #cc1856",
				backgroundColor: "#cc185622",
				offset: .8,
			},
			{
				boxShadow: "none",
				backgroundColor: "transparent",
				offset: 1,
			},
		], {
			duration: 1000,
		});
	} else {
		console.warn("couldn't find target to scroll to");
	}
}
