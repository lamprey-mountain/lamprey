import { batch as solidBatch } from "solid-js";
import { produce, reconcile, SetStoreFunction } from "solid-js/store";
import { Action, ChatCtx, Data } from "../context.ts";
import { RoleT } from "../types.ts";
import { types } from "sdk";
import { Api } from "../api.tsx";

function reduceServer(
	state: Data,
	delta: types.MessageSync,
): Data {
	switch (delta.type) {
		case "UpsertInvite": {
			const { invite } = delta;
			return { ...state, invites: { ...state.invites, [invite.code]: invite } };
		}
		case "UpsertMember": {
			const { member } = delta;
			const { room_id, user_id } = member;
			return {
				...state,
				// TODO: fix this (won't matter if data is normalized?)
				// users: {
				// 	...state.users,
				// 	[user.id]: user,
				// },
				room_members: {
					...state.room_members,
					[room_id]: {
						...state.room_members[room_id],
						[user_id]: member,
					},
				},
			};
		}
		default: {
			console.warn(`unknown event ${delta.type}`, delta);
			return state;
		}
	}
}

export function dispatchServer(
	ctx: ChatCtx,
	update: SetStoreFunction<Data>,
	action: Action,
	_dispatch: (action: Action) => void,
	api: Api,
) {
	switch (action.do) {
		case "server": {
			const msg = action.msg;
			if (msg.type === "UpsertMessage") {
				console.time("UpsertMessage");
				solidBatch(() => {
					const { message } = msg;
					const { id, version_id, thread_id } = message;
					update("messages", id, message);

					const t = api.threads.cache.get(thread_id);
					if (t) {
						api.threads.cache.set(thread_id, {
							...t,
							message_count: t.message_count + (id === version_id ? 1 : 0),
							last_version_id: version_id,
						});
					}
				});
				console.timeEnd("UpsertMessage");
				// TODO: message deletions
			} else if (msg.type === "UpsertRole") {
				const role: RoleT = msg.role;
				const { room_id } = role;
				if (!ctx.data.room_roles[room_id]) update("room_roles", room_id, {});
				update("room_roles", room_id, role.id, role);
				// } else if (msg.type === "DeleteMember") {
				// 	const { user_id, room_id } = msg;
				// 	update(
				// 		"room_members",
				// 		room_id,
				// 		produce((obj) => {
				// 			if (!obj) return;
				// 			delete obj[user_id];
				// 		}),
				// 	);
				// 	if (user_id === ctx.data.user?.id) {
				// 		update(
				// 			"rooms",
				// 			produce((obj) => {
				// 				delete obj[room_id];
				// 			}),
				// 		);
				// 	}
			} else if (msg.type === "DeleteInvite") {
				const { code } = msg;
				update(
					"invites",
					produce((obj) => {
						delete obj[code];
					}),
				);
			} else {
				update(reconcile(reduceServer(ctx.data, action.msg)));
			}
			return;
		}
	}
}
