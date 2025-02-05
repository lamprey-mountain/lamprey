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
import { MessageListAnchor } from "./api/messages.ts";

type ChatProps = {
	thread: ThreadT;
	room: RoomT;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();

	const read_marker_id = () => ctx.thread_read_marker_id.get(props.thread.id);

	const anchor = (): MessageListAnchor => {
		const a = ctx.thread_anchor.get(props.thread.id);
		const r = read_marker_id();
		if (a) return a;
		if (r) return { type: "context", limit: 50, message_id: r };
		return { type: "backwards", limit: 50 };
	};

	const messages = api.messages.list(() => props.thread.id, anchor);
	const [tl, setTl] = createSignal<Array<TimelineItemT>>([]);

	const markRead = throttle(
		() => {
			ctx.dispatch({
				do: "thread.mark_read",
				thread_id: props.thread.id,
				delay: true,
				version_id: props.thread.last_version_id,
			});
		},
		300,
	);

	const autoscroll = () =>
		!messages()?.has_forward && anchor().type !== "context";

	const list = createList({
		items: tl,
		autoscroll,
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

	// TODO: all of these effects are kind of annoying to work with - there has to be a better way
	// its also quite brittle...

	// effect to update timeline
	createRenderEffect(
		on(() => [messages(), read_marker_id()] as const, ([m, rid]) => {
			if (m?.items.length) {
				console.log("render timeline", m.items, rid);
				console.time("rendertimeline");
				const rendered = renderTimeline({
					items: m.items,
					has_after: m.has_forward,
					has_before: m.has_backwards,
					read_marker_id: rid ?? null,
					// slice: { start: 0, end: 50 },
				});
				setTl((old) => [...reconcile(rendered)(old)]);
				anchor();
				console.timeEnd("rendertimeline");
			} else {
				console.log("tried to render empty timeline");
			}
		}),
	);

	// effect to initialize new threads
	createEffect(() => {
		const tid = props.thread.id;
		const rid = props.thread.last_read_id ?? props.thread.last_version_id;
		if (ctx.thread_read_marker_id.has(tid)) return;
		ctx.thread_read_marker_id.set(tid, rid);
	});

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

	// effect to restore saved scroll position
	let last_thread_id: string | undefined;
	createEffect(
		on(
			() => [tl(), messages.loading, anchor()] as const,
			([_tl, loading, anchor]) => {
				// make sure this runs after tl renders
				if (loading) return;
				queueMicrotask(() => {
					if (anchor.type === "context") {
						console.log("scroll to anchor");
						// TODO: is this safe and performant?
						const target = document.querySelector(
							`li[data-message-id="${anchor.message_id}"]`,
						);
						if (target) {
							console.log("scroll into view", target);
							target.scrollIntoView({
								// behavior: "smooth",
								behavior: "instant",
								block: "center",
							});
							last_thread_id = props.thread.id;
						} else {
							console.warn("couldn't find target to scroll to");
						}
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
			},
		),
	);

	// effect to highlight selected message
	createEffect(
		on(
			() =>
				[messages.loading, ctx.thread_highlight.get(props.thread.id)] as const,
			([loading, hl]) => {
				if (loading) return;
				if (!hl) return;
				console.log("scroll to anchor");
				// TODO: is this safe and performant?
				const target = document.querySelector(
					`li[data-message-id="${hl}"]`,
				);
				console.log("scroll into view + animate", hl, target);
				if (!target) {
					console.warn("couldn't find target to scroll to");
					return;
				}
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
				ctx.thread_highlight.delete(props.thread.id);
			},
		),
	);

	return (
		<div class="chat">
			<Show when={messages.loading}>
				<div class="loading">loading...</div>
			</Show>
			<list.List>
				{(item) => renderTimelineItem(props.thread, item)}
			</list.List>
			<Input thread={props.thread} />
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
