import { createContext, createEffect, ParentProps, useContext } from "solid-js";
import { useApi } from "../api.tsx";
import type { MemberList } from "../api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import type { User } from "sdk";
import { useLocation } from "@solidjs/router";

const MemberListContext = createContext<ReactiveMap<string, MemberList>>();

export const MemberListProvider = (props: ParentProps) => {
	const api = useApi();
	const location = useLocation();
	const memberLists = new ReactiveMap<string, MemberList>();

	let currentSubscription: string | null = null;
	createEffect(() => {
		const roomIdMatch = location.pathname.match(/\/room\/([^/]+)/);
		if (roomIdMatch) {
			const id = roomIdMatch[1];
			if (currentSubscription !== id) {
				currentSubscription = id;
				api.room_members.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		const channelIdMatch = location.pathname.match(
			/\/(channel|thread)\/([^/]+)/,
		);
		if (channelIdMatch) {
			const id = channelIdMatch[2];
			if (currentSubscription !== id) {
				currentSubscription = id;
				api.thread_members.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		if (currentSubscription !== null) {
			currentSubscription = null;
			api.client.send({ type: "MemberListSubscribe", ranges: [] });
		}
	});

	api.events.on("sync", ([msg]) => {
		if (msg.type === "MemberListSync") {
			const { room_id, channel_id: thread_id, ops, groups } = msg;
			const id = thread_id ?? room_id;
			if (!id) return;

			let list = memberLists.get(id);
			if (!list) {
				list = { groups: [], items: [] };
				memberLists.set(id, list);
			}

			for (const op of ops) {
				if (op.type === "Sync") {
					if (op.users) {
						for (const user of op.users) {
							api.users.upsert(user);
						}
					}
					if (op.room_members && room_id) {
						for (const member of op.room_members) {
							api.room_members.upsert(member);
						}
					}
					if (op.thread_members && thread_id) {
						for (const member of op.thread_members) {
							api.thread_members.upsert(member);
						}
					}

					const items = op.items.map((user_id) => {
						const user = api.users.cache.get(user_id);
						const room_member = room_id
							? api.room_members.cache.get(room_id)?.get(user_id)
							: null;
						const thread_member = thread_id
							? api.thread_members.cache.get(thread_id)?.get(user_id)
							: null;

						return {
							user: user!,
							room_member: room_member ?? null,
							thread_member: thread_member ?? null,
						};
					});
					list.items.splice(Number(op.position), items.length, ...items);
				} else if (op.type === "Insert") {
					const user_id = op.user_id;
					if (op.user) {
						api.users.upsert(op.user);
					}
					if (op.room_member && room_id) {
						api.room_members.upsert(op.room_member);
					}
					if (op.thread_member && thread_id) {
						api.thread_members.upsert(op.thread_member);
					}

					const user = api.users.cache.get(user_id);
					const room_member = room_id
						? api.room_members.cache.get(room_id)?.get(user_id)
						: null;
					const thread_member = thread_id
						? api.thread_members.cache.get(thread_id)?.get(user_id)
						: null;

					const item = {
						user: user!,
						room_member: room_member ?? null,
						thread_member: thread_member ?? null,
					};
					list.items.splice(Number(op.position), 0, item);
				} else if (op.type === "Delete") {
					list.items.splice(Number(op.position), Number(op.count));
				}
			}
			list.groups = groups;
			memberLists.set(id, { ...list });
		} else if (
			msg.type === "RoomMemberCreate" || msg.type === "RoomMemberUpdate"
		) {
			const m = msg.member;
			const list = memberLists.get(m.room_id);
			if (list) {
				const newItems = list.items.map((item) => {
					if (item.user.id === m.user_id) {
						return { ...item, room_member: m };
					}
					return item;
				});
				memberLists.set(m.room_id, { ...list, items: newItems });
			}
		} else if (msg.type === "RoomMemberDelete") {
			const { room_id, user_id } = msg;
			const list = memberLists.get(room_id);
			if (list) {
				const newItems = list.items.filter((item) => item.user.id !== user_id);
				memberLists.set(room_id, { ...list, items: newItems });
			}
		} else if (msg.type === "ThreadMemberUpsert") {
			const { thread_id, added, removed } = msg;
			const list = memberLists.get(thread_id);
			if (list) {
				let newItems = [...list.items];

				// Handle added members
				for (const member of added) {
					const itemIndex = newItems.findIndex((item) =>
						item.user.id === member.user_id
					);
					if (itemIndex !== -1) {
						newItems[itemIndex] = {
							...newItems[itemIndex],
							thread_member: member,
						};
					} else {
						const userItem = newItems.find((item) =>
							item.user.id === member.user_id
						);
						if (userItem) {
							newItems = newItems.map((item) =>
								item.user.id === member.user_id
									? { ...item, thread_member: member }
									: item
							);
						} else {
							// FIXME: no user!?
						}
					}
				}

				for (const userId of removed) {
					newItems = newItems.map((item) =>
						item.user.id === userId ? { ...item, thread_member: null } : item
					);
				}

				memberLists.set(thread_id, { ...list, items: newItems });
			}
		} else if (msg.type === "UserUpdate") {
			for (const [id, list] of memberLists.entries()) {
				let wasUpdated = false;
				const newItems = list.items.map((item) => {
					if (item.user.id === msg.user.id) {
						wasUpdated = true;
						return { ...item, user: msg.user as User };
					}
					return item;
				});

				if (wasUpdated) {
					memberLists.set(id, { ...list, items: newItems });
				}
			}
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			for (const [id, list] of memberLists.entries()) {
				let wasUpdated = false;
				const newItems = list.items.map((item) => {
					if (item.user.id === user_id) {
						wasUpdated = true;
						const updatedUser = { ...item.user, presence };
						return { ...item, user: updatedUser };
					}
					return item;
				});

				if (wasUpdated) {
					memberLists.set(id, { ...list, items: newItems });
				}
			}
		}
	});

	return (
		<MemberListContext.Provider value={memberLists}>
			{props.children}
		</MemberListContext.Provider>
	);
};

export const useMemberList = () => {
	return useContext(MemberListContext)!;
};
