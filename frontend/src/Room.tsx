import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type { RoomT } from "./types.ts";
import { useCtx } from "./context.ts";
import { useModals } from "./contexts/modal";
import { type Channel, getTimestampFromUUID } from "sdk";
import { A, useNavigate } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { AvatarWithStatus } from "./User.tsx";
import { Time } from "./Time.tsx";
import { usePermissions } from "./hooks/usePermissions.ts";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { md } from "./markdown.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { useMemberList } from "./contexts/memberlist.tsx";

export const RoomMembers = (props: { room: RoomT }) => {
	const api = useApi();
	const memberLists = useMemberList();
	const room_id = () => props.room.id;
	const list = () => memberLists.get(room_id());
	const [collapsedGroups, setCollapsedGroups] = createSignal(
		new ReactiveMap<string, boolean>(),
	);

	const rows = createMemo(() => {
		const l = list();
		if (!l) return [];
		const rows: (
			| { type: "group"; group: any }
			| { type: "member"; item: any }
		)[] = [];
		let offset = 0;
		for (const group of l.groups) {
			if (group.count === 0) continue;
			const groupId = JSON.stringify(group.id);
			rows.push({ type: "group", group });
			if (!collapsedGroups().get(groupId)) {
				const members = l.items.slice(offset, offset + group.count);
				for (const member of members) {
					rows.push({ type: "member", item: member });
				}
			}
			offset += group.count;
		}
		return rows;
	});

	const getGroupName = (group: any) => {
		const role = api.roles.cache.get(group.id);
		return role?.name ?? group.id;
	};

	return (
		<div class="member-list" data-room-id={props.room.id}>
			<For each={rows()}>
				{(row) => {
					return row.type === "group"
						? (
							<div
								class="dim"
								style="margin-top:4px;margin-left:8px; cursor: pointer;"
								onClick={() => {
									const groupId = JSON.stringify(row.group.id);
									const newMap = new ReactiveMap(collapsedGroups());
									newMap.set(groupId, !newMap.get(groupId));
									setCollapsedGroups(newMap);
								}}
							>
								{getGroupName(row.group)} — {row.group.count}
							</div>
						)
						: (
							(() => {
								const member = () =>
									api.room_members.cache.get(room_id())?.get(
										row.item.user.id,
									) ??
										row.item.room_member;
								const user = () =>
									api.users.cache.get(row.item.user.id) ?? row.item.user;

								const ctx = useCtx();
								function name() {
									let name: string | undefined | null = null;
									if (member()?.membership === "Join") {
										name ??= member()!.override_name;
									}
									name ??= user()?.name;
									return name;
								}

								return (
									<li
										class="menu-user"
										data-user-id={row.item.user.id}
										onClick={(e) => {
											e.stopPropagation();
											const currentTarget = e.currentTarget as HTMLElement;
											if (ctx.userView()?.ref === currentTarget) {
												ctx.setUserView(null);
											} else {
												ctx.setUserView({
													user_id: user()!.id,
													room_id: props.room.id,
													ref: currentTarget,
													source: "member-list",
												});
											}
										}}
									>
										<AvatarWithStatus user={user()} />
										<span class="text">
											<span class="name">{name()}</span>
										</span>
									</li>
								);
							})()
						);
				}}
			</For>
		</div>
	);
};

// TODO: show online/member count
// TODO: show feed with important messages/highlights
// TODO: show active channels
// TODO: show invite button
export const RoomHome = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const room_id = () => props.room.id;

	const [threadFilter, setThreadFilter] = createSignal("active");

	const fetchMore = () => {
		return api.channels.list(room_id);
		// const filter = threadFilter();
		// if (filter === "active") {
		// 	return api.threads.list(room_id);
		// } else if (filter === "archived") {
		// 	return api.threads.listArchived(room_id);
		// } else if (filter === "removed") {
		// 	return api.threads.listRemoved(room_id);
		// }
	};

	const threadsResource = createMemo(fetchMore);

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const categorizedChannels = createMemo(() => {
		const items = threadsResource()?.()?.items;
		if (!items) return [];

		const allChannels: Channel[] = [...items];

		const threads = allChannels.filter(
			(c) => c.type === "ThreadPublic" || c.type === "ThreadPrivate",
		);
		const channels = allChannels.filter(
			(c) => c.type !== "ThreadPublic" && c.type !== "ThreadPrivate",
		);

		channels.sort((a, b) => {
			if (a.position === null && b.position === null) {
				return a.id < b.id ? 1 : -1;
			}
			if (a.position === null) return 1;
			if (b.position === null) return -1;
			return a.position! - b.position!;
		});

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
		return list;
	});

	function createThread(room_id: string) {
		modalCtl.open({
			type: "channel_create",
			room_id: room_id,
			cont: (data) => {
				if (!data) return;
				ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
					params: {
						path: { room_id },
					},
					body: {
						name: data.name,
						type: data.type,
					},
				});
			},
		});
	}

	function leaveRoom(_room_id: string) {
		modalCtl.confirm("are you sure you want to leave?", (confirmed) => {
			if (!confirmed) return;
			ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
				params: {
					path: {
						room_id: props.room.id,
						user_id: api.users.cache.get("@self")!.id,
					},
				},
			});
		});
	}

	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	return (
		<div class="room-home">
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.room.name}</h2>
					<p
						class="markdown"
						innerHTML={md(props.room.description ?? "") as string}
					>
					</p>
				</div>
				<div style="display:flex;flex-direction:column;gap:4px">
					<button onClick={() => leaveRoom(room_id())}>leave room</button>
					<A style="padding: 0 4px" href={`/room/${props.room.id}/settings`}>
						settings
					</A>
				</div>
			</div>
			<div style="display:flex; align-items:center">
				<h3 style="font-size:1rem; margin-top:8px;flex:1">
					{threadsResource()?.()?.total} channels
				</h3>
				{
					/*
				<div class="thread-filter">
					<button
						classList={{ selected: threadFilter() === "active" }}
						onClick={[setThreadFilter, "active"]}
					>
						active
					</button>
					<button
						classList={{ selected: threadFilter() === "archived" }}
						onClick={[setThreadFilter, "archived"]}
					>
						archived
					</button>
					<Show when={perms.has("ThreadManage")}>
						<button
							classList={{ selected: threadFilter() === "removed" }}
							onClick={[setThreadFilter, "removed"]}
						>
							removed
						</button>
					</Show>
				</div>
				*/
				}
				<button
					class="primary"
					style="margin-left: 8px;border-radius:4px"
					onClick={() => createThread(room_id())}
				>
					create channel
				</button>
			</div>
			<For each={categorizedChannels()}>
				{({ category, channels }) => (
					<>
						<h3 class="dim" style="margin-top:12px;margin-bottom:4px">
							{category?.name}
						</h3>
						<ul>
							<For each={channels}>
								{(thread) => (
									<li>
										<article
											class="thread menu-thread thread-card"
											data-thread-id={thread.id}
										>
											<header onClick={() => nav(`/thread/${thread.id}`)}>
												<div class="top">
													<div class="icon"></div>
													<div class="spacer">{thread.name}</div>
													<div class="time">
														Created{" "}
														<Time date={getTimestampFromUUID(thread.id)} />
													</div>
												</div>
												<div
													class="bottom"
													onClick={() => nav(`/thread/${thread.id}`)}
												>
													<div class="dim">
														{thread.message_count} message(s) &bull; last msg
														{" "}
														<Time
															date={getTimestampFromUUID(
																thread.last_version_id ?? thread.id,
															)}
														/>
													</div>
													<Show when={thread.description}>
														<div
															class="description markdown"
															innerHTML={md(thread.description ?? "") as string}
														>
														</div>
													</Show>
												</div>
											</header>
										</article>
									</li>
								)}
							</For>
						</ul>
					</>
				)}
			</For>
			<div ref={setBottom}></div>
		</div>
	);
};
