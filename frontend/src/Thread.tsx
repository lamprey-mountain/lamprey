import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type { Channel } from "sdk";
import { useApi } from "./api.tsx";
import { AvatarWithStatus } from "./User.tsx";
import { useCtx } from "./context.ts";
import { ReactiveMap } from "@solid-primitives/map";

export const ThreadMembers = (props: { thread: Channel }) => {
	const api = useApi();
	const thread_id = () => props.thread.id;
	const room_id = () => props.thread.room_id;
	const list = () => api.memberLists.get(thread_id());
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

	createEffect(() => {
		api.thread_members.subscribeList(thread_id(), [[0, 199]]);
	});

	const getGroupName = (group: any) => {
		if (typeof group.id === "object" && group.id.Role) {
			const role = api.roles.cache.get(group.id.Role);
			return role?.name ?? "Role";
		}
		return group.id;
	};

	return (
		<div class="member-list" data-thread-id={props.thread.id}>
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
									api.thread_members.cache.get(thread_id())?.get(
										row.item.user.id,
									) ?? row.item.thread_member;
								const user = () =>
									api.users.cache.get(row.item.user.id) ?? row.item.user;
								const room_member = props.thread?.room_id
									? api.room_members.fetch(
										room_id,
										() => user()!.id,
									)
									: () => null;
								const ctx = useCtx();

								function name() {
									let name: string | undefined | null = null;
									const rm = room_member();
									if (rm?.membership === "Join") {
										name ??= rm.override_name;
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
													room_id: room_id(),
													thread_id: thread_id(),
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
