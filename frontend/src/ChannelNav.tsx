import { A, useNavigate, useParams } from "@solidjs/router";
import type { Channel } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import {
	useApi2,
	useChannels2,
	useDms2,
	useRoomMembers2,
	useRooms2,
	useUsers2,
} from "@/api";
import { useConfig } from "./config";
import { flags } from "./flags";
import { useCtx } from "./context";
import { useMenu } from "./contexts/mod.tsx";
import { useModals } from "./contexts/modal";
import { Avatar, AvatarWithStatus, ChannelIcon, ChannelIconGdm } from "./User";
import { useVoice } from "./components/features/voice/voice-provider";
import { useCurrentUser } from "./contexts/currentUser.tsx";
import {
	calculatePermissions,
	type PermissionContext,
} from "./permission-calculator";
import icHome from "./assets/home.png";
import icInbox from "./assets/inbox.png";
import icSettings from "./assets/settings.png";
import icMemberAdd from "./assets/member-add.png";

// TODO: review llm code here because im lazy and dont like implementing drag and drop

function getLastViewedChannel(roomId: string): string | null {
	const key = `last_channel_${roomId}`;
	return localStorage.getItem(key);
}

function setLastViewedChannel(roomId: string, channelId: string): void {
	const key = `last_channel_${roomId}`;
	localStorage.setItem(key, channelId);
}

export const ChannelNav = (props: { room_id?: string }) => {
	const config = useConfig();
	const api2 = useApi2();
	const dms2 = useDms2();
	const rooms2 = useRooms2();
	const channels2 = useChannels2();
	const users2 = useUsers2();
	const [voice] = useVoice();
	const ctx = useCtx();
	const { setMenu } = useMenu();
	const params = useParams();
	const nav = useNavigate();

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

	// track drag ids
	const [dragging, setDragging] = createSignal<
		{ type: "channel" | "voice"; id: string; channelId?: string } | null
	>(null);
	const [target, setTarget] = createSignal<
		{ id: string; mode: "before" | "after" | "inside" } | null
	>(null);

	// track collapsed categories
	const [collapsedCategories, setCollapsedCategories] = createSignal<
		Set<string>
	>(new Set());

	// Load DMs when not in a room
	const dms = !props.room_id ? dms2.useList() : null;

	const room = props.room_id ? rooms2.use(() => props.room_id!) : () => null;
	const roomMembers2 = useRoomMembers2();

	const canViewChannel = (channel: Channel): boolean => {
		if (!props.room_id || !currentUserId()) {
			return true;
		}

		const permissionContext: PermissionContext = {
			api: api2,
			room_id: props.room_id,
			channel_id: channel.id,
		};

		const { permissions } = calculatePermissions(
			permissionContext,
			currentUserId()!,
		);

		return permissions.has("ChannelView");
	};

	const categories = createMemo<
		Array<{ category: Channel | null; channels: Array<Channel> }>
	>(() => {
		const allChannels = [...channels2.cache.values()].filter(
			(c) =>
				(props.room_id ? c.room_id === props.room_id : c.room_id === null) &&
				!c.deleted_at,
		);

		const threads = allChannels.filter(
			(c) =>
				(c.type === "ThreadPublic" ||
					c.type === "ThreadPrivate" ||
					c.type === "ThreadForum2" ||
					(c.type === "Document" &&
						c.parent_id &&
						channels2.cache.get(c.parent_id)?.type === "Wiki")) &&
				!c.archived_at &&
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
				if (a.position === null && b.position === null) {
					return a.id < b.id ? 1 : -1;
				}
				if (a.position === null) return 1;
				if (b.position === null) return -1;
				return a.position! - b.position!;
			});
		} else {
			// sort by activity in dms list
			channels.sort((a, b) =>
				(a.last_version_id ?? "") < (b.last_version_id ?? "") ? 1 : -1
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
				const children = categoryMap.get(c.parent_id!) ?? [];
				children.push(c);
				categoryMap.set(c.parent_id!, children);
			}
		}
		return [...categoryMap.entries()]
			.map(([cid, cs]) => ({
				category: cid ? channels2.cache.get(cid)! : null,
				channels: cs,
			}))
			.filter(({ channels }) => {
				return channels.length > 0;
			})
			.sort((a, b) => {
				if (!a.category) return -1;
				if (!b.category) return 1;

				if (a.category.position === null && b.category.position === null) {
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (a.category.position === null) return 1;
				if (b.category.position === null) return -1;

				const p = a.category.position! - b.category.position!;
				if (p === 0) {
					return a.category.id < b.category.id ? 1 : -1;
				}

				return p;
			}) as any;
	});

	// helper to get channel id from the element's data attribute
	const getChannelId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.channelId;

	const handleDragStart = (e: DragEvent) => {
		e.stopPropagation();
		e.dataTransfer!.effectAllowed = "move";
		const id = getChannelId(e);
		e.dataTransfer!.setData("text/plain", id || "");

		if (id) {
			setDragging({ type: "channel", id });
		}
	};

	const handleVoiceDragStart = (
		e: DragEvent,
		userId: string,
		channelId: string,
	) => {
		e.stopPropagation();
		e.stopImmediatePropagation();
		e.dataTransfer!.effectAllowed = "move";
		e.dataTransfer!.setData("text/plain", userId);
		setDragging({ type: "voice", id: userId, channelId });
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const id = getChannelId(e);
		const dragInfo = dragging();
		if (!id || dragInfo?.id === id) {
			return;
		}

		// handle voice participant drag
		if (dragInfo?.type === "voice") {
			const targetChannel = channels2.cache.get(id);
			if (targetChannel?.type === "Voice") {
				if (target()?.id !== id) {
					setTarget({ id, mode: "inside" });
				}
			}
			return;
		}

		const draggingId = dragInfo?.id;
		if (draggingId) {
			const draggingChannel = channels2.cache.get(draggingId);
			const targetChannel = channels2.cache.get(id);

			if (!draggingChannel || !targetChannel) return;

			// if dragging a thread
			if (
				draggingChannel.type === "ThreadPublic" ||
				draggingChannel.type === "ThreadPrivate" ||
				draggingChannel.type === "ThreadForum2"
			) {
				let validParents: string[] = [];
				if (
					draggingChannel.type === "ThreadPublic" ||
					draggingChannel.type === "ThreadPrivate"
				) {
					validParents = ["Text", "Announcement", "Forum"];
				} else if (draggingChannel.type === "ThreadForum2") {
					validParents = ["Forum2"];
				}

				if (validParents.length > 0) {
					// if hovering over another thread, check if its parent is a valid target
					if (
						(targetChannel.type === "ThreadPublic" ||
							targetChannel.type === "ThreadPrivate" ||
							targetChannel.type === "ThreadForum2") &&
						targetChannel.parent_id
					) {
						// find parent
						const parent = channels2.cache.get(targetChannel.parent_id);
						if (parent && validParents.includes(parent.type)) {
							if (target()?.id !== parent.id) {
								setTarget({ id: parent.id, mode: "inside" });
							}
							return;
						}
					}

					// check if target itself is a valid parent
					if (validParents.includes(targetChannel.type)) {
						if (target()?.id !== id) {
							setTarget({ id, mode: "inside" });
						}
						return;
					}
					return;
				}
			}

			// if dragging a document
			if (draggingChannel.type === "Document") {
				if (targetChannel.type === "Wiki") {
					if (target()?.id !== id) {
						setTarget({ id, mode: "inside" });
					}
					return;
				}
				// if hovering over another document in a wiki, target the wiki
				if (targetChannel.type === "Document" && targetChannel.parent_id) {
					const p = channels2.cache.get(targetChannel.parent_id);
					if (p?.type === "Wiki") {
						if (target()?.id !== p.id) {
							setTarget({ id: p.id, mode: "inside" });
						}
						return;
					}
				}
			}

			// if dragging a regular channel (or category)
			if (draggingChannel.type === "Category") {
				// Can only reorder categories
				if (targetChannel.type === "Category") {
					const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
					const after = e.clientY > rect.top + rect.height / 2;
					const mode = after ? "after" : "before";
					if (target()?.id !== id || target()?.mode !== mode) {
						setTarget({ id, mode });
					}
				}
				return;
			}

			// Dragging a normal channel
			if (targetChannel.type === "Category") {
				// Move into category
				if (target()?.id !== id || target()?.mode !== "inside") {
					setTarget({ id, mode: "inside" });
				}
				return;
			} else if ((targetChannel.type as any) !== "Category") {
				// Reorder relative to other channel
				const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
				const after = e.clientY > rect.top + rect.height / 2;
				const mode = after ? "after" : "before";
				if (target()?.id !== id || target()?.mode !== mode) {
					setTarget({ id, mode });
				}
				return;
			}
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const dragInfo = dragging();
		const t = target();
		const toId = t?.id;
		const mode = t?.mode;

		setDragging(null);
		setTarget(null);

		if (!dragInfo || !toId || dragInfo.id === toId) return;

		// Handle voice participant move
		if (dragInfo.type === "voice") {
			const toChannel = channels2.cache.get(toId);
			if (!toChannel || toChannel.type !== "Voice") return;

			// Move the user to the target voice channel
			api2.client.http.POST(
				"/api/v1/voice/{channel_id}/member/{user_id}/move",
				{
					params: {
						path: {
							channel_id: dragInfo.channelId!,
							user_id: dragInfo.id,
						},
					},
					body: {
						target_id: toChannel.id,
					},
				},
			);
			return;
		}

		const fromChannel = channels2.cache.get(dragInfo.id);
		if (!fromChannel) return;
		const toChannel = channels2.cache.get(toId);
		if (!toChannel) return;

		// Handle thread/doc move (Reparenting)
		if (mode === "inside") {
			if (
				fromChannel.type === "ThreadPublic" ||
				fromChannel.type === "ThreadPrivate"
			) {
				if (
					toChannel.type === "Text" ||
					toChannel.type === "Announcement" ||
					toChannel.type === "Forum"
				) {
					if (fromChannel.parent_id !== toChannel.id) {
						channels2.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			if (fromChannel.type === "ThreadForum2") {
				if (toChannel.type === "Forum2") {
					if (fromChannel.parent_id !== toChannel.id) {
						channels2.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			if (fromChannel.type === "Document") {
				if (toChannel.type === "Wiki") {
					if (fromChannel.parent_id !== toChannel.id) {
						channels2.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			// Move channel into category
			if (toChannel.type === "Category" && fromChannel.type !== "Category") {
				channels2.update(fromChannel.id, { parent_id: toChannel.id });
				return;
			}
		}

		// Handle Reordering
		// We need to calculate the new order for the affected list.

		let targetParentId = toChannel.parent_id;
		if (toChannel.type === "Category") {
			// If we are reordering categories (target is category, from is category)
			// parent is null
			if (fromChannel.type === "Category") {
				targetParentId = null;
			} else {
				// Should have been handled by "inside" logic above?
				// If we dropped "before/after" a category?
				// That implies moving out of category to top level?
				// For now, let's assume dropping ON category is inside, dropping near channel is reorder.
			}
		} else {
			// Target is a normal channel
			// If dragging category, can only drop relative to another category (handled above?)
			// If dragging channel, target is sibling channel.
		}

		// Reconstruct the list logic
		const currentCategories = categories();

		// Find the "list" we are modifying.
		// It's either the top-level list of categories, OR a specific category's channels.

		let siblings: Channel[] = [];
		let newParentId: string | null | undefined ;

		if (fromChannel.type === "Category") {
			// Reordering categories
			siblings = currentCategories.map((c) => c.category).filter((c) =>
				c !== null
			) as Channel[];
			newParentId = null; // Categories are top level
		} else {
			// Reordering channels
			// Identify target parent
			if (toChannel.type === "Category") {
				// If we dropped relative to a category (not inside), that's top level?
				// But earlier logic says "Category" target means "inside".
				// So this branch might not be reached for Category targets unless we support before/after.
				// Let's assume toChannel is NOT Category here.
				return;
			} else {
				// toChannel is a sibling
				// Find its category
				const cat = currentCategories.find(
					(c) => (c.category?.id ?? null) === toChannel.parent_id,
				);
				if (cat) {
					siblings = [...cat.channels];
					newParentId = toChannel.parent_id;
				}
			}
		}

		// Remove from old list?
		// Actually, we can just build the new list based on the target parent.
		// If we are moving between categories, `siblings` above is the TARGET list.
		// We need to insert `fromChannel` into `siblings`.

		// Remove `fromChannel` from `siblings` if it's there (same category reorder)
		const fromIndex = siblings.findIndex((c) => c.id === fromChannel.id);
		if (fromIndex !== -1) {
			siblings.splice(fromIndex, 1);
		} else {
			// Moving from another category?
			// Nothing to remove from `siblings`
		}

		// Find insertion index
		let toIndex = siblings.findIndex((c) => c.id === toId);
		if (toIndex === -1) {
			// Should not happen
			return;
		}

		if (mode === "after") {
			toIndex++;
		}

		siblings.splice(toIndex, 0, fromChannel);

		// Send update
		const body = siblings.map((c, i) => ({
			id: c.id,
			parent_id: newParentId,
			position: i,
		}));

		// If we moved FROM another category, we should also update the old category to close gaps?
		// The backend `ChannelReorder` updates positions for provided channels.
		// If we don't provide the old category channels, their positions remain.
		// It's polite to reorder the old category too.

		if (
			fromChannel.parent_id !== newParentId && fromChannel.type !== "Category"
		) {
			const oldCat = currentCategories.find(
				(c) => (c.category?.id ?? null) === fromChannel.parent_id,
			);
			if (oldCat) {
				const oldSiblings = oldCat.channels.filter((c) =>
					c.id !== fromChannel.id
				);
				body.push(...oldSiblings.map((c, i) => ({
					id: c.id,
					parent_id: fromChannel.parent_id,
					position: i,
				})));
			}
		}

		api2.client.http.PATCH("/api/v1/room/{room_id}/channel", {
			params: { path: { room_id: props.room_id! } },
			body: {
				channels: body,
			},
		});
	};

	const getDragMode = (id: string) => {
		if (dragging()?.type === "channel" && target()?.id === id) {
			return target()?.mode; // "before", "after", "inside"
		}
		return undefined;
	};

	const isDraggingThis = (id: string) =>
		dragging()?.type === "channel" && dragging()?.id === id;

	const isVoiceTarget = (id: string) =>
		dragging()?.type === "voice" && target()?.id === id &&
		target()?.mode === "inside";

	return (
		<nav id="channel-nav">
			<Show when={flags.has("nav_header")}>
				<header
					classList={{
						"menu-room": !!props.room_id,
					}}
					data-room-id={props.room_id}
					onClick={(e) => {
						if (props.room_id) {
							queueMicrotask(() => {
								setMenu({
									x: e.clientX,
									y: e.clientY,
									type: "room",
									room_id: props.room_id!,
								});
							});
						}
					}}
				>
					{props.room_id ? (room()?.name ?? "loading...") : "home"}
				</header>
			</Show>

			<ul class="channel-list">
				<li class="channel-item">
					<A
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="channel-link"
						draggable={false}
						end
					>
						<img src={icHome} class="icon" /> home
					</A>
				</li>

				<Show when={!props.room_id}>
					<Show when={flags.has("inbox")}>
						<li class="channel-item">
							<A
								href="/inbox"
								class="channel-link"
								draggable={false}
								end
							>
								<img src={icInbox} class="icon" /> inbox
							</A>
						</li>
					</Show>
				</Show>

				<For each={categories()}>
					{({ category, channels }) => (
						<>
							<Show when={category}>
								<div
									class="category-header"
									classList={{
										collapsed: collapsedCategories().has(category!.id),
									}}
									data-drag-mode={getDragMode(category!.id)}
									data-is-dragging={isDraggingThis(category!.id)}
									data-channel-id={category!.id}
									draggable="true"
									onDragStart={handleDragStart}
									onDragOver={handleDragOver}
									onDrop={handleDrop}
									onDragEnd={() => {
										setDragging(null);
										setTarget(null);
									}}
									onClick={() => {
										setCollapsedCategories((prev) => {
											const newSet = new Set(prev);
											if (newSet.has(category!.id)) {
												newSet.delete(category!.id);
											} else {
												newSet.add(category!.id);
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
												channel_id: category!.id,
											});
										});
									}}
								>
									<span class="category-toggle">
										{collapsedCategories().has(category!.id) ? "▶" : "▼"}
									</span>
									<span class="category-name">{category!.name}</span>
								</div>
							</Show>
							<Show
								when={!category || !collapsedCategories().has(category!.id)}
							>
								<ul class="category-channels">
									<For
										each={channels}
										fallback={
											<div class="empty-text" style="margin-left: 16px">
												(no channels)
											</div>
										}
									>
										{(channel) => (
											<li
												class="channel-item"
												data-drag-mode={getDragMode(channel.id)}
												data-is-dragging={isDraggingThis(channel.id)}
												data-voice-target={isVoiceTarget(channel.id)
													? "true"
													: undefined}
												data-channel-id={channel.id}
												draggable="true"
												onDragStart={handleDragStart}
												onDragOver={handleDragOver}
												onDrop={handleDrop}
												onDragEnd={() => {
													setDragging(null);
													setTarget(null);
												}}
											>
												<ItemChannel
													channel={channel}
													room_id={props.room_id}
												/>
												<Show when={(channel as any).threads?.length > 0}>
													<ul class="thread-list">
														<For each={(channel as any).threads}>
															{(thread: Channel) => (
																<li
																	class="channel-item"
																	data-channel-id={thread.id}
																	draggable={true}
																	onDragStart={handleDragStart}
																	onDragOver={handleDragOver}
																	onDrop={handleDrop}
																	onDragEnd={() => {
																		setDragging(null);
																		setTarget(null);
																	}}
																	classList={{
																		unread: thread.type !== "Voice" &&
																			!!thread.is_unread,
																	}}
																>
																	<ItemChannel
																		channel={thread}
																		room_id={props.room_id}
																	/>
																</li>
															)}
														</For>
													</ul>
												</Show>
												<For
													each={[...api2.voiceStates.values()].filter((i) =>
														i.channel_id === channel.id
													).sort((a, b) =>
														Date.parse(a.joined_at) - Date.parse(b.joined_at)
													)}
												>
													{(s) => {
														const user = () => users2.cache.get(s.user_id);
														const room_member = () =>
															props.room_id
																? roomMembers2.cache.get(
																	`${props.room_id!}:${s.user_id}`,
																)
																: null;
														const name = () =>
															room_member()?.override_name || user()?.name ||
															"unknown user";
														return (
															<div
																class="voice-participant menu-user"
																classList={{
																	speaking: ((voice.rtc?.speaking.get(s.user_id)
																		?.flags ??
																		0) &
																		1) === 1,
																}}
																data-channel-id={s.channel_id}
																data-user-id={s.user_id}
																draggable={true}
																onDragStart={(e) =>
																	handleVoiceDragStart(
																		e,
																		s.user_id,
																		s.channel_id,
																	)}
																onDragEnd={() => {
																	setDragging(null);
																	setTarget(null);
																}}
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
	const api2 = useApi2();
	const channels2 = useChannels2();
	const rooms2 = useRooms2();
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const user = useCurrentUser();
	const currentUserId = () => user()?.id;
	const [hovered, setHovered] = createSignal(false);

	const handleClick = (e: MouseEvent) => {
		if (props.room_id) {
			setLastViewedChannel(props.room_id, props.channel.id);
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
		if (c.notifs.mute.expires_at === null) return true;
		return Date.parse(c.notifs.mute.expires_at!) > Date.now();
	};

	const canInvite = createMemo(() => {
		if (!props.room_id || !currentUserId()) {
			return false;
		}

		const permissionContext: PermissionContext = {
			api: api2,
			room_id: props.room_id,
			channel_id: props.channel.id,
		};

		const { permissions } = calculatePermissions(
			permissionContext,
			currentUserId()!,
		);

		return permissions.has("InviteCreate");
	});

	const isDm = () =>
		props.channel.type === "Dm" || props.channel.type === "Gdm";

	return (
		<A
			href={`/channel/${props.channel.id}`}
			class="menu-channel channel-link"
			data-unread={props.channel.type !== "Voice" && !!props.channel.is_unread
				? "true"
				: undefined}
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
					when={otherUser()?.presence.activities.find((a) =>
						a.type === "Custom"
					)?.text}
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
							class="action-button"
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
							<img src={icMemberAdd} alt="Invite" />
						</button>
					</Show>

					<button
						class="action-button"
						title="Channel Settings"
						onClick={(e) => {
							e.preventDefault();
							e.stopPropagation();
							nav(`/channel/${props.channel.id}/settings`);
						}}
					>
						<img src={icSettings} alt="Settings" />
					</button>
				</div>
			</Show>
		</A>
	);
};
