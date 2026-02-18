import { useNavigate } from "@solidjs/router";
import type { Channel, Room } from "sdk";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { useApi } from "../api";
import { useCtx } from "../context";
import { getThumbFromId } from "../media/util";
import { ChannelIcon } from "../User";
import { Modal } from "./mod";
import { useModals } from "../contexts/modal";
import icHome from "../assets/home.png";
import icInbox from "../assets/inbox.png";
import icSettings from "../assets/settings.png";
import icMembers from "../assets/members.png";

export const ModalPalette = () => {
	const api = useApi();
	const ctx = useCtx();
	const navigate = useNavigate();
	const [, modalCtl] = useModals();

	const [query, setQuery] = createSignal("");
	const [selectedIndex, setSelectedIndex] = createSignal(0);

	// try to load all threads
	const rooms = api.rooms.list();
	api.dms.list();
	createEffect(() => {
		for (const room of rooms()?.items ?? []) {
			api.channels.list(() => room.id);
		}
	});

	type PaletteItem = {
		type: "room" | "thread" | "link" | "section";
		id: string;
		name: string;
		action: () => void;
		channel?: Channel;
		room?: Room;
	};

	const allItems = createMemo((): PaletteItem[] => {
		const rooms = [...api.rooms.cache.values()].map((room) => ({
			type: "room" as const,
			id: room.id,
			name: room.name,
			action: () => navigate(`/room/${room.id}`),
			room: room,
		}));
		const threads = [...api.channels.cache.values()].map((thread) => ({
			type: "thread" as const,
			id: thread.id,
			name: thread.name,
			action: () => navigate(`/channel/${thread.id}`),
			channel: thread,
		}));

		const staticItems: PaletteItem[] = [
			{
				type: "link" as const,
				id: "home",
				name: "home",
				action: () => navigate("/"),
			},
			{
				type: "link" as const,
				id: "inbox",
				name: "inbox",
				action: () => navigate("/inbox"),
			},
			{
				type: "link" as const,
				id: "friends",
				name: "friends",
				action: () => navigate("/friends"),
			},
			{
				type: "link" as const,
				id: "settings",
				name: "settings",
				action: () => navigate("/settings"),
			},
		];

		return [...staticItems, ...rooms, ...threads];
	});

	const recentChannels = createMemo(() => {
		return ctx.recentChannels().slice(1).map((i: any) =>
			api.channels.cache.get(i)!
		)
			.map((
				thread: any,
			) => ({
				type: "thread" as const,
				id: thread.id,
				name: thread.name,
				action: () => navigate(`/thread/${thread.id}`),
				channel: thread,
			}));
	});

	const channelsWithMentions = createMemo(() => {
		return [...api.channels.cache.values()]
			.filter((channel) => channel.mention_count && channel.mention_count > 0)
			.sort((a, b) => {
				if (b.mention_count !== a.mention_count) {
					return b.mention_count! - a.mention_count!;
				}
				return b.version_id.localeCompare(a.version_id);
			})
			.map((channel) => ({
				type: "thread" as const,
				id: channel.id,
				name: channel.name,
				action: () => navigate(`/channel/${channel.id}`),
				channel: channel,
			}));
	});

	const channelsWithDrafts = createMemo(() => {
		const draftChannels: Array<{
			channel: any;
			draftTimestamp: number;
		}> = [];

		for (const channel of api.channels.cache.values()) {
			// Check localStorage drafts (Forum2)
			const draftKey = `editor_draft_${channel.id}`;
			const draft = localStorage.getItem(draftKey);
			if (draft && draft.trim()) {
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
				if (editorState && editorState.doc.textContent.trim()) {
					draftChannels.push({
						channel,
						draftTimestamp: 0,
					});
				}
			}
		}

		return draftChannels
			.sort((a, b) => b.draftTimestamp - a.draftTimestamp)
			.map(({ channel }) => ({
				type: "thread" as const,
				id: channel.id,
				name: channel.name,
				action: () => navigate(`/channel/${channel.id}`),
				channel: channel,
			}));
	});

	const filteredItems = createMemo(() => {
		const q = query().toLowerCase();
		if (!q) {
			const mentions = channelsWithMentions();
			const drafts = channelsWithDrafts();
			const recent = recentChannels();

			const seen = new Set<string>();
			const items: PaletteItem[] = [];

			const mentionItems: PaletteItem[] = [];
			for (const item of mentions) {
				if (!seen.has(item.id)) {
					seen.add(item.id);
					mentionItems.push(item);
				}
			}
			if (mentionItems.length > 0) {
				items.push({
					type: "section" as const,
					id: "section-mentions",
					name: "recent mentions",
					action: () => {},
				});
				items.push(...mentionItems);
			}

			const draftItems: PaletteItem[] = [];
			for (const item of drafts) {
				if (!seen.has(item.id)) {
					seen.add(item.id);
					draftItems.push(item);
				}
			}
			if (draftItems.length > 0) {
				items.push({
					type: "section" as const,
					id: "section-drafts",
					name: "has draft",
					action: () => {},
				});
				items.push(...draftItems);
			}

			const recentItems: PaletteItem[] = [];
			for (const item of recent) {
				if (!seen.has(item.id)) {
					seen.add(item.id);
					recentItems.push(item);
				}
			}
			if (recentItems.length > 0) {
				items.push({
					type: "section" as const,
					id: "section-recent",
					name: "recent channels",
					action: () => {},
				});
				items.push(...recentItems);
			}

			return items;
		}

		return allItems().filter((item) =>
			item.name && item.name.toLowerCase().includes(q)
		);
	});

	createEffect(() => {
		setSelectedIndex(0);
	});

	const handleKeyDown = (e: KeyboardEvent) => {
		const items = filteredItems();
		const navigableItems = items.filter((item) => item.type !== "section");
		const len = navigableItems.length;
		if (len === 0) return;

		if (e.key === "ArrowDown") {
			e.preventDefault();
			setSelectedIndex((i) => (i + 1) % len);
		} else if (e.key === "ArrowUp") {
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
		const items = filteredItems();
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
				<h3 class="dim">palette</h3>
				<input
					type="text"
					autofocus
					ref={(a) => queueMicrotask(() => a.focus())}
					value={query()}
					onInput={(e) => setQuery(e.currentTarget.value)}
					placeholder="type to search..."
				/>
				<div class="items">
					<For each={filteredItems().slice(0, 10)}>
						{(item, i) => (
							<Show
								when={item.type !== "section"}
								fallback={
									<div class="item section-header dim">{item.name}</div>
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
									onMouseEnter={() => setSelectedIndex(getNavigableIndex(i()))}
								>
									<Show when={item.type === "thread" && item.channel} keyed>
										<div class="item-icon">
											<ChannelIcon channel={item.channel!} />
										</div>
									</Show>
									<Show when={item.type === "room" && item.room} keyed>
										<div class="item-icon">
											<Show
												when={item.room!.icon}
												fallback={
													<div class="avatar fake">
														{item.room!.name.substring(0, 2)}
													</div>
												}
											>
												<img
													src={getThumbFromId(item.room!.icon!, 64)}
													class="avatar"
												/>
											</Show>
										</div>
									</Show>
									<Show when={item.type === "link"} keyed>
										<div class="item-icon">
											<img
												src={item.id === "home"
													? icHome
													: item.id === "inbox"
													? icInbox
													: item.id === "settings"
													? icSettings
													: item.id === "friends"
													? icMembers
													: ""}
												class="icon"
											/>
										</div>
									</Show>
									<span>{item.name}</span>
								</div>
							</Show>
						)}
					</For>
				</div>
			</div>
		</Modal>
	);
};
