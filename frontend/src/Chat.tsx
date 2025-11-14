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
import type { Channel, Room } from "sdk";
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
import { md } from "./markdown.tsx";
import icPin from "./assets/pin.png";
import icMembers from "./assets/members.png";
import { useChannel } from "./channelctx.tsx";

type ChatProps = {
	channel: Channel;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useCtx();
	const api = useApi();
	const { t } = useCtx();
	const [channelState, setChannelState] = useChannel()!;

	const read_marker_id = () => channelState.read_marker_id;

	const anchor = (): MessageListAnchor => {
		const a = channelState.anchor;
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
					setChannelState("anchor", {
						type: "forwards",
						limit: SLICE_LEN,
						message_id: messages()?.items.at(-PAGINATE_LEN)?.id,
					});
				} else {
					setChannelState("anchor", {
						type: "backwards",
						limit: SLICE_LEN,
					});

					if (list.isAtBottom()) markRead();
				}
			} else {
				setChannelState("anchor", {
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
					const hl = channelState.highlight;
					if (hl) scrollAndHighlight(hl);
					return true;
				} else {
					console.warn("couldn't find target to scroll to");
					return false;
				}
			} else if (last_thread_id !== props.channel.id) {
				const pos = channelState.scroll_pos;
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
		if (channelState.read_marker_id) return;
		if (!last_read_id) return; // no messages in the channel
		setChannelState("read_marker_id", last_read_id);
	}));

	// effect to update saved scroll position
	const setPos = throttle(() => {
		const pos = list.isAtBottom() ? -1 : list.scrollPos();
		setChannelState("scroll_pos", pos);
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
		setChannelState("highlight", undefined);
	}

	createEffect(
		on(() => channelState.highlight, scrollAndHighlight),
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

	const uploads = useUploads();

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

					setChannelState("anchor", {
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
					uploads.init(local_id, props.channel.id, file);
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
import { ChannelIcon } from "./User.tsx";
import { useUploads } from "./contexts/uploads.tsx";

export const ChatHeader = (
	props: ChatProps & { showMembersButton?: boolean },
) => {
	const ctx = useCtx();
	const api = useApi();
	const [channelState, setChannelState] = useChannel()!;

	const selected = () => channelState.selectedMessages;
	const inSelectMode = () => channelState.selectMode;

	const { has: hasPermission } = usePermissions(
		() => api.users.cache.get("@self")?.id,
		() => props.channel.room_id,
		() => props.channel.id,
	);
	const canDelete = () => hasPermission("MessageDelete");
	const canRemove = () => hasPermission("MessageRemove");

	const exitSelectMode = () => {
		setChannelState("selectMode", false);
		setChannelState("selectedMessages", []);
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

	const isShowingPinned = () => channelState.pinned_view;

	const togglePinned = () => {
		setChannelState("pinned_view", (v) => !v);
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
					<ChannelIcon channel={props.channel} />
					<b>{name()}</b>
					<Show when={props.channel.description}>
						<span class="dim" style="white-space:pre;font-size:1em">
							{"  -  "}
						</span>
						<span
							class="markdown"
							innerHTML={md(props.channel.description ?? "") as string}
						>
						</span>
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
						<img class="icon" src={icPin} />
					</button>
					<Show when={props.showMembersButton ?? true}>
						<button
							onClick={toggleMembers}
							title="Show members"
						>
							<img class="icon" src={icMembers} />
						</button>
					</Show>
				</header>
			}
		>
			<header class="chat-header select-mode-header" style="display:flex">
				<ChannelIcon channel={props.channel} />
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

export const RoomHeader = (
	props: { room: Room },
) => {
	const ctx = useCtx();

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

	return (
		<header
			class="chat-header menu-room"
			style="display:flex"
			data-room-id={props.room.id}
		>
			<b>home</b>
			<div style="flex:1"></div>
			{/* <SearchInput room={props.room} /> */}
			<button
				onClick={toggleMembers}
				title="Show members"
			>
				<img class="icon" src={icMembers} />
			</button>
		</header>
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
	channel?: Channel;
	room?: Room;
	search: ThreadSearch;
}) => {
	const ctx = useCtx();
	const [channelCtx, setChannelState] = useChannel()!;
	const navigate = useNavigate();

	const searchId = () => props.channel?.id ?? props.room?.id;

	const onResultClick = (message: Message) => {
		navigate(`/channel/${message.channel_id}/message/${message.id}`);
		const id = searchId();
		if (id) {
			if (props.channel) {
				setChannelState("search", undefined);
			}
		}
	};

	return (
		<aside class="search-results">
			<header>
				<Show when={!props.search.loading} fallback={<>Searching...</>}>
					{props.search.results?.total ?? 0} results
				</Show>
				<button
					onClick={() => {
						const id = searchId();
						if (id) {
							if (props.channel && channelCtx) {
								const [, setChannelState] = channelCtx;
								setChannelState("search", undefined);
							}
						}
					}}
				>
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
