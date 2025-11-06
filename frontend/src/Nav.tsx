import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { A, useNavigate, useParams } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Channel } from "sdk";
import { flags } from "./flags.ts";
import { useVoice } from "./voice-provider.tsx";
import { useConfig } from "./config.tsx";
import { Avatar, AvatarWithStatus, ChannelIcon } from "./User.tsx";
import { getThumbFromId } from "./media/util.tsx";

export const ChannelNav = (props: { room_id?: string }) => {
	const config = useConfig();
	const api = useApi();
	const [voice] = useVoice();
	const params = useParams();
	const nav = useNavigate();

	// track drag ids
	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(
		null,
	);

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

	// update list when room changes
	createEffect(() => {
		const allChannels = [...api.channels.cache.values()].filter((c) =>
			props.room_id ? c.room_id === props.room_id : c.room_id === null
		);

		const threads = allChannels.filter(
			(c) => c.type === "ThreadPublic" || c.type === "ThreadPrivate",
		);
		const channels = allChannels.filter(
			(c) => c.type !== "ThreadPublic" && c.type !== "ThreadPrivate",
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
			const parent = channelMap.get(thread.parent_id!);
			if (parent) {
				parent.threads.push(thread);
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
				const cat = categories.get(c.id) ?? [];
				categories.set(c.id, cat);
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

	const previewedCategories = createMemo(() => {
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;
		const cats = categories();

		if (!fromId || !toId || fromId === toId) return cats;

		const fromChannel = api.channels.cache.get(fromId);
		const toChannel = api.channels.cache.get(toId);
		if (!fromChannel || !toChannel) return cats;

		const newCategories = cats.map((c) => ({
			category: c.category,
			channels: [...c.channels],
		}));

		const fromCat = newCategories.find(
			(c) => (c.category?.id ?? null) === fromChannel.parent_id,
		);
		if (!fromCat) return cats;
		const fromIndex = fromCat.channels.findIndex((c) => c.id === fromId);
		if (fromIndex === -1) return cats;

		const [moved] = fromCat.channels.splice(fromIndex, 1);

		if (toChannel.type === "Category") {
			const toCat = newCategories.find((c) => c.category?.id === toId);
			if (!toCat) return cats;
			if (after) toCat.channels.push(moved);
			else toCat.channels.unshift(moved);
		} else {
			const toCat = newCategories.find(
				(c) => (c.category?.id ?? null) === toChannel.parent_id,
			);
			if (!toCat) return cats;
			let toIndex = toCat.channels.findIndex((c) => c.id === toId);
			if (toIndex === -1) return cats;
			if (after) toIndex++;
			toCat.channels.splice(toIndex, 0, moved);
		}

		return newCategories;
	});

	// helper to get channel id from the element's data attribute
	const getChannelId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.channelId;

	const handleDragStart = (e: DragEvent) => {
		const id = getChannelId(e);
		if (id) setDragging(id);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		const id = getChannelId(e);
		if (!id || id === dragging()) {
			return;
		}
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;

		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) return;

		const fromChannel = api.channels.cache.get(fromId);
		const toChannel = api.channels.cache.get(toId);
		if (!fromChannel || !toChannel) return;

		const fromCategory = categories().find(
			(c) => (c.category?.id ?? null) === fromChannel.parent_id,
		);
		if (!fromCategory) return;

		const fromIndex = fromCategory.channels.findIndex((c) => c.id === fromId);
		if (fromIndex === -1) return;

		let toCategory;
		let toIndex;
		let newParentId;

		if (toChannel.type === "Category") {
			toCategory = categories().find((c) => c.category?.id === toId);
			if (!toCategory) return;
			toIndex = after ? toCategory.channels.length : 0;
			newParentId = toId;
		} else {
			toCategory = categories().find(
				(c) => (c.category?.id ?? null) === toChannel.parent_id,
			);
			if (!toCategory) return;
			toIndex = toCategory.channels.findIndex((c) => c.id === toId);
			if (toIndex === -1) return;
			if (after) toIndex++;
			newParentId = toChannel.parent_id;
		}

		const reordered = [...toCategory.channels];
		if (fromCategory === toCategory) {
			if (fromIndex < toIndex) toIndex--;
			const [moved] = reordered.splice(fromIndex, 1);
			reordered.splice(toIndex, 0, moved);
		} else {
			reordered.splice(toIndex, 0, fromChannel);
		}

		if (
			fromCategory === toCategory &&
			JSON.stringify(fromCategory.channels.map((c) => c.id)) ===
				JSON.stringify(reordered.map((c) => c.id))
		) {
			return;
		}

		const body = reordered.map((c, i) => ({
			id: c.id,
			parent_id: newParentId,
			position: i,
		}));

		if (fromCategory !== toCategory) {
			const sourceBody = fromCategory.channels
				.filter((c) => c.id !== fromId)
				.map((c, i) => ({
					id: c.id,
					parent_id: fromChannel.parent_id,
					position: i,
				}));
			body.push(...sourceBody);
		}

		api.client.http.PATCH("/api/v1/room/{room_id}/channel", {
			params: { path: { room_id: props.room_id! } },
			body: {
				channels: body,
			},
		});
	};

	return (
		<nav id="nav">
			<Show when={flags.has("nav_header")}>
				<header>
					{props.room_id ? (room()?.name ?? "loading...") : "home"}
				</header>
			</Show>

			<ul>
				<li>
					<A
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="menu-channel"
						draggable={false}
						end
					>
						home
					</A>
				</li>

				<Show when={!props.room_id}>
					<Show when={flags.has("inbox")}>
						<li>
							<A
								href="/inbox"
								class="menu-channel"
								draggable={false}
								end
							>
								inbox
							</A>
						</li>
					</Show>
				</Show>

				<For each={previewedCategories()}>
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
									classList={{
										dragging: dragging() === category!.id,
										collapsed: collapsedCategories().has(category!.id),
										category: true,
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
											classList={{
												dragging: dragging() === channel.id,
												unread: channel.type !== "Voice" && !!channel.is_unread,
											}}
										>
											<ItemChannel channel={channel} />
											<Show when={(channel as any).threads?.length > 0}>
												<ul class="threads">
													<For each={(channel as any).threads}>
														{(thread: Channel) => (
															<li
																data-channel-id={thread.id}
																draggable={false}
																classList={{
																	unread: thread.type !== "Voice" &&
																		!!thread.is_unread,
																}}
															>
																<ItemChannel channel={thread} />
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
													// <svg viewBox="0 0 32 32" style="height:calc(1em + 4px);margin-right:8px" preserveAspectRatio="none">
													// 	<line x1={0} y1={0} x2={0} y2={32} stroke-width={4} style="stroke:white"/>
													// 	<line x1={0} y1={32} x2={32} y2={32} stroke-width={4} style="stroke:white"/>
													// </svg>

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

const ItemChannel = (props: { channel: Channel }) => {
	const api = useApi();
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

	return (
		<A
			href={`/channel/${props.channel.id}`}
			class="menu-channel nav-channel"
			classList={{
				unread: props.channel.type !== "Voice" && !!props.channel.is_unread,
			}}
			data-channel-id={props.channel.id}
		>
			<Switch>
				<Match when={props.channel.type === "Dm" && otherUser()}>
					<AvatarWithStatus user={otherUser()} />
				</Match>
				<Match when={props.channel.type === "Gdm"}>
					<ChannelIcon id={props.channel.id} icon={props.channel.icon} />
				</Match>
			</Switch>
			<div style="pointer-events:none;line-height:1">
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
		</A>
	);
};
