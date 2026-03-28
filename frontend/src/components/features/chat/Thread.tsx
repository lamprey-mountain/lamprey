import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { Channel, Role, RoomMember, User } from "sdk";
import {
	useApi2,
	useRoles2,
	useRoomMembers2,
	useThreadMembers2,
	useUsers2,
} from "@/api";
import { AvatarWithStatus } from "../../../User.tsx";
import { useCtx } from "../../../context.ts";
import { useUserPopout } from "../../../contexts/mod.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { useMemberList } from "../../../contexts/memberlist.tsx";

export const ThreadMembers = (props: { thread: Channel }) => {
	const api2 = useApi2();
	const roles2 = useRoles2();
	const threadMembers2 = useThreadMembers2();
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();
	const memberLists = useMemberList();
	const thread_id = () => props.thread.id;
	const room_id = () => props.thread.room_id;
	const list = () => memberLists.get(thread_id());
	const [collapsedGroups, setCollapsedGroups] = createSignal(
		new ReactiveMap<string, boolean>(),
	);

	type Row =
		| { type: "group"; group: import("sdk").MemberListGroup }
		| {
			type: "member";
			item: import("@/api/services/MemberListService").MemberListItem;
		};

	const rows = createMemo(() => {
		const l = list();
		if (!l) return [];
		const rows: Row[] = [];
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

	const getGroupName = (group: import("sdk").MemberListGroup) => {
		if (typeof group.id === "string") {
			const role = roles2.cache.get(group.id);
			return role?.name ?? group.id;
		}
		return JSON.stringify(group.id);
	};

	let parentRef!: HTMLDivElement;

	const rowVirtualizer = createVirtualizer({
		get count() {
			return rows().length;
		},
		getScrollElement: () => parentRef,
		estimateSize: (i) => {
			const row = rows()[i];
			return row.type === "group" ? 28 : 44;
		},
		overscan: 5,
	});

	return (
		<div ref={parentRef} class="member-list" data-thread-id={props.thread.id}>
			<div
				style={{
					height: `${rowVirtualizer.getTotalSize()}px`,
					width: "100%",
					position: "relative",
				}}
			>
				<For each={rowVirtualizer.getVirtualItems()}>
					{(virtualRow) => {
						const row = rows()[virtualRow.index];
						return (
							<div
								style={{
									position: "absolute",
									top: 0,
									left: 0,
									width: "100%",
									transform: `translateY(${virtualRow.start}px)`,
								}}
							>
								{row.type === "group"
									? (
										<div
											class="member-group"
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
												threadMembers2.cache.get(
													`${thread_id()}:${row.item.user.id}`,
												) ??
													row.item.thread_member;
											const user = () =>
												(users2.cache.get(row.item.user.id) ??
													row.item.user) as User;
											const room_member = props.thread.room_id
												? roomMembers2.cache.get(
													`${room_id() as string}:${row.item.user.id}`,
												)
												: null;
											const isOffline = () =>
												user().presence.status === "Offline";
											// Thread member display - end
											const ctx = useCtx();
											const { userView, setUserView } = useUserPopout();
											const [hovered, setHovered] = createSignal(false);

											function name() {
												let name: string | undefined | null = null;
												const rm = room_member as RoomMember;
												if (rm) {
													name ??= rm.override_name;
												}
												name ??= user().name;
												return name;
											}

											return (
												<div
													class="menu-user"
													data-user-id={row.item.user.id}
													classList={{ offline: isOffline() }}
													onClick={(e) => {
														e.stopPropagation();
														const currentTarget = e
															.currentTarget as HTMLElement;
														if (userView()?.ref === currentTarget) {
															setUserView(null);
														} else {
															setUserView({
																user_id: user().id,
																room_id: room_id() as string | undefined,
																thread_id: thread_id() as string,
																ref: currentTarget,
																source: "member-list",
															});
														}
													}}
													// FIXME: handle keyboard naviatation
													onMouseEnter={() => setHovered(true)}
													onMouseLeave={() =>
														setHovered(false)}
												>
													<AvatarWithStatus user={user()} animate={hovered()} />
													<span class="text">
														<span class="name">{name()}</span>
													</span>
												</div>
											);
										})()
									)}
							</div>
						);
					}}
				</For>
			</div>
		</div>
	);
};
