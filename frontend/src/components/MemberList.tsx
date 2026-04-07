import { ReactiveMap } from "@solid-primitives/map";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { MemberListGroup, RoomMember, User } from "sdk";
import { createMemo, createSignal, For } from "solid-js";
import {
	useRoles2,
	useRoomMembers2,
	useThreadMembers2,
	useUsers2,
} from "@/api";
import type { MemberListItem } from "@/api/services/MemberListService";
import { useMemberList } from "@/contexts/memberlist.tsx";
import { useUserPopout } from "@/contexts/mod.tsx";
import { AvatarWithStatus } from "@/User.tsx";

type MemberListProps =
	| {
			type: "room";
			id: string;
			roomId: string;
			threadId?: undefined;
	  }
	| {
			type: "thread";
			id: string;
			roomId?: string | null;
			threadId: string;
	  };

export const MemberList = (props: MemberListProps) => {
	const roles2 = useRoles2();
	const roomMembers2 = useRoomMembers2();
	const _threadMembers2 = useThreadMembers2();
	const users2 = useUsers2();
	const memberLists = useMemberList();
	const list = () => memberLists.get(props.id);
	const [collapsedGroups, setCollapsedGroups] = createSignal(
		new ReactiveMap<string, boolean>(),
	);

	type Row =
		| { type: "group"; group: MemberListGroup }
		| { type: "member"; item: MemberListItem };

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

	const getGroupName = (group: MemberListGroup) => {
		if (typeof group.id === "string") {
			const role = roles2.cache.get(group.id);
			return role?.name ?? group.id;
		}
		// Handle role-based group id
		const roleId = Object.values(group.id)[0];
		const role = roles2.cache.get(roleId);
		return role?.name ?? roleId;
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

	const { userView, setUserView } = useUserPopout();

	const handleUserClick = (e: MouseEvent, user: User) => {
		e.stopPropagation();
		const currentTarget = e.currentTarget as HTMLElement;
		if (userView()?.ref === currentTarget) {
			setUserView(null);
		} else {
			setUserView({
				user_id: user.id,
				room_id: props.roomId ?? undefined,
				thread_id: props.threadId,
				ref: currentTarget,
				source: "member-list",
			});
		}
	};

	const handleUserKeyDown = (
		e: KeyboardEvent,
		user: User,
		_room_member: RoomMember | null | undefined,
	) => {
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			e.stopPropagation();
			const currentTarget = e.currentTarget as HTMLElement;
			if (userView()?.ref === currentTarget) {
				setUserView(null);
			} else {
				setUserView({
					user_id: user.id,
					room_id: props.roomId ?? undefined,
					thread_id: props.threadId,
					ref: currentTarget,
					source: "member-list",
				});
			}
		}
	};

	const toggleGroup = (group: MemberListGroup) => {
		const groupId = JSON.stringify(group.id);
		const newMap = new ReactiveMap(collapsedGroups());
		newMap.set(groupId, !newMap.get(groupId));
		setCollapsedGroups(newMap);
	};

	const handleGroupKeyDown = (e: KeyboardEvent, group: MemberListGroup) => {
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			toggleGroup(group);
		}
	};

	return (
		<div ref={parentRef} class="member-list" data-room-id={props.id}>
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
								{row.type === "group" ? (
									<button
										type="button"
										class="member-group"
										onClick={() => toggleGroup(row.group)}
										onKeyDown={(e) => handleGroupKeyDown(e, row.group)}
									>
										{getGroupName(row.group)} — {row.group.count}
									</button>
								) : (
									(() => {
										const item = row.item;
										const user = () =>
											users2.cache.get(item.user.id) ?? item.user;
										const room_member = () =>
											props.roomId
												? (roomMembers2.cache.get(
														`${props.roomId}:${item.user.id}`,
													) ?? item.room_member)
												: item.room_member;
										const isOffline = () =>
											user()?.presence.status === "Offline";

										const [hovered, setHovered] = createSignal(false);

										function name() {
											let name: string | undefined | null = null;
											const rm = room_member();
											if (rm) {
												name ??= rm.override_name;
											}
											name ??= user()?.name;
											return name;
										}

										return (
											<button
												type="button"
												class="menu-user"
												data-user-id={item.user.id}
												classList={{ offline: isOffline() }}
												onClick={(e) => handleUserClick(e, user())}
												onKeyDown={(e) =>
													handleUserKeyDown(e, user(), room_member())
												}
												onMouseEnter={() => setHovered(true)}
												onMouseLeave={() => setHovered(false)}
											>
												<AvatarWithStatus user={user()} animate={hovered()} />
												<span class="text">
													<span class="name">{name()}</span>
												</span>
											</button>
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
