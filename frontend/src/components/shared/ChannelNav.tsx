import { A, useNavigate, useParams } from "@solidjs/router";
import type { Channel, ChannelType } from "sdk";
import {
	createEffect,
	createMemo,
	createSelector,
	createSignal,
	For,
	Show,
} from "solid-js";

type ChannelWithThreads = Channel & { threads?: Channel[] };

import type { DOMElement } from "solid-js/jsx-runtime";
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
import icSettings from "@/assets/settings.png";
import icMembers from "@/assets/members.png";
import { Icon } from "@/atoms/Icon";
import { useCurrentUser } from "@/contexts/currentUser";
import { useDisplay, useMenu } from "@/contexts/mod";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions";
import { colors } from "@/lib/colors";
import { flags } from "@/lib/flags";
import {
	calculatePermissions,
	type PermissionContext,
} from "@/lib/permissions/calculator";
import { useVoice } from "../features/voice/context";
import { Avatar, ChannelIcon } from "./User";
import { useChannelDnd } from "@/hooks/useChannelDnd";

// TODO: review llm code here because im lazy and dont like implementing drag and drop

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

export const ChannelNav = (props: { room_id?: string }) => {
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

	const user = useCurrentUser();
	const currentUserId = () => user()?.id;

	createEffect(() => {
		if (!props.room_id) return;

		if (!flags.has("auto_redirect_last_channel")) return;

		const lastChannelId = getLastViewedChannel(props.room_id);
		if (!lastChannelId) return;

		const currentPath = params.channel_id;
		if (currentPath && currentPath === lastChannelId) return;

		const channel = channels2.cache.get(lastChannelId);
		if (!channel) return;

		nav(`/channel/${lastChannelId}`, { replace: true });
	});

	// track collapsed categories
	const [collapsedCategories, setCollapsedCategories] = createSignal<
		Set<string>
	>(new Set());

	// load dms
	createEffect(() => {
		currentUserId(); // retrigger useList on load
		dms2.useList();
	});

	const room = rooms2.use(() => props.room_id);
	const roomMembers2 = useRoomMembers();

	const canViewChannel = (channel: Channel): boolean => {
		if (!props.room_id || !currentUserId()) {
			return true;
		}

		const rid = props.room_id;
		const uid = currentUserId();

		// Optimization: Owners can always view all channels.
		if (uid && room()?.owner_id === uid) return true;

		const permissionContext: PermissionContext = {
			api: api2,
			room_id: rid,
			channel_id: channel.id,
		};

		if (!uid) return false;

		const { permissions } = calculatePermissions(permissionContext, uid);

		return permissions.has("ChannelView");
	};

	const categories = createMemo<
		Array<{ category: Channel | null; channels: Array<Channel> }>
	>(() => {
		const allChannelsMap = new Map<string, Channel>();

		for (const c of channels2.listByRoom(props.room_id ?? null)) {
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
			.map(([cid, cs]) => ({
				category: cid ? (channels2.cache.get(cid) ?? null) : null,
				channels: cs,
			}))
			.sort((a, b) => {
				if (!a.category) return -1;
				if (!b.category) return 1;

				if (a.category.position == null && b.category.position == null) {
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (a.category.position == null) return 1;
				if (b.category.position == null) return -1;

				const p = a.category.position - b.category.position;
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

	return (
		<nav id="channel-nav">
			<Show when={flags.has("nav_header")}>
				<button
					id="room-name-btn"
					type="button"
					classList={{
						"menu-room": !!props.room_id,
					}}
					data-room-id={props.room_id}
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
						{props.room_id ? (room()?.name ?? "loading...") : "home"}
					</div>
				</button>
			</Show>

			<ul class="channel-list">
				<li class="channel-item">
					<A
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="channel-link"
						draggable={false}
						end
					>
						<Icon src={icHome} color={colors.fg500} /> home
					</A>
				</li>

				<Show when={!props.room_id}>
					<Show when={flags.has("inbox")}>
						<li class="channel-item">
							<A href="/inbox" class="channel-link" draggable={false} end>
								<Icon src={icInbox} color={colors.fg500} /> inbox
							</A>
						</li>
					</Show>

					<Show when={flags.has("friends")}>
						<li class="channel-item">
							<A href="/friends" class="channel-link" draggable={false} end>
								<Icon src={icMembers} color={colors.fg500} /> friends
							</A>
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
										data-dnd-placement={dnd.placement(cat().id)}
										data-channel-id={cat().id}
										draggable="true"
										onDragStart={dnd.handle}
										onDragOver={dnd.handle}
										onDrop={dnd.handle}
										onDragEnd={dnd.handle}
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
											e.preventDefault();
											queueMicrotask(() => {
												setMenu({
													x: e.clientX,
													y: e.clientY,
													type: "channel",
													channel_id: cat().id,
												});
											});
										}}
									>
										<span class="category-toggle">
											{collapsedCategories().has(cat().id) ? "▶" : "▼"}
										</span>
										<span class="category-name">{cat().name}</span>
									</div>
								)}
							</Show>
							<Show when={!category || !collapsedCategories().has(category.id)}>
								<ul class="category-channels">
									<For
										each={channels}
										fallback={
											<div
												class="empty-text"
												style="margin-left: 16px"
												data-channel-id={category?.id}
												onDragOver={dnd.handle}
												onDrop={dnd.handle}
											>
												(no channels)
											</div>
										}
									>
										{(channel) => (
											<li
												class="channel-item"
												classList={{
													dm: channel.type === "Dm" || channel.type === "Gdm",
												}}
												data-dnd-placement={dnd.placement(channel.id)}
												data-channel-id={channel.id}
												draggable="true"
												onDragStart={dnd.handle}
												onDragOver={dnd.handle}
												onDrop={dnd.handle}
												onDragEnd={dnd.handle}
											>
												<ItemChannel
													channel={channel}
													room_id={props.room_id}
												/>
												<Show
													when={(channel as ChannelWithThreads).threads?.length}
												>
													<ul class="thread-list">
														<For each={(channel as ChannelWithThreads).threads}>
															{(chan: Channel) => (
																<li
																	class="channel-item"
																	data-channel-id={chan.id}
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
																		room_id={props.room_id}
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
																Date.parse(a.joined_at) -
																Date.parse(b.joined_at),
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
										)}
									</For>
								</ul>
							</Show>
						</>
					)}
				</For>
			</ul>
			<div style="margin: 8px" />
		</nav>
	);
};

export const ItemChannel = (props: { channel: Channel; room_id?: string }) => {
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const user = useCurrentUser();
	const currentUserId = () => user()?.id;
	const [hovered, setHovered] = createSignal(false);
	const { isMobile } = useDisplay();

	const handleClick = (_e: MouseEvent) => {
		if (props.room_id) {
			setLastViewedChannel(props.room_id, props.channel.id);
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
	};

	const otherUser = createMemo(() => {
		if (props.channel.type === "Dm") {
			const selfId = user()?.id;
			return props.channel.recipients?.find((i) => i.id !== selfId);
		}
		return undefined;
	});

	const name = () => {
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
		() => props.room_id,
		() => props.channel.id,
	);

	const canInvite = () => perms.has("InviteCreate");

	const isDm = () =>
		props.channel.type === "Dm" || props.channel.type === "Gdm";

	const params = useParams();

	return (
		<A
			href={`/channel/${props.channel.id}`}
			class="menu-channel channel-link"
			classList={{ active: props.channel.id === params.channel_id }}
			data-unread={
				CHANNEL_TYPES_HAS_UNREAD.has(props.channel.type) &&
				props.channel.last_read_id !== props.channel.last_message_id
			}
			data-muted={isMuted() ? "true" : undefined}
			data-channel-id={props.channel.id}
			onClick={handleClick}
			onMouseEnter={() => setHovered(true)}
			onMouseLeave={() => setHovered(false)}
		>
			<ChannelIcon channel={props.channel} animate={hovered()} />

			<div class="channel-details">
				<span class="channel-name">{name()}</span>
				<Show
					when={
						otherUser()?.presence.activities.find((a) => a.type === "Custom")
							?.text
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
									room_id: props.room_id,
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
		</A>
	);
};
