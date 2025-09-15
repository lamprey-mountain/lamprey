import {
	createEffect,
	createRenderEffect,
	Match,
	on,
	Show,
	Switch,
} from "solid-js";
import { useCtx } from "./context.ts";
import { createList } from "./list.tsx";
import type { ThreadT } from "./types.ts";
import { renderTimelineItem, type TimelineItemT } from "./Messages.tsx";
import { Input } from "./Input.tsx";
import { useApi } from "./api.tsx";
import { createSignal } from "solid-js";
import { reconcile } from "solid-js/store";
import type { Message } from "sdk";
import { throttle } from "@solid-primitives/scheduled";
import type { MessageListAnchor } from "./api/messages.ts";
import { getMessageOverrideName, getMsgTs as get_msg_ts } from "./util.tsx";
import { uuidv7 } from "uuidv7";
import { Portal } from "solid-js/web";

type ChatProps = {
	thread: ThreadT;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();
	const { t } = useCtx();

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

	createEffect(() =>
		console.log(
			"msgs",
			messages.loading,
			messages.latest,
			messages.error,
			messages(),
		)
	);

	const markRead = throttle(
		() => {
			const version_id = props.thread.last_version_id;
			if (version_id) {
				ctx.dispatch({
					do: "thread.mark_read",
					thread_id: props.thread.id,
					delay: true,
					version_id,
					also_local: false,
				});
			}
		},
		300,
	);

	const autoscroll = () =>
		!messages()?.has_forward && anchor().type !== "context";

	let last_thread_id: string | undefined;
	let chatRef: HTMLDivElement | undefined;
	const list = createList({
		items: tl,
		autoscroll,
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
		onPaginate(dir) {
			// FIXME: this tends to fire an excessive number of times
			// it's not a problem when *actually* paginating, but is for eg. marking threads read or scrolling to replies
			console.log("paginate", dir, messages.loading);
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
		onRestore() {
			const a = anchor();
			if (a.type === "context") {
				// TODO: is this safe and performant?
				const target = chatRef?.querySelector(
					`article[data-message-id="${a.message_id}"]`,
				);
				console.log("scroll restore: to anchor", a.message_id, target);
				if (target) {
					last_thread_id = props.thread.id;
					target.scrollIntoView({
						behavior: "instant",
						block: "center",
					});
					const hl = ctx.thread_highlight.get(props.thread.id);
					if (hl) scrollAndHighlight(hl);
					return true;
				} else {
					console.warn("couldn't find target to scroll to");
					return false;
				}
			} else if (last_thread_id !== props.thread.id) {
				const pos = ctx.thread_scroll_pos.get(props.thread.id);
				console.log("scroll restore: load pos", pos);
				if (pos === undefined || pos === -1) {
					list.scrollTo(999999);
				} else {
					list.scrollTo(pos);
				}
				last_thread_id = props.thread.id;
				return true;
			} else {
				console.log("nothing special");
				return false;
			}
		},
	});

	// TODO: all of these effects are kind of annoying to work with - there has to be a better way
	// its also quite brittle...

	// effect to update timeline
	createRenderEffect(
		on(() => [messages(), read_marker_id()] as const, ([m, rid]) => {
			console.log(m);
			if (m?.items) {
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
				console.log("reconciled", tl());
				console.timeEnd("rendertimeline");
			} else {
				console.log("tried to render empty timeline");
			}
		}),
	);

	// effect to initialize new threads
	createEffect(on(() => props.thread.id, (thread_id) => {
		const rid = props.thread.last_read_id ?? props.thread.last_version_id;
		if (ctx.thread_read_marker_id.has(thread_id)) return;
		ctx.thread_read_marker_id.set(thread_id, rid);
	}));

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		ctx.thread_scroll_pos.set(props.thread.id, pos);
	}, 300);

	// called both during reanchor and when thread_highlight changes
	function scrollAndHighlight(hl?: string) {
		if (!hl) return;
		const target = chatRef?.querySelector(
			`li:has(article.message[data-message-id="${hl}"])`,
		);
		console.log("scroll highlight", hl, target);
		if (!target) {
			// console.warn("couldn't find target to scroll to");
			return;
		}
		// target.scrollIntoView({
		// 	behavior: "instant",
		// 	block: "nearest",
		// });
		// target.scrollIntoView({
		// 	behavior: "smooth",
		// 	block: "center",
		// });
		target.scrollIntoView({
			behavior: "instant",
			block: "center",
		});
		highlight(target);
		ctx.thread_highlight.delete(props.thread.id);
	}

	createEffect(
		on(() => ctx.thread_highlight.get(props.thread.id), scrollAndHighlight),
	);

	createEffect(on(list.scrollPos, setPos));

	const [dragging, setDragging] = createSignal(false);

	return (
		<div
			ref={chatRef}
			class="chat"
			data-thread-id={props.thread.id}
			role="log"
			onKeyDown={(e) => {
				// console.log(e);
				if (e.key === "Escape") {
					const thread_id = props.thread.id;

					// messages are approx. 20 px high, show 3 pages of messages
					const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

					ctx.thread_anchor.set(thread_id, {
						type: "backwards",
						limit: SLICE_LEN,
					});

					const version_id =
						api.messages.cacheRanges.get(thread_id)?.live.end ??
							props.thread.last_version_id;

					if (version_id) {
						ctx.dispatch({
							do: "thread.mark_read",
							thread_id: thread_id,
							delay: false,
							also_local: true,
							version_id,
						});
					}

					// HACK: i need to make the update order less jank
					setTimeout(() => {
						list.scrollTo(99999999);
					});
				} else if (e.key === "PageDown") {
					list.scrollBy(globalThis.innerHeight * .8, true);
				} else if (e.key === "PageUp") {
					list.scrollBy(-globalThis.innerHeight * .8, true);
				}
			}}
			onDragEnter={(e) => {
				e.preventDefault();
				setDragging(true);
			}}
			onDragOver={(e) => {
				e.preventDefault();
				setDragging(true);
			}}
			onDragLeave={(e) => {
				e.preventDefault();
				setDragging(false);
			}}
			onDrop={(e) => {
				e.preventDefault();
				setDragging(false);
				for (const file of Array.from(e.dataTransfer?.files ?? [])) {
					console.log(file);
					const local_id = uuidv7();
					ctx.dispatch({
						do: "upload.init",
						file,
						local_id,
						thread_id: props.thread.id,
					});
				}
			}}
		>
			<Show when={messages.loading}>
				<div class="loading">{t("loading")}</div>
			</Show>
			<list.List>
				{(item) => renderTimelineItem(props.thread, item)}
			</list.List>
			<Input thread={props.thread} />
			<Portal>
				<Show when={dragging()}>
					<div class="dnd-upload-message">
						<div class="inner">
							drop to upload
						</div>
					</div>
				</Show>
			</Portal>
		</div>
	);
};

export const ChatHeader = (props: ChatProps) => {
	return (
		<header class="chat-header">
			<b>{props.thread.name}</b>
			<Show when={props.thread.description}>
				<span class="dim" style="white-space:pre;font-size:1em">{"  -  "}</span>
				{props.thread.description}
			</Show>
			<Switch>
				<Match when={props.thread.deleted_at}>{" (removed)"}</Match>
				<Match when={props.thread.archived_at}>{" (archived)"}</Match>
			</Switch>
		</header>
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
	if (has_before) {
		newItems.push({
			type: "spacer",
			id: "spacer-top",
		});
	} else {
		newItems.push({
			type: "info",
			id: "thread-header",
			header: true,
		});
	}
	for (let i = 0; i < items.length; i++) {
		const msg = items[i];
		const prev = items[i - 1] as Message | undefined;
		if (prev) {
			const ts_a = get_msg_ts(msg);
			const ts_b = get_msg_ts(prev);
			if (ts_a.getDay() !== ts_b.getDay()) {
				newItems.push({
					type: "time-split",
					id: `${msg.id}-timesplit`,
					date: get_msg_ts(msg),
				});
			}
		}
		newItems.push({
			type: "message",
			id: msg.version_id + "/" + ("embeds" in msg ? msg.embeds.length : 0),
			message: msg,
			separate: prev ? shouldSplit(msg, prev) : true,
		});
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
	console.log("newItems", newItems);
	return newItems;
}

function highlight(el: Element) {
	el.animate([
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
}

const shouldSplitMemo = new WeakMap();
function shouldSplit(a: Message, b: Message) {
	const s1 = shouldSplitMemo.get(a);
	if (s1) return s1;
	const s2 = shouldSplitInner(a, b);
	shouldSplitMemo.set(a, s2);
	return s2;
}

function shouldSplitInner(a: Message, b: Message) {
	shouldSplitMemo;
	if (a.author_id !== b.author_id) return true;
	if (getMessageOverrideName(a) !== getMessageOverrideName(b)) return true;
	const ts_a = get_msg_ts(a);
	const ts_b = get_msg_ts(b);
	if (+ts_a - +ts_b > 1000 * 60 * 5) return true;
	return false;
}
