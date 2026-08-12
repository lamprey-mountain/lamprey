import { ReactiveMap } from "@solid-primitives/map";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { MemberListGroup, RoomMember, User } from "sdk";
import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { useChannels, useRoles, useRoomMembers, useUsers } from "@/api";
import type { MemberListItem } from "@/api/services/MemberListService";
import { AvatarWithStatus } from "@/components/shared/User";
import { useMemberListContext } from "@/contexts/memberlist.tsx";
import { useUserPopout } from "@/contexts/mod";
import { MemberListSkeleton } from "./MemberListSkeleton";

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
	const roles2 = useRoles();
	const roomMembers2 = useRoomMembers();
	const users2 = useUsers();
	const channels2 = useChannels();
	const memberLists = useMemberListContext();
	const list = () => memberLists.get(props.id);
	const [collapsedGroups, setCollapsedGroups] = createSignal(
		new ReactiveMap<string, boolean>(),
	);

	type Row =
		| { type: "group"; group: MemberListGroup }
		| { type: "member"; item: MemberListItem };

	const rows = createMemo(() => {
		if (props.type === "thread") {
			const channel = channels2.cache.get(props.threadId);
			if (channel) {
				const isDm = channel.type === "Dm" || channel.type === "Gdm";
				if (isDm && channel.recipients) {
					const onlineItems: MemberListItem[] = [];
					const offlineItems: MemberListItem[] = [];
					for (const u of channel.recipients) {
						const user = users2.cache.get(u.id) ?? u;
						const item: MemberListItem = {
							user,
							room_member: null,
							thread_member: null,
						};
						if (user.presence.status === "Offline") {
							offlineItems.push(item);
						} else {
							onlineItems.push(item);
						}
					}

					const rows: Row[] = [];
					if (onlineItems.length > 0) {
						rows.push({
							type: "group",
							group: { id: "Online", count: onlineItems.length },
						});
						for (const item of onlineItems) rows.push({ type: "member", item });
					}
					if (offlineItems.length > 0) {
						rows.push({
							type: "group",
							group: { id: "Offline", count: offlineItems.length },
						});
						for (const item of offlineItems)
							rows.push({ type: "member", item });
					}
					return rows;
				}
			}
		}

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
		<Show when={list()} fallback={<MemberListSkeleton />}>
			{(l) => (
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
								const row = () => rows()[virtualRow.index];

								const matchesGroup = () => {
									const r = row();
									if (r?.type === "group") return r.group;
								};

								const matchesMember = () => {
									const r = row();
									if (r?.type === "member") return r.item;
								};

								// FIXME: measure element when row changes

								return (
									<div
										style={{
											position: "absolute",
											top: 0,
											left: 0,
											width: "100%",
											transform: `translateY(${virtualRow.start}px)`,
										}}
										ref={rowVirtualizer.measureElement}
										data-index={virtualRow.index}
									>
										<Switch>
											<Match when={matchesGroup()}>
												{(group) => (
													<button
														type="button"
														class="member-group"
														onClick={() => toggleGroup(group())}
														onKeyDown={(e) => handleGroupKeyDown(e, group())}
													>
														{getGroupName(group())} — {group().count}
													</button>
												)}
											</Match>
											<Match when={matchesMember()}>
												{(item) => {
													const user = () =>
														users2.cache.get(item().user.id) ?? item().user;
													const room_member = () =>
														props.roomId
															? (roomMembers2.cache.get(
																	`${props.roomId}:${item().user.id}`,
																) ?? item().room_member)
															: item().room_member;
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

													// TODO: apply .active after clicking a user, while the user popout is open
													// probably will only apply it when the user popout is opened from the member list

													return (
														<button
															type="button"
															class="menu-user"
															data-user-id={item().user.id}
															classList={{
																active: false,
																offline: isOffline(),
															}}
															onClick={(e) => handleUserClick(e, user())}
															onKeyDown={(e) =>
																handleUserKeyDown(e, user(), room_member())
															}
															onMouseEnter={[setHovered, true]}
															onMouseLeave={[setHovered, false]}
														>
															<div class="inner">
																<AvatarWithStatus
																	user={user()}
																	animate={hovered()}
																/>
																<span class="text">
																	<div class="name">{name()}</div>
																	<Show
																		when={
																			user()?.presence.activities.find(
																				(a) => a.type === "Custom",
																			)?.text
																		}
																	>
																		{(t) => (
																			<div class="status-message">{t()}</div>
																		)}
																	</Show>
																</span>
															</div>
														</button>
													);
												}}
											</Match>
										</Switch>
									</div>
								);
							}}
						</For>
					</div>
				</div>
			)}
		</Show>
	);
};
