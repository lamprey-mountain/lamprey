import { produce, SetStoreFunction } from "solid-js/store";
import { Action, Data } from "./context.ts";
import { InviteT, MemberT, MessageT, MessageType, RoleT } from "./types.ts";
import { batch as solidBatch } from "solid-js";
import { ChatCtx } from "./context.ts";
import { createEditorState } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";

const SLICE_LEN = 100;
const PAGINATE_LEN = 30;

export function createDispatcher(ctx: ChatCtx, update: SetStoreFunction<Data>) {
	let ackGraceTimeout: number | undefined;
	let ackDebounceTimeout: number | undefined;
	
  async function dispatch(action: Action) {
  	console.log("dispatch", action.do);
  	switch (action.do) {
  		case "setView": {
  			if ("room" in action.to) {
  				const room_id = action.to.room.id;
  				const roomThreadCount = [...Object.values(ctx.data.threads)].filter((i) =>
  					i.room_id === room_id
  				).length;
  				if (roomThreadCount === 0) {
  					const data = await ctx.client.http(
  						"GET",
  						`/api/v1/room/${room_id}/thread?dir=f`,
  					);
  					for (const item of data.items) {
  						update("threads", item.id, item);
  					}
  				}
  			}
  			if (action.to.view === "thread") {
  				const thread_id = action.to.thread.id;
  				dispatch({ do: "thread.init", thread_id });
					update("thread_state", thread_id, "read_marker_id", ctx.data.threads[thread_id].last_read_id);
  			}
  			ackDebounceTimeout = undefined; // make sure threads past the grace period get marked as read
  			update("view", action.to);
  			return;
  		}
  		case "paginate": {
  			const { dir, thread_id } = action;
  			const slice = ctx.data.slices[thread_id];
  			console.log("paginate", { dir, thread_id, slice });
  			if (!slice) {
  				const batch = await ctx.client.http(
  					"GET",
  					`/api/v1/thread/${thread_id}/message?dir=b&from=ffffffff-ffff-ffff-ffff-ffffffffffff&limit=100`,
  				);
  				const tl = batch.items.map((i: MessageT) => ({
  					type: "remote" as const,
  					message: i,
  				}));
  				if (batch.has_more) tl.unshift({ type: "hole" });
  				solidBatch(() => {
  					update("timelines", thread_id, tl);
  					update("slices", thread_id, { start: 0, end: tl.length });
  					for (const msg of batch.items) {
  						update("messages", msg.id, msg);
  					}
			    	ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
  				});
  				return;
  			}

  			const tl = ctx.data.timelines[thread_id];
  			// console.log({ tl, slice })
  			if (tl.length < 2) return; // needs startitem and nextitem
  			if (dir === "b") {
  				const startItem = tl[slice.start];
  				const nextItem = tl[slice.start + 1];
  				if (startItem?.type === "hole") {
  					const from = nextItem.type === "remote" ? nextItem.message.id :
  						"ffffffff-ffff-ffff-ffff-ffffffffffff";
  					const batch = await ctx.client.http(
  						"GET",
  						`/api/v1/thread/${thread_id}/message?dir=b&limit=100&from=${from}`,
  					);
  					solidBatch(() => {
  						update("timelines", thread_id, (i) => [
  							...batch.has_more ? [{ type: "hole" }] : [],
  							...batch.items.map((j: MessageT) => ({
  								type: "remote",
  								message: j,
  							})),
  							...i.slice(slice.start + 1),
  						]);
  						for (const msg of batch.items) {
  							update("messages", msg.id, msg);
  						}
  					});
  				}

  				const newTl = ctx.data.timelines[thread_id];
  				const newOff = newTl.indexOf(nextItem) - slice.start;
  				const newStart = Math.max(slice.start + newOff - PAGINATE_LEN, 0);
  				const newEnd = Math.min(newStart + SLICE_LEN, newTl.length);
  				console.log({ start: newStart, end: newEnd });
  				update("slices", thread_id, { start: newStart, end: newEnd });
  			} else {
    			console.log(slice.start, slice.end, [...tl]);
  				const startItem = tl[slice.end - 1];
  				const nextItem = tl[slice.end - 2];
  				if (startItem.type === "hole") {
  					const from = nextItem.type === "remote" ? nextItem.message.id :
  						"00000000-0000-0000-0000-000000000000";
  					const batch = await ctx.client.http(
  						"GET",
  						`/api/v1/thread/${thread_id}/message?dir=f&limit=100&from=${from}`,
  					);
  					solidBatch(() => {
  						update("timelines", thread_id, (i) => [
  							...i.slice(0, slice.end - 1),
  							...batch.items.map((j: MessageT) => ({
  								type: "remote",
  								message: j,
  							})),
  							...batch.has_more ? [{ type: "hole" }] : [],
  						]);
  						for (const msg of batch.items) {
  							update("messages", msg.id, msg);
  						}
  					});
  				}

  				const newTl = ctx.data.timelines[thread_id];
  				const newOff = newTl.indexOf(nextItem) - slice.end - 1;
  				const newEnd = Math.min(
  					slice.end + newOff + PAGINATE_LEN,
  					newTl.length,
  				);
  				const newStart = Math.max(newEnd - SLICE_LEN, 0);
  				console.log({ start: newStart, end: newEnd });
  				update("slices", thread_id, { start: newStart, end: newEnd });
  			}
  			return;
  		}
  		case "menu": {
  			if (action.menu) console.log("handle menu", action.menu);
  			update("menu", action.menu);
  			return;
  		}
  		// case "modal.open": {
  		// 	updateData("modals", i => [action.modal, ...i ?? []]);
  		// 	return;
  		// }
  		case "modal.close": {
  			update("modals", i => i.slice(1));
  			return;
  		}
  		case "modal.alert": {
  			update("modals", i => [{ type: "alert", text: action.text }, ...i ?? []]);
  			return;
  		}
  		case "modal.confirm": {
  			const p = Promise.withResolvers();
  			const modal = {
  				type: "confirm",
  				text: action.text,
  				cont: p.resolve,
  			};
  			update("modals", i => [modal, ...i ?? []]);
  			return p.promise;
  		}
  		case "modal.prompt": {
  			const p = Promise.withResolvers();
  			const modal = {
  				type: "prompt",
  				text: action.text,
  				cont: p.resolve,
  			};
  			update("modals", i => [modal, ...i ?? []]);
  			return p.promise;
  		}
  		case "thread.init": {
  		  if (ctx.data.thread_state[action.thread_id]) return;
  		  update("thread_state", action.thread_id, {
    		  state: createEditorState(text => handleSubmit(ctx, action.thread_id, text, update)),
    		  reply_id: null,
    		  scroll_pos: null,
					read_marker_id: null,
					attachments: [],
  		  });
  		  return;
  		}
  		case "thread.reply": {
  		  update("thread_state", action.thread_id, "reply_id", action.reply_id);
  		  return;
  		}
  		case "thread.scroll_pos": {
  		  update("thread_state", action.thread_id, "scroll_pos", action.pos);
  		  return;
  		}
  		case "server": {
  			const msg = action.msg;
				if (msg.type === "Ready") {
					update("user", msg.user);
				} else if (msg.type === "UpsertRoom") {
					update("rooms", msg.room.id, msg.room);
				} else if (msg.type === "UpsertThread") {
					update("threads", msg.thread.id, msg.thread);
				} else if (msg.type === "UpsertMessage") {
					solidBatch(() => {
						const { message } = msg;
						const { id, version_id, thread_id, nonce } = message;
						update("messages", id, message);
						if (ctx.data.threads[thread_id]) {
							update("threads", thread_id, "last_version_id", version_id);
							if (id === version_id) {
								update("threads", thread_id, "message_count", i => i + 1);
							}
						}
						if (!ctx.data.timelines[thread_id]) {
							update("timelines", thread_id, [{ type: "hole" }, {
								type: "remote",
								message
							}]);
							update("slices", thread_id, { start: 0, end: 2 });
						} else {
							const tl = ctx.data.timelines[thread_id];
							const isAtEnd = ctx.data.slices[thread_id].end === tl.length;
							if (id === version_id) {
								const idx = tl.findIndex(i => i.type === "local" && i.message.nonce === nonce);
								console.log({ idx })
								if (idx === -1) {
									update(
										"timelines",
										message.thread_id,
										(i) => [...i, { type: "remote" as const, message }],
									);
								} else {
									update(
										"timelines",
										message.thread_id,
										(i) => [...i.slice(0, idx), { type: "remote" as const, message }, ...i.slice(idx + 1)],
									);
								}
							} else {
								update(
									"timelines",
									message.thread_id,
									(i) => i.map(j => (j.type === "remote" && j.message.id === id) ? { type: "remote" as const, message } : j),
								);
							}
							if (isAtEnd) {
								const newEnd = ctx.data.timelines[thread_id].length;
								const newStart = Math.max(newEnd - PAGINATE_LEN, 0);
								update("slices", thread_id, { start: newStart, end: newEnd });
							}
						}
						if (ctx.data.view.view === "thread" && ctx.data.view.thread.id === thread_id) {
							const tl = ctx.data.timelines[thread_id];
							const isAtEnd = tl?.at(-1)?.type !== "hole" && ctx.data.slices[thread_id].end >= tl.length;
							if (isAtEnd) {
					    	ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
							}
						}
					});
					// TODO: message deletions
				} else if (msg.type === "UpsertRole") {
					const role: RoleT = msg.role;
					const { room_id } = role;
					if (!ctx.data.room_roles[room_id]) update("room_roles", room_id, {});
					update("room_roles", room_id, role.id, role);
				} else if (msg.type === "UpsertMember") {
					const member: MemberT = msg.member;
					const { room_id } = member;
					if (!ctx.data.room_members[room_id]) update("room_members", room_id, {});
					update("users", member.user.id, member.user);
					update("room_members", room_id, member.user.id, member);
				} else if (msg.type === "UpsertInvite") {
					const invite: InviteT = msg.invite;
					update("invites", invite.code, invite);
				} else if (msg.type === "DeleteMember") {
					const { user_id, room_id } = msg
					update("room_members", room_id, produce((obj) => {
						if (!obj) return;
						delete obj[user_id];
					}));
					if (user_id === ctx.data.user?.id) {
						update("rooms", produce((obj) => {
							delete obj[room_id];
						}));
					}
				} else if (msg.type === "DeleteInvite") {
					const { code } = msg
					update("invites", produce((obj) => {
						delete obj[code];
					}));
				} else {
					console.warn("unknown message", msg);
				}
  		  return;
  		}
			case "thread.mark_read": {
				const { thread_id, delay, also_local } = action;
				// NOTE: may need separate timeouts per thread
				clearTimeout(ackGraceTimeout);
				clearTimeout(ackDebounceTimeout);
				if (delay) {
			    ackGraceTimeout = setTimeout(() => {
				    ackDebounceTimeout = setTimeout(() => {
				    	ctx.dispatch({ ...action, delay: false });
				    }, 800);
			    }, 200);
			    return;
				}
		    
				const version_id = action.version_id ?? ctx.data.threads[thread_id].last_version_id;
				await ctx.client.http("PUT", `/api/v1/thread/${thread_id}/ack`, { version_id });
				update("threads", thread_id, "last_read_id", version_id);
				const has_thread = !!ctx.data.thread_state[action.thread_id];
				if (also_local && has_thread) update("thread_state", action.thread_id, "read_marker_id", version_id);
  		  return;
			}
  	}
  }

  return dispatch;
}

export function createWebsocketHandler(ws: WebSocket, ctx: ChatCtx) {	
  return function(msg: any) {
		console.log("recv", msg);
		if (msg.type === "Ping") {
			ws.send(JSON.stringify({ type: "Pong" }));
		} else {
			ctx.dispatch({
				do: "server",
				msg,
			});
		}
  }
}

async function handleSubmit(ctx: ChatCtx, thread_id: string, text: string, update: SetStoreFunction<Data>) {
	if (text.startsWith("/")) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const { room_id } = ctx.data.threads[thread_id];
		if (cmd === "thread") {
			const name = text.slice("/thread ".length);
			await ctx.client.http("POST", `/api/v1/room/${room_id}/thread`, {
				name,
			});
		} else if (cmd === "archive") {
			await ctx.client.http("PATCH", `/api/v1/thread/${thread_id}`, {
				is_closed: true,
			});
		} else if (cmd === "unarchive") {
			await ctx.client.http("PATCH", `/api/v1/thread/${thread_id}`, {
				is_closed: false,
			});
		} else if (cmd === "desc") {
			const description = args.join(" ");
			await ctx.client.http("PATCH", `/api/v1/thread/${thread_id}`, {
				description: description || null,
			});
		} else if (cmd === "name") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http("PATCH", `/api/v1/thread/${thread_id}`, {
				name,
			});
		} else if (cmd === "desc-room") {
			const description = args.join(" ");
			await ctx.client.http("PATCH", `/api/v1/room/${room_id}`, {
				description: description || null,
			});
		} else if (cmd === "name-room") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http("PATCH", `/api/v1/room/${room_id}`, {
				name,
			});
		}
		return;
	}
	const reply_id = ctx.data.thread_state[thread_id].reply_id;
	const nonce = uuidv7();
	ctx.client.http("POST", `/api/v1/thread/${thread_id}/message`, {
		content: text,
		reply_id,
		nonce,
	});
	const localMessage: MessageT = {
		type: MessageType.Default,
		id: nonce,
		thread_id,
		version_id: nonce,
		override_name: null,
		reply_id,
		nonce,
		content: text,
		author: ctx.data.user!,
		metadata: null,
		attachments: [],
	};
	solidBatch(() => {
		const slice = ctx.data.slices[thread_id];
		update(
			"timelines",
			thread_id,
			(i) => [...i, { type: "local" as const, message: localMessage }],
		);
		update("slices", thread_id, { start: slice.start + 1, end: slice.end + 1 });
		// for (const msg of batch.items) {
		// 	update("messages", msg.id, msg);
		// }
	});
	ctx.dispatch({ do: "thread.reply", thread_id, reply_id: null });
	// props.thread.send({ content: text });
	// await new Promise(res => setTimeout(res, 1000));
}
