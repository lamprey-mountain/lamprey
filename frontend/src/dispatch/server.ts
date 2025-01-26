import { batch as solidBatch } from "solid-js";
import { produce, reconcile, SetStoreFunction } from "solid-js/store";
import { Action, ChatCtx, Data } from "../context.ts";
import { InviteT, MemberT, RoleT } from "../types.ts";
import { types } from "sdk";

function reduceServer(
	state: Data,
	delta: types.MessageSync,
): Data {
	switch (delta.type) {
		case "UpsertSession":
			{
				const { session } = delta;
				if (session.id === state.session?.id) {
					return { ...state, session };
				} else {
					return state;
				}
				// if (!ctx.data.user) {
				// 	ctx.client.http.GET("/api/v1/user/{user_id}", {
				// 		params: {
				// 			path: {
				// 				user_id: "@self",
				// 			},
				// 		},
				// 	}).then((res) => {
				// 		const user = res.data;
				// 		if (!user) {
				// 			throw new Error("couldn't fetch user");
				// 		}
				// 		update("user", user);
				// 		update("users", user.id, user);
				// 	});
				// 	ctx.dispatch({ do: "init" });
				// }
			}
		case "UpsertRoom": {
			const { room } = delta;
			return { ...state, rooms: { ...state.rooms, [room.id]: room } };
		}
		case "UpsertThread": {
			const { thread } = delta;
			return { ...state, threads: { ...state.threads, [thread.id]: thread } };
		}
		case "UpsertUser": {
			const { user } = delta;
			return {
				...state,
				users: {
					...state.users,
					[user.id]: user,
				},
				user: user.id === state.user?.id ? user : state.user,
			};
		}
		case "UpsertInvite": {
			const { invite } = delta;
			return { ...state, invites: { ...state.invites, [invite.code]: invite } };
		}
		case "UpsertMember": {
			const { member } = delta;
			const { room_id, user } = member;
			return {
				...state,
				users: {
					...state.users,
					[user.id]: user,
				},
				room_members: {
					...state.room_members,
					[room_id]: {
						...state.room_members[room_id],
						[user.id]: member,
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
	dispatch: (action: Action) => Promise<void>,
) {
	switch (action.do) {
		case "server": {
			const msg = action.msg;
			if (msg.type === "UpsertMessage") {
				console.time("UpsertMessage");
				solidBatch(() => {
					const { message } = msg;
					const { id, version_id, thread_id, nonce } = message;
					update("messages", id, message);

					if (ctx.data.threads[thread_id]) {
						update("threads", thread_id, "last_version_id", version_id);
						if (id === version_id) {
							update("threads", thread_id, "message_count", (i) => i + 1);
						}
					}

					if (!ctx.data.timelines[thread_id]) {
						update("timelines", thread_id, [{ type: "hole" }, {
							type: "remote",
							message,
						}]);
					} else {
						const tl = ctx.data.timelines[thread_id];
						const item = { type: "remote" as const, message };
						if (id === version_id) {
							const idx = tl.findIndex((i) =>
								i.type === "local" && i.message.nonce === nonce
							);
							if (idx === -1) {
								update(
									"timelines",
									message.thread_id,
									(i) => [...i, item],
								);
							} else {
								update("timelines", message.thread_id, idx, item);
							}
						} else {
							update(
								"timelines",
								message.thread_id,
								(i) =>
									i.map((j) =>
										(j.type === "remote" && j.message.id === id) ? item : j
									),
							);
						}
					}

					dispatch({ do: "thread.autoscroll", thread_id });
				});
				console.timeEnd("UpsertMessage");
				// TODO: message deletions
			} else if (msg.type === "UpsertRole") {
				const role: RoleT = msg.role;
				const { room_id } = role;
				if (!ctx.data.room_roles[room_id]) update("room_roles", room_id, {});
				update("room_roles", room_id, role.id, role);
			} else if (msg.type === "DeleteMember") {
				const { user_id, room_id } = msg;
				update(
					"room_members",
					room_id,
					produce((obj) => {
						if (!obj) return;
						delete obj[user_id];
					}),
				);
				if (user_id === ctx.data.user?.id) {
					update(
						"rooms",
						produce((obj) => {
							delete obj[room_id];
						}),
					);
				}
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
