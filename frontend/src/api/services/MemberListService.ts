import { MemberListGroup, RoomMember, ThreadMember, User } from "sdk";
import { RootStore } from "../core/Store";
import { ReactiveMap } from "@solid-primitives/map";
import { batch, createMemo } from "solid-js";
import { createStore, reconcile } from "solid-js/store";

export type MemberListItem = {
	room_member: RoomMember | null;
	thread_member: ThreadMember | null;
	user: User;
};

export type MemberList = {
	groups: MemberListGroup[];
	items: MemberListItem[];
};

export class MemberListService {
	// key is either room_id or thread_id
	lists = new ReactiveMap<string, MemberList>();

	constructor(private store: RootStore) {}

	handleSync(msg: any) {
		if (msg.type === "MemberListSync") {
			const { room_id, channel_id: thread_id, ops, groups } = msg;
			const id = thread_id ?? room_id;
			if (!id) return;

			let list = this.lists.get(id);
			if (!list) {
				list = { groups: [], items: [] };
				this.lists.set(id, list);
			}

            // Copy items to a new array for manipulation
            const newItems = [...list.items];

			for (const op of ops) {
				if (op.type === "Sync") {
					if (op.users) {
						for (const user of op.users) {
							this.store.users.upsert(user);
						}
					}
					if (op.room_members && room_id) {
						for (const member of op.room_members) {
							this.store.roomMembers.upsert(member);
						}
					}
					if (op.thread_members && thread_id) {
						for (const member of op.thread_members) {
							this.store.threadMembers.upsert(member);
						}
					}

					const items = op.items.map((user_id: string) => {
						const user = this.store.users.get(user_id);
						const room_member = room_id
							? this.store.roomMembers.get(`${room_id}:${user_id}`)
							: null;
						const thread_member = thread_id
							? this.store.threadMembers.get(`${thread_id}:${user_id}`)
							: null;

						return {
							user: user!,
							room_member: room_member ?? null,
							thread_member: thread_member ?? null,
						};
					});
					newItems.splice(Number(op.position), items.length, ...items);
				} else if (op.type === "Insert") {
					const user_id = op.user_id;
					if (op.user) {
						this.store.users.upsert(op.user);
					}
					if (op.room_member && room_id) {
						this.store.roomMembers.upsert(op.room_member);
					}
					if (op.thread_member && thread_id) {
						this.store.threadMembers.upsert(op.thread_member);
					}

					const user = this.store.users.get(user_id);
					const room_member = room_id
						? this.store.roomMembers.get(`${room_id}:${user_id}`)
						: null;
					const thread_member = thread_id
						? this.store.threadMembers.get(`${thread_id}:${user_id}`)
						: null;

					const item = {
						user: user!,
						room_member: room_member ?? null,
						thread_member: thread_member ?? null,
					};
					newItems.splice(Number(op.position), 0, item);
				} else if (op.type === "Delete") {
					newItems.splice(Number(op.position), Number(op.count));
				}
			}
			
            this.lists.set(id, { 
                groups: groups, 
                items: newItems 
            });
		}
	}

    // Reactively update lists when entities update
    updateMember(user_id: string, room_id?: string, thread_id?: string) {
        if (room_id) {
            const list = this.lists.get(room_id);
            if (list) {
                const member = this.store.roomMembers.get(`${room_id}:${user_id}`);
                const newItems = list.items.map(item => 
                    item.user.id === user_id ? { ...item, room_member: member ?? null } : item
                );
                this.lists.set(room_id, { ...list, items: newItems });
            }
        }
        if (thread_id) {
            const list = this.lists.get(thread_id);
            if (list) {
                const member = this.store.threadMembers.get(`${thread_id}:${user_id}`);
                const newItems = list.items.map(item => 
                    item.user.id === user_id ? { ...item, thread_member: member ?? null } : item
                );
                this.lists.set(thread_id, { ...list, items: newItems });
            }
        }
    }

    updateUser(user: User) {
        for (const [id, list] of this.lists.entries()) {
            let changed = false;
            const newItems = list.items.map(item => {
                if (item.user.id === user.id) {
                    changed = true;
                    return { ...item, user };
                }
                return item;
            });
            if (changed) {
                this.lists.set(id, { ...list, items: newItems });
            }
        }
    }
}
