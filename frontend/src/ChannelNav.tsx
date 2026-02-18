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
import { useApi } from "./api";
import { useConfig } from "./config";
import { flags } from "./flags";
import { useCtx } from "./context";
import { Avatar, AvatarWithStatus, ChannelIcon, ChannelIconGdm } from "./User";
import { useVoice } from "./voice-provider";
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
	const api = useApi();
	const [voice] = useVoice();
	const ctx = useCtx();
	const params = useParams();
	const nav = useNavigate();

	const currentUserId = () => api.users.cache.get("@self")?.id;

	createEffect(() => {
		if (!props.room_id) return;

		if (!flags.has("auto_redirect_last_channel")) return;

		const lastChannelId = getLastViewedChannel(props.room_id);
		if (!lastChannelId) return;

		const currentPath = params.channel_id;
		if (currentPath && currentPath === lastChannelId) return;

		const channel = api.channels.cache.get(lastChannelId);
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

	const [categories, setCategories] = createSignal<
		Array<{ category: Channel | null; channels: Array<Channel> }>
	>([]);

	createEffect(() => {
		if (props.room_id) {
			api.channels.list(() => props.room_id!);
		} else {
			api.dms.list();
		}
	});

	const room = props.room_id
		? api.rooms.fetch(() => props.room_id!)
		: () => null;

	const canViewChannel = (channel: Channel): boolean => {
		if (!props.room_id || !currentUserId()) {
			return true;
		}

		const permissionContext: PermissionContext = {
			api,
			room_id: props.room_id,
			channel_id: channel.id,
		};

		const { permissions } = calculatePermissions(
			permissionContext,
			currentUserId()!,
		);

		return permissions.has("ViewChannel");
	};

	// update list when room changes
	createEffect(() => {
		const allChannels = [...api.channels.cache.values()].filter(
			(c) =>
				(props.room_id ? c.room_id === props.room_id : c.room_id === null) &&
				!c.deleted_at,
		);

		const threads = allChannels.filter(
			(c) =>
				(c.type === "ThreadPublic" ||
					c.type === "ThreadPrivate" ||
					(c.type === "Document" &&
						c.parent_id &&
						api.channels.cache.get(c.parent_id)?.type === "Wiki")) &&
				!c.archived_at &&
				canViewChannel(c),
		);
		const channels = allChannels.filter(
			(c) =>
				c.type !== "ThreadPublic" &&
				c.type !== "ThreadPrivate" &&
				!(
					c.type === "Document" &&
					c.parent_id &&
					api.channels.cache.get(c.parent_id)?.type === "Wiki"
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
			// parent_id should assume to be present if it was filtered as a thread/nested doc
			// but we handle null just in case
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

		const categories = new Map<
			string | null,
			Array<Channel & { threads: Channel[] }>
		>();
		for (const c of channelMap.values()) {
			if (c.type === "Category") {
				if (canViewChannel(c)) {
					const cat = categories.get(c.id) ?? [];
					categories.set(c.id, cat);
				}
			} else {
				const children = categories.get(c.parent_id!) ?? [];
				children.push(c);
				categories.set(c.parent_id!, children);
			}
		}
		const list = [...categories.entries()]
			.map(([cid, cs]) => ({
				category: cid ? api.channels.cache.get(cid)! : null,
				channels: cs,
			}))
			.filter(({ channels }) => {
				// categories and should still be shown if they have visible channels
				return channels.length > 0;
			})
			.sort((a, b) => {
				// null category comes first
				if (!a.category) return -1;
				if (!b.category) return 1;

				// categories with positions come first
				if (a.category.position === null && b.category.position === null) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (a.category.position === null) return 1;
				if (b.category.position === null) return -1;

				// order by position
				const p = a.category.position! - b.category.position!;
				if (p === 0) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}

				return p;
			});
		setCategories(list as any);
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
			const targetChannel = api.channels.cache.get(id);
			if (targetChannel?.type === "Voice") {
				if (target()?.id !== id) {
					setTarget({ id, mode: "inside" });
				}
			}
			return;
		}

		const draggingId = dragInfo?.id;
		if (draggingId) {
			const draggingChannel = api.channels.cache.get(draggingId);
			const targetChannel = api.channels.cache.get(id);

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
						const parent = api.channels.cache.get(targetChannel.parent_id);
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
					const p = api.channels.cache.get(targetChannel.parent_id);
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
			} else if (targetChannel.type !== "Category") {
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
			const toChannel = api.channels.cache.get(toId);
			if (!toChannel || toChannel.type !== "Voice") return;

			// Move the user to the target voice channel
			api.client.http.POST("/api/v1/voice/{channel_id}/member/{user_id}/move", {
				params: {
					path: {
						channel_id: dragInfo.channelId!,
						user_id: dragInfo.id,
					},
				},
				body: {
					target_id: toChannel.id,
				},
			});
			return;
		}

		const fromChannel = api.channels.cache.get(dragInfo.id);
		if (!fromChannel) return;
		const toChannel = api.channels.cache.get(toId);
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
						api.channels.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			if (fromChannel.type === "ThreadForum2") {
				if (toChannel.type === "Forum2") {
					if (fromChannel.parent_id !== toChannel.id) {
						api.channels.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			if (fromChannel.type === "Document") {
				if (toChannel.type === "Wiki") {
					if (fromChannel.parent_id !== toChannel.id) {
						api.channels.update(fromChannel.id, { parent_id: toChannel.id });
					}
				}
				return;
			}

			// Move channel into category
			if (toChannel.type === "Category" && fromChannel.type !== "Category") {
				api.channels.update(fromChannel.id, { parent_id: toChannel.id });
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
		let newParentId: string | null | undefined = undefined;

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

		api.client.http.PATCH("/api/v1/room/{room_id}/channel", {
			params: { path: { room_id: props.room_id! } },
			body: {
				channels: body,
			},
		});
	};

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
								ctx.setMenu({
									x: e.clientX,
									y: e.clientY,
									type: "room",
									room_id: props.room_id,
								});
							});
						}
					}}
				>
					{props.room_id ? (room()?.name ?? "loading...") : "home"}
				</header>
			</Show>

			<ul>
				<li>
					<A
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="menu-channel nav-channel"
						draggable={false}
						end
					>
						<img src={icHome} class="icon" /> home
					</A>
				</li>

				<Show when={!props.room_id}>
					<Show when={flags.has("inbox")}>
						<li>
							<A
								href="/inbox"
								class="menu-channel nav-channel"
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
									class="dim"
									style="margin-left:8px;margin-top:8px"
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
											ctx.setMenu({
												x: e.clientX,
												y: e.clientY,
												type: "channel",
												channel_id: category!.id,
											});
										});
									}}
									classList={{
										dragging: dragging()?.type === "channel" &&
											dragging()?.id === category!.id,
										collapsed: collapsedCategories().has(category!.id),
										category: true,
										"drop-before": dragging()?.type === "channel" &&
											target()?.id === category!.id &&
											target()?.mode === "before",
										"drop-after": dragging()?.type === "channel" &&
											target()?.id === category!.id &&
											target()?.mode === "after",
										"drop-inside": dragging()?.type === "channel" &&
											target()?.id === category!.id &&
											target()?.mode === "inside",
									}}
								>
									<span class="category-toggle">
										{collapsedCategories().has(category!.id) ? "▶" : "▼"}
									</span>
									{category!.name}
								</div>
							</Show>
							<Show
								when={!category || !collapsedCategories().has(category!.id)}
							>
								<For
									each={channels}
									fallback={
										<div class="dim" style="margin-left: 16px">
											(no channels)
										</div>
									}
								>
									{(channel) => (
										<li
											data-channel-id={channel.id}
											draggable="true"
											onDragStart={handleDragStart}
											onDragOver={handleDragOver}
											onDrop={handleDrop}
											onDragEnd={() => {
												setDragging(null);
												setTarget(null);
											}}
											class="toplevel"
											classList={{
												dragging: dragging()?.type === "channel" &&
													dragging()?.id === channel.id,
												unread: channel.type !== "Voice" && !!channel.is_unread,
												"voice-channel-target": dragging()?.type === "voice" &&
													target()?.id === channel.id &&
													target()?.mode === "inside",
												"channel-reorder-target":
													dragging()?.type === "channel" &&
													target()?.id === channel.id &&
													target()?.mode === "inside",
												"drop-before": dragging()?.type === "channel" &&
													target()?.id === channel.id &&
													target()?.mode === "before",
												"drop-after": dragging()?.type === "channel" &&
													target()?.id === channel.id &&
													target()?.mode === "after",
											}}
										>
											<ItemChannel channel={channel} room_id={props.room_id} />
											<Show when={(channel as any).threads?.length > 0}>
												<ul class="threads">
													<For each={(channel as any).threads}>
														{(thread: Channel) => (
															<li
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
												each={[...api.voiceStates.values()].filter((i) =>
													i.channel_id === channel.id
												).sort((a, b) =>
													Date.parse(a.joined_at) - Date.parse(b.joined_at)
												)}
											>
												{(s) => {
													const user = api.users.fetch(() => s.user_id);
													const room_member = props.room_id
														? api.room_members.fetch(
															() => props.room_id!,
															() => s.user_id,
														)
														: () => null;
													const name = () =>
														room_member()?.override_name || user()?.name ||
														"unknown user";
													return (
														<div
															class="voice-participant menu-user"
															classList={{
																speaking:
																	((voice.rtc?.speaking.get(s.user_id)?.flags ??
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
	const api = useApi();
	const nav = useNavigate();

	const handleClick = (e: MouseEvent) => {
		if (props.room_id) {
			setLastViewedChannel(props.room_id, props.channel.id);
		}
	};

	const otherUser = createMemo(() => {
		if (props.channel.type === "Dm") {
			const selfId = api.users.cache.get("@self")!.id;
			return props.channel.recipients.find((i) => i.id !== selfId);
		}
		return undefined;
	});

	const name = () => {
		if (props.channel.type === "Dm") {
			return otherUser()?.name ?? "dm";
		}

		return props.channel.name;
	};

	const channelConfig = () => props.channel.user_config;

	const isMuted = () => {
		const c = channelConfig();
		if (!c?.notifs.mute) return false;
		if (c.notifs.mute.expires_at === null) return true;
		return Date.parse(c.notifs.mute.expires_at) > Date.now();
	};

	return (
		<A
			href={`/channel/${props.channel.id}`}
			class="menu-channel nav-channel"
			classList={{
				unread: props.channel.type !== "Voice" && !!props.channel.is_unread,
				muted: isMuted(),
			}}
			data-channel-id={props.channel.id}
			onClick={handleClick}
		>
			<ChannelIcon channel={props.channel} />
			<div style="pointer-events:none;line-height:1;flex:1;overflow:hidden">
				<div
					style={{
						"text-overflow": "ellipsis",
						overflow: "hidden",
						"white-space": "nowrap",
					}}
				>
					{name()}
				</div>
				<Show
					when={otherUser()?.presence.activities.find((a) =>
						a.type === "Custom"
					)?.text}
				>
					{(t) => <div class="dim">{t()}</div>}
				</Show>
			</div>
			<Show when={props.channel.mention_count}>
				<div class="mentions">{props.channel.mention_count}</div>
			</Show>
			<Show when={true}>
				<button onClick={() => {/* TODO: show invite modal */ }}>
					<img class="icon" src={icMemberAdd} />
				</button>
				<button
					onClick={(e) => {
						nav(`/channel/${props.channel.id}/settings`);
						e.preventDefault();
						e.stopPropagation();
						e.stopImmediatePropagation();
					}}
				>
					<img class="icon" src={icSettings} />
				</button>
			</Show>
		</A>
	);
};
