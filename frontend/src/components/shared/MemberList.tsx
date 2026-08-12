import { ReactiveMap } from "@solid-primitives/map";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { MemberListGroup, RoomMember, User } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	from,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import type { MemberListItem } from "@/api/services/MemberListService";
import { AvatarWithStatus } from "@/components/shared/User";
import { useUserPopout } from "@/contexts/mod";
import { logger } from "@/utils/logger";
import { MemberListSkeleton } from "./MemberListSkeleton";

const memberListLog = logger.for("member_list");

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

type Row =
	| { type: "group"; group: MemberListGroup }
	| { type: "member"; item: MemberListItem };

export const MemberList = (props: MemberListProps) => {
	const api = useApi();
	const clientState = from(api.client.state);

	let lastSub: string | null = null;
	createEffect(() => {
		if (clientState() !== "ready") return;

		const id = props.type === "room" ? props.roomId : props.threadId;
		const ranges: [number, number][] = [[0, 199]];

		// TODO: theres probably a better way to do this
		const subKey = `${props.type}:${id}:${JSON.stringify(ranges)}`;

		if (lastSub === subKey) return;

		if (props.type === "room") {
			memberListLog.info("subscribing to room member list", {
				room_id: id,
				ranges,
			});
			api.roomMembers.subscribeList(id, ranges);
		} else {
			const channel = api.channels.cache.get(id);
			if (channel && channel.type !== "Dm" && channel.type !== "Gdm") {
				memberListLog.info("subscribing to thread member list", {
					thread_id: id,
					ranges,
				});
				api.threadMembers.subscribeList(id, ranges);
			}
		}
		lastSub = subKey;
	});

	const list = () => api.memberLists.lists.get(props.id);
	const [collapsedGroups, setCollapsedGroups] = createSignal(
		new ReactiveMap<string, boolean>(),
	);

	const rows = createMemo(() => {
		if (props.type === "thread") {
			const channel = api.channels.cache.get(props.threadId);
			if (channel) {
				const isDm = channel.type === "Dm" || channel.type === "Gdm";
				if (isDm && channel.recipients) {
					const onlineItems: MemberListItem[] = [];
					const offlineItems: MemberListItem[] = [];
					for (const u of channel.recipients) {
						const user = api.users.cache.get(u.id) ?? u;
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
			const groupId = group.id as string;
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
			const role = api.roles.cache.get(group.id);
			return role?.name ?? group.id;
		}
		// Handle role-based group id
		const roleId = Object.values(group.id)[0];
		const role = api.roles.cache.get(roleId);
		return role?.name ?? roleId;
	};

	let parentRef!: HTMLDivElement;

	const virt = createVirtualizer({
		get count() {
			return rows().length;
		},
		getScrollElement: () => parentRef,
		estimateSize: (i) => {
			const row = rows()[i];
			return row.type === "group" ? 28 : 44;
		},
		overscan: 10,
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

	const isLoading = () => !list();

	return (
		<div
			ref={parentRef}
			class="member-list"
			classList={{ skeleton: isLoading() }}
			data-room-id={props.id}
		>
			<div
				style={{
					height: `${virt.getTotalSize()}px`,
					width: "100%",
					position: "relative",
				}}
			>
				<Show when={!isLoading()} fallback={<MemberListSkeleton />}>
					<For each={virt.getVirtualItems()}>
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
									ref={(el) => queueMicrotask(() => virt.measureElement(el))}
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
													api.users.cache.get(item().user.id) ?? item().user;
												const room_member = () =>
													props.roomId
														? (api.roomMembers.cache.get(
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
				</Show>
			</div>
		</div>
	);
};
