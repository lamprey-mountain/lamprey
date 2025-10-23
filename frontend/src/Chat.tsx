import {
	createEffect,
	createRenderEffect,
	For,
	Match,
	on,
	Show,
	Switch,
	useContext,
} from "solid-js";
import { useCtx } from "./context.ts";
import { createList } from "./list.tsx";
import type { Channel } from "sdk";
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
import { useNavigate } from "@solidjs/router";
import type { ThreadSearch } from "./context.ts";
import { MessageView } from "./Message.tsx";
import { SearchInput } from "./SearchInput.tsx";

type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();
	const { t } = useCtx();

	const read_marker_id = () => ctx.channel_read_marker_id.get(props.channel.id);

	const anchor = (): MessageListAnchor => {
		const a = ctx.channel_anchor.get(props.channel.id);
		const r = read_marker_id();
		if (a) return a;
		if (r) return { type: "context", limit: 50, message_id: r };
		return { type: "backwards", limit: 50 };
	};

	const messages = api.messages.list(() => props.channel.id, anchor);
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
			const version_id = props.channel.last_version_id;
			if (version_id) {
				ctx.dispatch({
					do: "channel.mark_read",
					channel_id: props.channel.id,
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
			// it's not a problem when *actually* paginating, but is for eg. marking channels read or scrolling to replies
			console.log("paginate", dir, messages.loading);
			if (messages.loading) return;
			const channel_id = props.channel.id;

			// messages are approx. 20 px high, show 3 pages of messages
			const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

			// scroll a page at a time
			const PAGINATE_LEN = SLICE_LEN / 3;

			const msgs = messages()!;
			if (dir === "forwards") {
				if (msgs.has_forward) {
					ctx.channel_anchor.set(channel_id, {
						type: "forwards",
						limit: SLICE_LEN,
						message_id: messages()?.items.at(-PAGINATE_LEN)?.id,
					});
				} else {
					ctx.channel_anchor.set(channel_id, {
						type: "backwards",
						limit: SLICE_LEN,
					});

					if (list.isAtBottom()) markRead();
				}
			} else {
				ctx.channel_anchor.set(channel_id, {
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
					last_thread_id = props.channel.id;
					target.scrollIntoView({
						behavior: "instant",
						block: "center",
					});
					const hl = ctx.channel_highlight.get(props.channel.id);
					if (hl) scrollAndHighlight(hl);
					return true;
				} else {
					console.warn("couldn't find target to scroll to");
					return false;
				}
			} else if (last_thread_id !== props.channel.id) {
				const pos = ctx.channel_scroll_pos.get(props.channel.id);
				console.log("scroll restore: load pos", pos);
				if (pos === undefined || pos === -1) {
					list.scrollTo(999999);
				} else {
					list.scrollTo(pos);
				}
				last_thread_id = props.channel.id;
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

	// effect to initialize new channels
	createEffect(on(() => props.channel.id, (channel_id) => {
		const last_read_id = props.channel.last_read_id ??
			props.channel.last_version_id;
		if (ctx.channel_read_marker_id.has(channel_id)) return;
		if (!last_read_id) return; // no messages in the channel
		ctx.channel_read_marker_id.set(channel_id, last_read_id);
	}));

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		ctx.channel_scroll_pos.set(props.channel.id, pos);
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
		ctx.channel_highlight.delete(props.channel.id);
	}

	createEffect(
		on(() => ctx.channel_highlight.get(props.channel.id), scrollAndHighlight),
	);

	createEffect(on(list.scrollPos, setPos));

	const [dragging, setDragging] = createSignal(false);

	const getTyping = () => {
		// TODO: fix types here
		const user_id = api.users.cache.get("@self")?.id;
		const user_ids = [...api.typing.get(props.channel.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return user_ids;
	};

	return (
		<div
			ref={chatRef}
			class="chat"
			classList={{ "has-typing": !!getTyping().length }}
			data-channel-id={props.channel.id}
			role="log"
			onKeyDown={(e) => {
				// console.log(e);
				if (e.key === "Escape") {
					const channel_id = props.channel.id;

					// messages are approx. 20 px high, show 3 pages of messages
					const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

					ctx.channel_anchor.set(channel_id, {
						type: "backwards",
						limit: SLICE_LEN,
					});

					const version_id =
						api.messages.cacheRanges.get(channel_id)?.live.end ??
							props.channel.last_version_id;

					if (version_id) {
						ctx.dispatch({
							do: "channel.mark_read",
							channel_id: channel_id,
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
						channel_id: props.channel.id,
					});
				}
			}}
		>
			<Show when={messages.loading}>
				<div class="loading">{t("loading")}</div>
			</Show>
			<list.List>
				{(item) => renderTimelineItem(props.channel, item)}
			</list.List>
			<Input channel={props.channel} />
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

import { usePermissions } from "./hooks/usePermissions.ts";

export const ChatHeader = (
	props: ChatProps & { showMembersButton?: boolean },
) => {
	const ctx = useCtx();
	const api = useApi();

	const selected = () => ctx.selectedMessages.get(props.channel.id) ?? [];
	const inSelectMode = () => ctx.selectMode.get(props.channel.id) ?? false;

	const { has: hasPermission } = usePermissions(
		() => api.users.cache.get("@self")?.id,
		() => props.channel.room_id,
		() => props.channel.id,
	);
	const canDelete = () => hasPermission("MessageDelete");
	const canRemove = () => hasPermission("MessageRemove");

	const exitSelectMode = () => {
		ctx.selectMode.set(props.channel.id, false);
		ctx.selectedMessages.set(props.channel.id, []);
	};

	const deleteSelected = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: `Are you sure you want to delete ${selected().length} messages?`,
			cont: (confirmed) => {
				if (!confirmed) return;
				api.messages.deleteBulk(props.channel.id, selected());
				exitSelectMode();
			},
		});
	};

	const removeSelected = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: `Are you sure you want to remove ${selected().length} messages?`,
			cont: (confirmed) => {
				if (!confirmed) return;
				api.messages.removeBulk(props.channel.id, selected());
				exitSelectMode();
			},
		});
	};

	const toggleMembers = () => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				showMembers: !(c.frontend.showMembers ?? true),
			},
		});
	};

	const isShowingPinned = () =>
		ctx.channel_pinned_view.get(props.channel.id) ?? false;

	const togglePinned = () => {
		ctx.channel_pinned_view.set(props.channel.id, !isShowingPinned());
	};

	const name = () => {
		if (props.channel.type === "Dm") {
			const user_id = api.users.cache.get("@self")?.id;
			return props.channel.recipients.find((i) => i.id !== user_id)?.name ??
				"dm";
		}

		return props.channel.name;
	};

	return (
		<Show
			when={inSelectMode()}
			fallback={
				<header class="chat-header" style="display:flex">
					<b>{name()}</b>
					<Show when={props.channel.description}>
						<span class="dim" style="white-space:pre;font-size:1em">
							{"  -  "}
						</span>
						{props.channel.description}
					</Show>
					<Switch>
						<Match when={props.channel.deleted_at}>{" (removed)"}</Match>
						<Match when={props.channel.archived_at}>{" (archived)"}</Match>
					</Switch>
					<div style="flex:1"></div>
					<SearchInput channel={props.channel} />
					<button
						onClick={togglePinned}
						classList={{ active: isShowingPinned() }}
						title="Show pinned messages"
					>
						pins
					</button>
					<Show when={props.showMembersButton ?? true}>
						<button
							onClick={toggleMembers}
							title="Show members"
						>
							members
						</button>
					</Show>
				</header>
			}
		>
			<header class="chat-header select-mode-header" style="display:flex">
				<span>{selected().length} selected</span>
				<div style="flex:1"></div>
				<Show when={canDelete()}>
					<button onClick={deleteSelected}>Delete</button>
				</Show>
				<Show when={canRemove()}>
					<button onClick={removeSelected}>Remove</button>
				</Show>
				<button onClick={exitSelectMode}>Cancel</button>
			</header>
		</Show>
	);
};

const SearchResultItem = (props: {
	message: Message;
	prevMessage?: Message;
	onResultClick: (message: Message) => void;
}) => {
	const api = useApi();
	const channel = api.channels.fetch(() => props.message.channel_id);
	const showHeader = () =>
		!props.prevMessage ||
		props.prevMessage.channel_id !== props.message.channel_id;

	return (
		<>
			<Show when={showHeader() && channel()}>
				<div style="padding: 4px 12px 0; font-weight: bold; color: var(--text-dim);">
					#{channel()!.name}
				</div>
			</Show>
			<li onClick={() => props.onResultClick(props.message)}>
				<MessageView message={props.message} separate={true} />
			</li>
		</>
	);
};

export const SearchResults = (props: {
	channel: Channel;
	search: ThreadSearch;
}) => {
	const ctx = useCtx();
	const navigate = useNavigate();

	const onResultClick = (message: Message) => {
		navigate(`/channel/${message.channel_id}/message/${message.id}`);
		ctx.channel_search.delete(props.channel.id);
	};

	return (
		<aside class="search-results">
			<header>
				<Show when={!props.search.loading} fallback={<>Searching...</>}>
					{props.search.results?.total ?? 0} results
				</Show>
				<button onClick={() => ctx.channel_search.delete(props.channel.id)}>
					Clear
				</button>
			</header>
			<Show when={!props.search.loading}>
				<ul>
					<For each={props.search.results?.items}>
						{(message, index) => {
							const prev = () => {
								const i = index();
								if (i > 0) return props.search.results!.items[i - 1];
								return undefined;
							};
							return (
								<SearchResultItem
									message={message}
									prevMessage={prev()}
									onResultClick={onResultClick}
								/>
							);
						}}
					</For>
				</ul>
			</Show>
		</aside>
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
		const markerTime = prev &&
			get_msg_ts(msg).getDay() !== get_msg_ts(prev).getDay();
		const markerUnread = prev?.id === read_marker_id;
		if (markerTime || markerUnread) {
			newItems.push({
				type: "divider",
				id: `divider-${msg.id}-${markerUnread}`,
				date: markerTime ? get_msg_ts(msg) : undefined,
				unread: markerUnread,
			});
		}
		newItems.push({
			type: "message",
			id: msg.version_id + "/" + ("embeds" in msg ? msg.embeds.length : 0),
			message: msg,
			separate: prev ? shouldSplit(msg, prev) : true,
		});
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
	if (a.type !== "DefaultMarkdown") return true;
	if (b.type !== "DefaultMarkdown") return true;
	if (a.author_id !== b.author_id) return true;
	if (getMessageOverrideName(a) !== getMessageOverrideName(b)) return true;
	const ts_a = get_msg_ts(a);
	const ts_b = get_msg_ts(b);
	if (+ts_a - +ts_b > 1000 * 60 * 5) return true;
	return false;
}
