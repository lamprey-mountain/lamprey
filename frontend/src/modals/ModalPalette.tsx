import type { Channel, Room } from "sdk";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { useChannels, useDms, useRooms } from "@/api";
import { useCtx } from "@/app/context";
import icHome from "@/assets/home.png";
import icInbox from "@/assets/inbox.png";
import icMembers from "@/assets/members.png";
import icSearch from "@/assets/search.png";
import icSettings from "@/assets/settings.png";
import { Search } from "@/atoms/Search";
import { ChannelIcon } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { useNavigate } from "@/contexts/router";
import { parsePaletteLink } from "@/lib/palette-link";
import { getThumbFromId } from "@/media/util";
import { channelDisplayName } from "@/utils/channel";
import { Modal } from "./mod";

type PaletteItem = {
	type: "room" | "thread" | "link" | "section";
	id: string;
	name: string;
	/** shown when the item has no channel or room attached */
	icon?: string;
	/** where the channel lives */
	meta?: { room?: string; category?: string };
	action: () => void;
	channel?: Channel;
	room?: Room;
};

const isDm = (channel: Channel) =>
	channel.type === "Dm" || channel.type === "Gdm";

const isThread = (channel: Channel) => channel.type.startsWith("Thread");

const isUnread = (channel: Channel) =>
	!!channel.last_read_id && channel.last_read_id !== channel.last_message_id;

const section = (name: string, items: PaletteItem[]): PaletteItem[] =>
	items.length === 0
		? []
		: [
				{
					type: "section" as const,
					id: `section-${name}`,
					name,
					action: () => {},
				},
				...items,
			];

/** higher is better, 0 means no match */
const matchScore = (name: string, query: string) => {
	const n = name.toLowerCase();
	if (!query) return 1;
	if (n === query) return 4;
	if (n.startsWith(query)) return 3;
	if (n.includes(` ${query}`)) return 2;
	if (n.includes(query)) return 1;
	return 0;
};

const rank = (items: PaletteItem[], query: string) =>
	items
		.map((item) => ({ item, score: matchScore(item.name, query) }))
		.filter((entry) => entry.score > 0)
		.sort((a, b) => b.score - a.score)
		.map((entry) => entry.item);

export const ModalPalette = () => {
	const channels2 = useChannels();
	const rooms2 = useRooms();
	const dms2 = useDms();
	const ctx = useCtx();
	const navigate = useNavigate();
	const currentUser = useCurrentUser();
	const [, modalCtl] = useModals();

	const [query, setQuery] = createSignal("");
	const [selectedIndex, setSelectedIndex] = createSignal(0);
	let itemsEl: HTMLDivElement | undefined;

	// try to load all threads
	const _rooms = rooms2.useList();
	dms2.useList();

	const channelName = (channel: Channel) => {
		if (channel.type === "Dm") {
			const selfId = currentUser()?.id;
			return channel.recipients?.find((i) => i.id !== selfId)?.name ?? "dm";
		}
		return channelDisplayName(channel);
	};

	const findChannel = (id: string) =>
		channels2.cache.get(id) ?? dms2.cache.get(id);

	const roomChannels = createMemo(() =>
		[...channels2.cache.values()].filter(
			(channel) =>
				channel.type !== "Category" && !isThread(channel) && !isDm(channel),
		),
	);

	const dmChannels = createMemo(() => [...dms2.cache.values()]);

	const allChannels = createMemo(() => [...roomChannels(), ...dmChannels()]);

	const channelRoom = (channel: Channel): Room | undefined => {
		const room = channel.room_id
			? rooms2.cache.get(channel.room_id)
			: undefined;
		if (room) return room;
		// threads inherit the room of their parent channel
		const parent = channel.parent_id
			? channels2.cache.get(channel.parent_id)
			: undefined;
		return parent?.room_id ? rooms2.cache.get(parent.room_id) : undefined;
	};

	const channelCategory = (channel: Channel): Channel | undefined => {
		const parent = channel.parent_id
			? channels2.cache.get(channel.parent_id)
			: undefined;
		if (!parent) return undefined;
		if (parent.type === "Category") return parent;
		// threads hang off a channel, so the category is one level up
		const grandparent = parent.parent_id
			? channels2.cache.get(parent.parent_id)
			: undefined;
		return grandparent?.type === "Category" ? grandparent : undefined;
	};

	const channelMeta = (channel: Channel): PaletteItem["meta"] => {
		const room = channelRoom(channel)?.name;
		const category = channelCategory(channel);
		const categoryName = category ? channelDisplayName(category) : undefined;
		if (!room && !categoryName) return undefined;
		return { room, category: categoryName };
	};

	const channelItem = (channel: Channel): PaletteItem => ({
		type: "thread",
		id: channel.id,
		name: channelName(channel),
		meta: channelMeta(channel),
		action: () => navigate(`/channel/${channel.id}`),
		channel,
	});

	const roomItem = (room: Room): PaletteItem => ({
		type: "room",
		id: room.id,
		name: room.name,
		action: () => navigate(`/room/${room.id}`),
		room,
	});

	const staticLinks = (): PaletteItem[] => [
		{
			type: "link",
			id: "home",
			name: "home",
			icon: icHome,
			action: () => navigate("/"),
		},
		{
			type: "link",
			id: "inbox",
			name: "inbox",
			icon: icInbox,
			action: () => navigate("/inbox"),
		},
		{
			type: "link",
			id: "friends",
			name: "friends",
			icon: icMembers,
			action: () => navigate("/friends"),
		},
		{
			type: "link",
			id: "settings",
			name: "settings",
			icon: icSettings,
			action: () => navigate("/settings"),
		},
	];

	const recentChannels = createMemo(() =>
		ctx
			.recentChannels()
			.slice(1)
			.map((id) => findChannel(id))
			.filter(
				(channel): channel is Channel =>
					!!channel && channel.type !== "Category",
			)
			.map(channelItem),
	);

	const channelsWithDrafts = createMemo(() => {
		const draftChannels: Array<{
			channel: Channel;
			draftTimestamp: number;
		}> = [];

		for (const channel of allChannels()) {
			// Check localStorage drafts (Forum2)
			const draftKey = `editor_draft_${channel.id}`;
			const draft = localStorage.getItem(draftKey);
			if (draft?.trim()) {
				try {
					const parsed = JSON.parse(draft);
					const timestamp = parsed.timestamp ?? 0;
					draftChannels.push({
						channel: channel,
						draftTimestamp: timestamp,
					});
				} catch {
					draftChannels.push({
						channel: channel,
						draftTimestamp: 0,
					});
				}
			}

			// Check in-memory drafts from channel contexts (Input.tsx)
			const chCtx = ctx.channel_contexts.get(channel.id);
			if (chCtx) {
				const [chState] = chCtx;
				const editorState = chState.editor_state;
				if (editorState?.doc.textContent.trim()) {
					draftChannels.push({
						channel,
						draftTimestamp: 0,
					});
				}
			}
		}

		return draftChannels
			.sort((a, b) => b.draftTimestamp - a.draftTimestamp)
			.map(({ channel }) => channelItem(channel));
	});

	const channelsWithMentions = createMemo(() =>
		allChannels()
			.filter((channel) => channel.mention_count && channel.mention_count > 0)
			.sort((a, b) => {
				if (b.mention_count !== a.mention_count) {
					return b.mention_count! - a.mention_count!;
				}
				return b.version_id.localeCompare(a.version_id);
			})
			.map(channelItem),
	);

	const unreadChannels = createMemo(() =>
		allChannels()
			.filter(isUnread)
			.sort((a, b) => b.version_id.localeCompare(a.version_id))
			.map(channelItem),
	);

	const defaultItems = createMemo(() => {
		const seen = new Set<string>();
		const unique = (items: PaletteItem[]) =>
			items.filter((item) => {
				if (seen.has(item.id)) return false;
				seen.add(item.id);
				return true;
			});

		return [
			...section("recent channels", unique(recentChannels())),
			...section("has draft", unique(channelsWithDrafts())),
			...section("recent mentions", unique(channelsWithMentions())),
			...section("unread", unique(unreadChannels())),
		];
	});

	const everything = createMemo(() => [
		...staticLinks(),
		...[...rooms2.cache.values()].map(roomItem),
		...allChannels().map(channelItem),
	]);

	const pastedLink = createMemo((): PaletteItem | null => {
		const link = parsePaletteLink(query());
		if (!link) return null;

		const channel = link.channelId ? findChannel(link.channelId) : undefined;
		const room = link.roomId ? rooms2.cache.get(link.roomId) : undefined;
		const name = channel
			? channelName(channel)
			: room
				? room.name
				: (link.name ?? link.path);

		return {
			type: channel ? "thread" : room ? "room" : "link",
			id: `link:${link.path}`,
			name: link.messageId ? `message in ${name}` : name,
			meta: channel ? channelMeta(channel) : undefined,
			channel,
			room,
			action: () => navigate(link.path),
		};
	});

	const showHelp = createMemo(() => query().trim() === "?");

	const filteredItems = createMemo((): PaletteItem[] => {
		if (showHelp()) return [];

		const raw = query().trim();
		const link = pastedLink();
		const prefix = raw.startsWith("@") ? "@" : raw.startsWith("#") ? "#" : "";
		const text = (prefix ? raw.slice(1) : raw).toLowerCase().trim();

		const matches = () => {
			if (prefix === "@") return rank(dmChannels().map(channelItem), text);
			if (prefix === "#") {
				return rank(roomChannels().map(channelItem), text);
			}
			if (text) return rank(everything(), text);

			const items = defaultItems();
			return items.length > 0 ? items : everything();
		};

		const items = matches();
		const navigable = items.filter((item) => item.type !== "section");
		// nothing good left to show, offer to search instead
		const searchItem: PaletteItem[] =
			text &&
			!prefix &&
			(navigable.length < 10 ||
				items.every((item) => matchScore(item.name, text) < 2))
				? [
						{
							type: "link",
							id: "search",
							name: `search for "${raw}"`,
							icon: icSearch,
							action: () => navigate(`/search?q=${encodeURIComponent(raw)}`),
						},
					]
				: [];

		return [...(link ? [link] : []), ...items, ...searchItem];
	});

	const visibleItems = createMemo(() => {
		const items = filteredItems().slice(0, 10);
		// don't leave a section header dangling at the end
		return items.at(-1)?.type === "section" ? items.slice(0, -1) : items;
	});

	createEffect(() => {
		setSelectedIndex(0);
	});

	createEffect(() => {
		selectedIndex();
		itemsEl?.querySelector<HTMLElement>(".item.selected")?.scrollIntoView({
			block: "nearest",
		});
	});

	const handleKeyDown = (e: KeyboardEvent) => {
		const navigableItems = visibleItems().filter(
			(item) => item.type !== "section",
		);
		const len = navigableItems.length;
		if (len === 0) return;

		if (e.key === "ArrowDown" || (e.key === "Tab" && !e.shiftKey)) {
			e.preventDefault();
			setSelectedIndex((i) => (i + 1) % len);
		} else if (e.key === "ArrowUp" || (e.key === "Tab" && e.shiftKey)) {
			e.preventDefault();
			setSelectedIndex((i) => (i - 1 + len) % len);
		} else if (e.key === "Enter") {
			e.preventDefault();
			const item = navigableItems[selectedIndex()];
			if (item) {
				item.action();
				modalCtl.close();
			}
		}
	};

	const close = () => {
		modalCtl.close();
	};

	const getNavigableIndex = (index: number) => {
		const items = visibleItems();
		let navigableIndex = 0;
		for (let i = 0; i < index; i++) {
			if (items[i]?.type !== "section") {
				navigableIndex++;
			}
		}
		return navigableIndex;
	};

	return (
		<Modal>
			<div onKeyDown={handleKeyDown} class="palette">
				<Search
					ref={(a) => queueMicrotask(() => a.focus())}
					value={query}
					onInput={setQuery}
					placeholder="lets go somewhere"
				/>
				<div class="items" ref={itemsEl}>
					<Show
						when={!showHelp()}
						fallback={
							<div class="help">
								<div>type to search channels and rooms</div>
								<div>paste a link to jump somewhere</div>
								<div>
									<kbd>@</kbd> dms and groups
								</div>
								<div>
									<kbd>#</kbd> room channels
								</div>
								<div>
									<kbd>?</kbd> this help
								</div>
							</div>
						}
					>
						<Show
							when={visibleItems().length > 0}
							fallback={<div class="empty dim">no results</div>}
						>
							<For each={visibleItems()}>
								{(item, i) => (
									<Show
										when={item.type !== "section"}
										fallback={
											<div class="item section-header">{item.name}</div>
										}
									>
										<div
											class="item"
											classList={{
												selected: getNavigableIndex(i()) === selectedIndex(),
											}}
											onClick={() => {
												item.action();
												close();
											}}
											onMouseEnter={() =>
												setSelectedIndex(getNavigableIndex(i()))
											}
										>
											<Show when={item.type === "thread" && item.channel} keyed>
												<div class="item-icon">
													<ChannelIcon channel={item.channel!} />
												</div>
											</Show>
											<Show when={item.type === "room" && item.room} keyed>
												<div class="item-icon">
													<Show
														when={item.room?.icon}
														fallback={
															<div class="avatar fake">
																{item.room?.name.substring(0, 2)}
															</div>
														}
													>
														<img
															src={getThumbFromId(item.room?.icon!, 64)}
															class="avatar"
														/>
													</Show>
												</div>
											</Show>
											<Show when={item.type === "link" && item.icon} keyed>
												<div class="item-icon">
													<img src={item.icon} class="icon" />
												</div>
											</Show>
											<span>{item.name}</span>
											<Show when={item.meta}>
												{(meta) => (
													<span class="meta">
														<Show when={meta().room}>
															<span class="room">{meta().room}</span>
														</Show>
														<Show when={meta().room && meta().category}>
															{/* TODO: use chevron icon */}
															<span class="chevron">›</span>
														</Show>
														<Show when={meta().category}>
															<span class="category">{meta().category}</span>
														</Show>
													</span>
												)}
											</Show>
										</div>
									</Show>
								)}
							</For>
						</Show>
					</Show>
				</div>
				<div class="hint dim">
					<span>
						<kbd>?</kbd> help
					</span>
					<span>
						<kbd>@</kbd> dms
					</span>
					<span>
						<kbd>#</kbd> channels
					</span>
				</div>
			</div>
		</Modal>
	);
};
