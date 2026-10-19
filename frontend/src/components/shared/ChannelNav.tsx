import { useParams } from "@solidjs/router";
import type { api, Channel, ChannelType } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Index,
	Show,
} from "solid-js";
import { useCtx } from "@/app/context";
import {
	useApi,
	useChannels,
	useDms,
	useRoomMembers,
	useRooms,
	useUsers,
} from "@/api";
import icChevron from "@/assets/chevron.png";
import icHome from "@/assets/home.png";
import icInbox from "@/assets/inbox.png";
import icMemberAdd from "@/assets/member-add.png";
import icMembers from "@/assets/members.png";
import icSettings from "@/assets/settings.png";
import { Icon } from "@/atoms/Icon";
import { useCurrentUser } from "@/contexts/currentUser";
import { useDisplay, useMenu } from "@/contexts/mod";
import { useModals } from "@/contexts/modal";
import { useNavigate, useRouter } from "@/contexts/router";
import {
	UNCATEGORIZED_CHANNEL_ID,
	useChannelDnd,
} from "@/hooks/useChannelDnd";
import { useViewDnd } from "@/hooks/useViewDnd";
import {
	type NavItem,
	useChannelNavKeybinds,
} from "@/hooks/useChannelNavKeybinds";
import { usePermissions } from "@/hooks/usePermissions";
import { colors } from "@/lib/colors";
import { flags } from "@/lib/flags";
import {
	calculatePermissions,
	type PermissionContext,
} from "@/lib/permissions/calculator";
import {
	VIEW_UNCATEGORIZED_KEY,
	updateRoomNavView,
	viewCategoryKey,
} from "@/lib/room-nav";
import type { RoomNavItem, RoomNavView } from "@/types/room-nav";
import { getDate } from "@/utils/general";
import { useVoice } from "../features/voice/context";
import { Avatar, ChannelIcon } from "./User";

type ChannelWithThreads = Channel & { threads?: Channel[] };
type NavChannel = ChannelWithThreads & { nickname?: string | null };
type NavCategoryInfo = {
	id: string;
	name: string;
	channel: Channel | null;
};
type NavCategory = {
	category: NavCategoryInfo | null;
	channels: NavChannel[];
};

const CHANNEL_TYPES_HAS_UNREAD = new Set<ChannelType>([
	"Text",
	"ThreadPublic",
	"ThreadPrivate",
	"ThreadForum2",
	"Dm",
	"Gdm",
	"Announcement",
]);

// TODO: move last viewed channel into a context
function getLastViewedChannel(roomId: string): string | null {
	const key = `last_channel_${roomId}`;
	return localStorage.getItem(key);
}

function setLastViewedChannel(roomId: string, channelId: string): void {
	const key = `last_channel_${roomId}`;
	localStorage.setItem(key, channelId);
}

export const ChannelNav = (props: { room_id: string; view_id?: string }) => {
	const ctx = useCtx();
	const api2 = useApi();
	const dms2 = useDms();
	const rooms2 = useRooms();
	const channels2 = useChannels();
	const users2 = useUsers();
	const [voice] = useVoice();
	const { setMenu } = useMenu();
	const params = useParams();
	const nav = useNavigate();
	const dnd = useChannelDnd();
	const router = useRouter();

	const user = useCurrentUser();
	const currentUserId = () => user()?.id;

	const view = createMemo<RoomNavView | null>(() => {
		const viewId = props.view_id;
		if (!viewId) return null;

		const config = ctx.preferences().frontend.roomNav as
			| RoomNavItem[]
			| undefined;
		if (!config) return null;

		return (
			config.find(
				(item): item is RoomNavView =>
					item.type === "view" && item.id === viewId,
			) ?? null
		);
	});

	const updateView = (update: (view: RoomNavView) => RoomNavView) => {
		const viewId = props.view_id;
		if (!viewId) return;

		const prefs = ctx.preferences();
		const config = (prefs.frontend.roomNav ?? []) as RoomNavItem[];
		const roomNav = updateRoomNavView(config, viewId, update);
		if (!roomNav) return;

		ctx.setPreferences({
			...prefs,
			frontend: {
				...prefs.frontend,
				roomNav,
			},
		});
	};

	const viewDnd = useViewDnd({ view, updateView });

	createEffect(() => {
		if (!props.room_id || props.view_id) return;

		if (!flags.has("auto_redirect_last_channel")) return;

		const lastChannelId = getLastViewedChannel(props.room_id);
		if (!lastChannelId) return;

		const currentPath = params.channel_id;
		if (currentPath && currentPath === lastChannelId) return;

		const channel = channels2.cache.get(lastChannelId);
		if (!channel) return;

		nav(`/channel/${lastChannelId}`, { replace: true });
	});

	// make sure we have the room's channels (handle invite joins and room previewing)
	createEffect(() => {
		const rid = props.room_id;
		if (!rid) return;
		api2.channels.fetchRoomList(rid);
	});

	// views may reference channels from several rooms
	createEffect(() => {
		const currentView = view();
		if (!currentView) return;

		const roomIds = new Set<string>();
		const addChannels = (channels: api["ViewChannel"][] | undefined) => {
			for (const channel of channels ?? []) {
				if (channel.room_id) roomIds.add(channel.room_id);
			}
		};

		addChannels(currentView.uncategorized_channels);
		for (const category of currentView.categories ?? []) {
			if ("channels" in category) addChannels(category.channels);
			else if (category.room_id) roomIds.add(category.room_id);
		}

		for (const rid of roomIds) api2.channels.fetchRoomList(rid);
	});

	// track collapsed categories
	// TODO: persist in user preferences
	const [collapsedCategories, setCollapsedCategories] = createSignal<
		Set<string>
	>(new Set());

	// load dms
	createEffect(() => {
		const uid = currentUserId(); // retrigger useList on load
		if (uid) dms2.useList();
	});

	const room = rooms2.use(() => props.room_id || undefined);
	const roomMembers2 = useRoomMembers();

	const canViewChannel = (channel: Channel): boolean => {
		const rid = channel.room_id ?? props.room_id;
		if (!rid || !currentUserId()) {
			return true;
		}

		const uid = currentUserId();

		// Optimization: Owners can always view all channels.
		if (uid && rooms2.cache.get(rid)?.owner_id === uid) return true;

		const permissionContext: PermissionContext = {
			api: api2,
			room_id: rid,
			channel_id: channel.id,
		};

		if (!uid) return false;

		const { permissions } = calculatePermissions(permissionContext, uid);

		return permissions.has("ChannelView");
	};

	const viewChannel = (
		channel: api["ViewChannel"],
	): NavChannel | null => {
		const cached = channels2.cache.get(channel.id);
		if (!cached || cached.deleted_at || !canViewChannel(cached)) return null;

		return {
			...cached,
			name: channel.nickname ?? cached.name,
			nickname: channel.nickname,
			threads: [],
		};
	};

	const viewCategories = (currentView: RoomNavView): NavCategory[] => {
		const result: NavCategory[] = [];
		const uncategorized = (currentView.uncategorized_channels ?? [])
			.map(viewChannel)
			.filter((channel): channel is NavChannel => !!channel);

		result.push({ category: null, channels: uncategorized });

		for (const [index, category] of
			currentView.categories?.entries() ?? []) {
			if ("channels" in category) {
				result.push({
					category: {
						id: viewCategoryKey(currentView.id, index),
						name: category.name,
						channel: null,
					},
					channels: category.channels
						.map(viewChannel)
						.filter((channel): channel is NavChannel => !!channel),
				});
				continue;
			}

			const channel = viewChannel(category);
			if (!channel) continue;

			if (channel.type === "Category") {
				result.push({
					category: {
						id: channel.id,
						name: channel.name,
						channel,
					},
					channels: [],
				});
			} else {
				result.push({ category: null, channels: [channel] });
			}
		}

		return result;
	};

	const categories = createMemo<NavCategory[]>(() => {
		const currentView = view();
		if (currentView) return viewCategories(currentView);

		const allChannelsMap = new Map<string, ChannelWithThreads>();

		for (const c of channels2.listByRoom(props.room_id || null)) {
			if (!c.deleted_at) {
				allChannelsMap.set(c.id, c);
			}
		}

		if (!props.room_id) {
			for (const c of dms2.cache.values()) {
				if (!c.deleted_at) {
					allChannelsMap.set(c.id, c);
				}
			}
		}

		const allChannels = Array.from(allChannelsMap.values());

		const threads = allChannels.filter(
			(c) =>
				(c.type === "ThreadPublic" ||
					c.type === "ThreadPrivate" ||
					c.type === "ThreadForum2" ||
					(c.type === "Document" &&
						c.parent_id &&
						channels2.cache.get(c.parent_id)?.type === "Wiki")) &&
				(!c.archived_at || c.id === params.channel_id) &&
				canViewChannel(c),
		);
		const channels = allChannels.filter(
			(c) =>
				c.type !== "ThreadPublic" &&
				c.type !== "ThreadPrivate" &&
				c.type !== "ThreadForum2" &&
				!(
					c.type === "Document" &&
					c.parent_id &&
					channels2.cache.get(c.parent_id)?.type === "Wiki"
				) &&
				canViewChannel(c),
		);

		if (props.room_id) {
			// sort by id
			channels.sort((a, b) => {
				if (a.position == null && b.position == null) {
					return a.id < b.id ? 1 : -1;
				}
				if (a.position == null) return 1;
				if (b.position == null) return -1;
				return a.position - b.position;
			});
		} else {
			// sort by activity in dms list
			channels.sort((a, b) =>
				(a.last_version_id ?? "") < (b.last_version_id ?? "") ? 1 : -1,
			);
		}

		const channelMap = new Map<string, Channel & { threads: Channel[] }>();
		for (const c of channels) {
			channelMap.set(c.id, { ...c, threads: [] });
		}

		for (const thread of threads) {
			if (thread.parent_id) {
				const parent = channelMap.get(thread.parent_id);
				if (parent) {
					parent.threads.push(thread);
				}
			}
		}

		for (const c of channelMap.values()) {
			if (c.threads.length > 1) {
				c.threads.sort((a, b) => a.id.localeCompare(b.id));
			}
		}

		const categoryMap = new Map<
			string | null,
			Array<Channel & { threads: Channel[] }>
		>();
		categoryMap.set(null, []);

		for (const c of channelMap.values()) {
			if (c.type === "Category") {
				if (canViewChannel(c)) {
					const cat = categoryMap.get(c.id) ?? [];
					categoryMap.set(c.id, cat);
				}
			} else {
				const parentId = c.parent_id ?? null;
				const children = categoryMap.get(parentId) ?? [];
				children.push(c);
				categoryMap.set(parentId, children);
			}
		}
		return [...categoryMap.entries()]
			.map(([cid, cs]) => {
				const channel = cid ? (channels2.cache.get(cid) ?? null) : null;
				return {
					category: channel
						? { id: channel.id, name: channel.name, channel }
						: null,
					channels: cs,
				};
			})
			.sort((a, b) => {
				const aPosition = a.category?.channel?.position;
				const bPosition = b.category?.channel?.position;
				if (!a.category) return -1;
				if (!b.category) return 1;

				if (aPosition == null && bPosition == null) {
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (aPosition == null) return 1;
				if (bPosition == null) return -1;

				const p = aPosition - bPosition;
				if (p === 0) {
					return a.category.id < b.category.id ? 1 : -1;
				}

				return p;
			});
	});

	let buttonRef: HTMLButtonElement;
	const openRoomMenu = () => {
		const roomId = props.room_id;
		if (!roomId) return;
		setTimeout(() => {
			const rect = buttonRef!.getBoundingClientRect();
			setMenu({
				x: rect.left + 8,
				y: rect.bottom + 8,
				type: "room",
				room_id: roomId,
			});
		});
	};

	const navItems = createMemo(() => {
		const items: NavItem[] = [];

		if (props.room_id) {
			items.push({
				id: "home",
				type: "channel",
				categoryId: null,
				channelId: "home",
			});
		} else {
			items.push({
				id: "home",
				type: "channel",
				categoryId: null,
				channelId: "home",
			});
			if (flags.has("inbox"))
				items.push({
					id: "inbox",
					type: "channel",
					categoryId: null,
					channelId: "inbox",
				});
			if (flags.has("friends"))
				items.push({
					id: "friends",
					type: "channel",
					categoryId: null,
					channelId: "friends",
				});
		}

		for (const { category: cat, channels } of categories()) {
			const catId = cat ? cat.id : null;
			if (cat) {
				items.push({
					id: cat.id,
					type: "category",
					categoryId: catId,
					channelId: null,
				});
			}
			if (!cat || !collapsedCategories().has(cat.id)) {
				for (const channel of channels) {
					items.push({
						id: channel.id,
						type: "channel",
						categoryId: catId,
						channelId: channel.id,
						hasThreads: !!channel.threads?.length,
					});
					if (channel.threads?.length) {
						for (const thread of channel.threads) {
							items.push({
								id: thread.id,
								type: "thread",
								categoryId: catId,
								channelId: channel.id,
							});
						}
					}
				}
			}
		}
		return items;
	});

	const keybinds = useChannelNavKeybinds({
		items: navItems,
		categories: categories,
		selectedId: () => params.channel_id ?? "home",
		onToggleCategory: (id) => {
			setCollapsedCategories((prev) => {
				const newSet = new Set(prev);
				if (newSet.has(id)) {
					newSet.delete(id);
				} else {
					newSet.add(id);
				}
				return newSet;
			});
		},
		onSelectChannel: () => {},
	});

	const isFocused = (id: string) => {
		const focused = keybinds.focusedId();
		if (focused !== null) return focused === id;
		const selected = params.channel_id ?? "home";
		return selected === id;
	};

	const isVisible = (
		chan: ChannelWithThreads,
		categoryId: string | null = chan.parent_id ?? null,
	) => {
		const isFocused =
			chan.id === params.channel_id ||
			chan.threads?.some((t) => t.id === params.channel_id);
		const isCollapsed = collapsedCategories().has(categoryId ?? "");
		return !isCollapsed || isFocused;
	};

	// event handlers are not reactive in solid, so branch on view() at call time
	const handleHomeDragOver = (e: DragEvent) => {
		if (view()) viewDnd.overUncategorized(e);
		else dnd.handle(e);
	};

	const handleHomeDrop = (e: DragEvent) => {
		if (view()) viewDnd.handleDrop(e);
		else dnd.handle(e);
	};

	return (
		<nav
			id="channel-nav"
			ref={keybinds.container}
			tabindex="-1"
			onContextMenu={(e) => {
				e.preventDefault();
				const room_id = props.room_id;
				const view_id = props.view_id;
				if (!room_id && !view_id) return;
				queueMicrotask(() => {
					setMenu({
						x: e.clientX,
						y: e.clientY,
						type: "channel_nav",
						room_id,
						view_id,
					});
				});
			}}
		>
			<Show when={flags.has("nav_header")}>
				<button
					id="room-name-btn"
					type="button"
					classList={{
						"menu-room": !!props.room_id,
					}}
					data-room-id={props.room_id || undefined}
					ref={buttonRef!}
					onClick={openRoomMenu}
					onKeyDown={(e) => {
						if (e.key === "Enter" || e.key === " ") {
							e.preventDefault();
							openRoomMenu();
						}
					}}
				>
					<div>
						{/* <Icon src={icChevron} alt="" /> */}
						{view()?.name ??
							(props.room_id ? (room()?.name ?? "loading...") : "home")}
					</div>
				</button>
			</Show>

			<ul class="channel-list">
				<li
					class="channel-item"
					data-nav-id="home"
					tabIndex={isFocused("home") ? 0 : -1}
					data-dnd-placement={
						view()
							? viewDnd.placement(VIEW_UNCATEGORIZED_KEY)
							: dnd.placement(UNCATEGORIZED_CHANNEL_ID)
					}
					data-channel-id={
						!view() && props.room_id ? UNCATEGORIZED_CHANNEL_ID : undefined
					}
					onDragOver={handleHomeDragOver}
					onDrop={handleHomeDrop}
				>
					<a
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="channel-link"
						draggable={false}
						classList={{
							active:
								router.main ===
								(props.room_id ? `/room/${props.room_id}` : "/"),
						}}
						tabIndex={-1}
					>
						<Icon src={icHome} color={colors.fg500} /> home
					</a>
				</li>

				<Show when={!props.room_id && !view()}>
					<Show when={flags.has("inbox")}>
						<li
							class="channel-item"
							data-nav-id="inbox"
							tabIndex={isFocused("inbox") ? 0 : -1}
						>
							<a
								href="/inbox"
								class="channel-link"
								draggable={false}
								classList={{ active: router.main === "/inbox" }}
								tabIndex={-1}
							>
								<Icon src={icInbox} color={colors.fg500} /> inbox
							</a>
						</li>
					</Show>

					<Show when={flags.has("friends")}>
						<li
							class="channel-item"
							data-nav-id="friends"
							tabIndex={isFocused("friends") ? 0 : -1}
						>
							<a
								href="/friends"
								class="channel-link"
								draggable={false}
								classList={{ active: router.main === "/friends" }}
								tabIndex={-1}
							>
								<Icon src={icMembers} color={colors.fg500} /> friends
							</a>
						</li>
					</Show>

					<hr class="separator" />
				</Show>

				<For each={categories()}>
					{({ category, channels }) => (
						<>
							<Show when={category}>
								{(cat) => (
									<div
										class="category-header"
										classList={{
											collapsed: collapsedCategories().has(cat().id),
										}}
										data-dnd-placement={
											view()
												? cat().channel
													? undefined
													: viewDnd.placement(cat().id)
												: cat().channel
													? dnd.placement(cat().id)
													: undefined
										}
										data-channel-id={cat().channel ? cat().id : undefined}
										data-nav-id={cat().id}
										tabIndex={isFocused(cat().id) ? 0 : -1}
										draggable={!!cat().channel && !view()}
										onDragStart={view() ? undefined : dnd.handle}
										onDragOver={
											view()
												? cat().channel
													? undefined
													: (e: DragEvent) => viewDnd.overCategory(e, cat().id)
												: dnd.handle
										}
										onDrop={view() ? viewDnd.handleDrop : dnd.handle}
										onDragEnd={view() ? viewDnd.handleEnd : dnd.handle}
										onClick={() => {
											setCollapsedCategories((prev) => {
												const newSet = new Set(prev);
												if (newSet.has(cat().id)) {
													newSet.delete(cat().id);
												} else {
													newSet.add(cat().id);
												}
												return newSet;
											});
										}}
										onContextMenu={(e) => {
											const channel = cat().channel;
											if (!channel) return;
											e.preventDefault();
											e.stopPropagation();
											queueMicrotask(() => {
												setMenu({
													x: e.clientX,
													y: e.clientY,
													type: "channel",
													channel_id: channel.id,
												});
											});
										}}
									>
										<span
											class="category-toggle"
											classList={{
												collapsed: collapsedCategories().has(cat().id),
											}}
										>
											<Icon src={icChevron} />
										</span>
										<span class="category-name">{cat().name}</span>
									</div>
								)}
							</Show>
							<ul
								class="category-channels"
								data-channel-id={
									category?.channel?.id ??
									(view() || !props.room_id
										? undefined
										: UNCATEGORIZED_CHANNEL_ID)
								}
								data-dnd-placement={
									view()
										? category?.channel
											? undefined
											: viewDnd.placement(
													category ? category.id : VIEW_UNCATEGORIZED_KEY,
												)
										: category
											? undefined
											: dnd.placement(UNCATEGORIZED_CHANNEL_ID)
								}
								onDragOver={
									view()
										? category?.channel
											? undefined
											: category
												? (e: DragEvent) => viewDnd.overCategory(e, category.id)
												: viewDnd.overUncategorized
										: dnd.handle
								}
								onDrop={view() ? viewDnd.handleDrop : dnd.handle}
							>
								<For
									each={channels}
									fallback={
										<Show when={!props.room_id && !view()}>
											<Index each={Array(10).fill(0)}>
												{(_, index) => (
													<li
														class="channel-item skeleton"
														style={{
															"pointer-events": "none",
															opacity: 1 - index / 10,
														}}
													>
														<div class="channel-link">
															<div
																class="ghost avatar"
																style="border-radius: 4px"
															/>
															<div
																class="ghost"
																style="width: 128px; height: 1em;"
															/>
														</div>
													</li>
												)}
											</Index>
										</Show>
									}
								>
									{(channel) => (
										<Show when={isVisible(channel, category?.id ?? null)}>
											<li
												class="channel-item"
												classList={{
													dm: channel.type === "Dm" || channel.type === "Gdm",
												}}
												data-dnd-placement={
													view()
														? viewDnd.placement(channel.id)
														: dnd.placement(channel.id)
												}
												data-channel-id={channel.id}
												data-nav-id={channel.id}
												tabIndex={isFocused(channel.id) ? 0 : -1}
												draggable="true"
												onDragStart={view() ? viewDnd.handleStart : dnd.handle}
												onDragOver={
													view()
														? (e: DragEvent) =>
																viewDnd.overChannel(
																	e,
																	category?.id ?? null,
																	channel.id,
																)
															: dnd.handle
												}
												onDrop={view() ? viewDnd.handleDrop : dnd.handle}
												onDragEnd={view() ? viewDnd.handleEnd : dnd.handle}
												onContextMenu={(e) => {
													if (!view()) return;
													e.preventDefault();
													e.stopPropagation();
													queueMicrotask(() => {
														setMenu({
															x: e.clientX,
															y: e.clientY,
															type: "view_channel",
															view_id: props.view_id!,
															channel_id: channel.id,
														});
													});
												}}
											>
												<ItemChannel
													channel={channel}
													room_id={channel.room_id ?? props.room_id}
													view_id={props.view_id}
													nickname={channel.nickname}
													icon
												/>
												<Show when={channel.threads?.length}>
													<ul class="thread-list">
														<For each={channel.threads}>
															{(chan: Channel) => (
																<li
																	class="channel-item"
																	data-channel-id={chan.id}
																	data-nav-id={chan.id}
																	tabIndex={isFocused(chan.id) ? 0 : -1}
																	draggable={true}
																	onDragStart={dnd.handle}
																	onDragOver={dnd.handle}
																	onDrop={dnd.handle}
																	onDragEnd={dnd.handle}
																	classList={{
																		unread:
																			CHANNEL_TYPES_HAS_UNREAD.has(chan.type) &&
																			chan.last_read_id !==
																				chan.last_message_id,
																	}}
																>
																	<ItemChannel
																		channel={chan}
																		room_id={chan.room_id ?? props.room_id}
																		view_id={props.view_id}
																		icon={false}
																	/>
																</li>
															)}
														</For>
													</ul>
												</Show>
												<For
													each={[...api2.voiceStates.values()]
														.filter((i) => i.channel_id === channel.id)
														.sort(
															(a, b) =>
																getDate(a.joined_at).valueOf() -
																getDate(b.joined_at).valueOf(),
														)}
												>
													{(s) => {
														const user = () => users2.cache.get(s.user_id);
														const room_member = () => {
															const roomId = props.room_id;
															return roomId
																? roomMembers2.cache.get(
																		`${roomId}:${s.user_id}`,
																	)
																: null;
														};
														const name = () =>
															room_member()?.override_name ||
															user()?.name ||
															"unknown user";
														return (
															<div
																class="voice-participant menu-user"
																classList={{
																	speaking:
																		((voice.vc.speaking?.users.get(s.user_id)
																			?.flags ?? 0) &
																			1) ===
																		1,
																}}
																data-user-id={s.user_id}
																draggable={true}
																onDragStart={dnd.handle}
																onDragEnd={dnd.handle}
															>
																<Avatar user={user()} />
																{name()}
															</div>
														);
													}}
												</For>
											</li>
										</Show>
									)}
								</For>
							</ul>
						</>
					)}
				</For>
			</ul>
			<div style="margin: 8px" />
		</nav>
	);
};

export const ItemChannel = (props: {
	channel: Channel;
	room_id: string;
	view_id?: string;
	nickname?: string | null;
	icon: boolean;
}) => {
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const user = useCurrentUser();
	const currentUserId = () => user()?.id;
	const [hovered, setHovered] = createSignal(false);
	const [voice, voiceActions] = useVoice();
	const { isMobile } = useDisplay();

	const handleClick = (_e: MouseEvent) => {
		const roomId = props.channel.room_id ?? props.room_id;
		if (roomId) {
			setLastViewedChannel(roomId, props.channel.id);
		}

		if (isMobile()) {
			setTimeout(() => {
				const chat = document.querySelector(".chat");
				if (chat) {
					chat.scrollIntoView();
				} else {
					console.warn("could not find chat!");
				}
			});
		}

		if (props.channel.type === "Voice" || props.channel.type === "Broadcast") {
			if (
				!voice.joinedChannelId ||
				voice.joinedChannelId !== props.channel.id
			) {
				voiceActions.selectChannel(props.channel.id);
			}
		}
	};

	const otherUser = createMemo(() => {
		if (props.channel.type === "Dm") {
			const selfId = user()?.id;
			return props.channel.recipients?.find((i) => i.id !== selfId);
		}
		return undefined;
	});

	const name = () => {
		if (props.nickname) return props.nickname;

		if (props.channel.type === "Dm") {
			return otherUser()?.name ?? "dm";
		}

		return props.channel.name;
	};

	const channelConfig = () => props.channel.preferences;

	const isMuted = () => {
		const c = channelConfig();
		if (!c?.notifs.mute) return false;
		const expiresAt = c.notifs.mute.expires_at;
		if (!expiresAt) return true;
		return Date.parse(expiresAt) > Date.now();
	};

	const perms = usePermissions(
		currentUserId,
		() => props.room_id || undefined,
		() => props.channel.id,
	);

	const canInvite = () => perms.has("InviteCreate");

	const isDm = () =>
		props.channel.type === "Dm" || props.channel.type === "Gdm";

	const href = () => {
		if (props.channel.type === "Info") {
			return props.channel.url ?? null;
		}
		if (props.view_id) {
			return `/view/${props.view_id}/channel/${props.channel.id}`;
		}
		return `/channel/${props.channel.id}`;
	};

	const isUnread = (chan: Channel) => {
		if (!CHANNEL_TYPES_HAS_UNREAD.has(props.channel.type)) return false;
		if (!props.channel.last_read_id) return false;
		return props.channel.last_read_id !== props.channel.last_message_id;
	};

	const params = useParams();

	// TODO: dedupe copy pasted code
	return (
		<Show
			when={href()}
			fallback={
				<div
					class="menu-channel channel-link"
					classList={{ active: props.channel.id === params.channel_id }}
					data-unread={isUnread(props.channel)}
					data-muted={isMuted() ? "true" : undefined}
					data-channel-id={props.channel.id}
					onClick={handleClick}
					onMouseEnter={() => setHovered(true)}
					onMouseLeave={() => setHovered(false)}
					tabIndex={-1} // TODO: move tab index here?
				>
					<Show when={props.icon}>
						<ChannelIcon channel={props.channel} animate={hovered()} />
					</Show>

					<div class="channel-details">
						<span class="channel-name">{name()}</span>
						<Show
							when={
								otherUser()?.presence.activities.find(
									(a) => a.type === "Custom",
								)?.text
							}
						>
							{(t) => <span class="channel-status dim">{t()}</span>}
						</Show>
					</div>
					<Show when={props.channel.mention_count}>
						<div class="mentions">{props.channel.mention_count}</div>
					</Show>

					<Show when={!isDm()}>
						<div class="channel-actions">
							<Show when={canInvite()}>
								<button
									type="button"
									class="action-button button"
									title="Create Invite"
									onClick={(e) => {
										e.preventDefault();
										e.stopPropagation();
										modalCtl.open({
											type: "invite_create",
											room_id: props.room_id || undefined,
											channel_id: props.channel.id,
										});
									}}
								>
									<Icon src={icMemberAdd} color={colors.fg500} />
								</button>
							</Show>

							<button
								type="button"
								class="action-button button"
								title="Channel Settings"
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									nav(`/channel/${props.channel.id}/settings`);
								}}
							>
								<Icon src={icSettings} color={colors.fg500} />
							</button>
						</div>
					</Show>
				</div>
			}
		>
			{(href) => (
				<a
					href={href()}
					class="menu-channel channel-link"
					classList={{ active: props.channel.id === params.channel_id }}
					data-unread={isUnread(props.channel)}
					data-muted={isMuted() ? "true" : undefined}
					data-channel-id={props.channel.id}
					onClick={handleClick}
					onMouseEnter={() => setHovered(true)}
					onMouseLeave={() => setHovered(false)}
					tabIndex={-1} // TODO: move tab index here?
				>
					<Show when={props.icon}>
						<ChannelIcon channel={props.channel} animate={hovered()} />
					</Show>

					<div class="channel-details">
						<span class="channel-name">{name()}</span>
						<Show
							when={
								otherUser()?.presence.activities.find(
									(a) => a.type === "Custom",
								)?.text
							}
						>
							{(t) => <span class="channel-status dim">{t()}</span>}
						</Show>
					</div>
					<Show when={props.channel.mention_count}>
						<div class="mentions">{props.channel.mention_count}</div>
					</Show>

					<Show when={!isDm()}>
						<div class="channel-actions">
							<Show when={canInvite()}>
								<button
									type="button"
									class="action-button button"
									title="Create Invite"
									onClick={(e) => {
										e.preventDefault();
										e.stopPropagation();
										modalCtl.open({
											type: "invite_create",
											room_id: props.room_id || undefined,
											channel_id: props.channel.id,
										});
									}}
								>
									<Icon src={icMemberAdd} color={colors.fg500} />
								</button>
							</Show>

							<button
								type="button"
								class="action-button button"
								title="Channel Settings"
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									nav(`/channel/${props.channel.id}/settings`);
								}}
							>
								<Icon src={icSettings} color={colors.fg500} />
							</button>
						</div>
					</Show>
				</a>
			)}
		</Show>
	);
};
