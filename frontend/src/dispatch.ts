import { produce, SetStoreFunction } from "solid-js/store";
import { Action, Data } from "./context.ts";
import { InviteT, MemberT, MessageT, RoleT } from "./types.ts";
import { batch as solidBatch } from "solid-js";
import { ChatCtx } from "./context.ts";
import { createEditorState } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";

const SLICE_LEN = 100;
const PAGINATE_LEN = 30;

async function handleSubmit(ctx: ChatCtx, thread_id: string, text: string) {
	if (text.startsWith("/")) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const { room_id } = ctx.data.threads[thread_id];
		if (cmd === "thread") {
			const name = text.slice("/thread ".length);
			await ctx.client.http("POST", `/api/v1/rooms/${room_id}/threads`, {
				name,
			});
		} else if (cmd === "archive") {
			await ctx.client.http("PATCH", `/api/v1/threads/${thread_id}`, {
				is_closed: true,
			});
		} else if (cmd === "unarchive") {
			await ctx.client.http("PATCH", `/api/v1/threads/${thread_id}`, {
				is_closed: false,
			});
		} else if (cmd === "desc") {
			const description = args.join(" ");
			await ctx.client.http("PATCH", `/api/v1/threads/${thread_id}`, {
				description: description || null,
			});
		} else if (cmd === "name") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http("PATCH", `/api/v1/threads/${thread_id}`, {
				name,
			});
		} else if (cmd === "desc-room") {
			const description = args.join(" ");
			await ctx.client.http("PATCH", `/api/v1/rooms/${room_id}`, {
				description: description || null,
			});
		} else if (cmd === "name-room") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http("PATCH", `/api/v1/rooms/${room_id}`, {
				name,
			});
		}
		return;
	}
	ctx.client.http("POST", `/api/v1/threads/${thread_id}/messages`, {
		content: text,
		reply_id: ctx.data.edit_states[thread_id].reply_id,
		nonce: uuidv7(),
	});
	ctx.dispatch({ do: "editor.reply", thread_id, reply_id: null });
	// props.thread.send({ content: text });
	// await new Promise(res => setTimeout(res, 1000));
}

export function createDispatcher(ctx: ChatCtx, update: SetStoreFunction<Data>) {	
  async function dispatch(action: Action) {
  	console.log("dispatch", action.do);
  	switch (action.do) {
  		case "setView": {
  			update("view", action.to);
  			if ("room" in action.to) {
  				const room_id = action.to.room.id;
  				const roomThreadCount = [...Object.values(ctx.data.threads)].filter((i) =>
  					i.room_id === room_id
  				).length;
  				if (roomThreadCount === 0) {
  					const data = await ctx.client.http(
  						"GET",
  						`/api/v1/rooms/${room_id}/threads?dir=f`,
  					);
  					for (const item of data.items) {
  						update("threads", item.id, item);
  					}
  				}
  			}
  			return;
  		}
  		case "paginate": {
  			const { dir, thread_id } = action;
  			const slice = ctx.data.slices[thread_id];
  			console.log("paginate", { dir, thread_id, slice });
  			if (!slice) {
  				const batch = await ctx.client.http(
  					"GET",
  					`/api/v1/threads/${thread_id}/messages?dir=b&from=ffffffff-ffff-ffff-ffff-ffffffffffff&limit=100`,
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
  						`/api/v1/threads/${thread_id}/messages?dir=b&limit=100&from=${from}`,
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
  						`/api/v1/threads/${thread_id}/messages?dir=f&limit=100&from=${from}`,
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
  		case "editor.init": {
  		  if (ctx.data.edit_states[action.thread_id]) return;
  		  update("edit_states", action.thread_id, {
    		  state: createEditorState(text => handleSubmit(ctx, action.thread_id, text)),
    		  reply_id: null,
  		  });
  		  return;
  		}
  		case "editor.reply": {
  		  update("edit_states", action.thread_id, "reply_id", action.reply_id);
  		  return;
  		}
  	}
  }

  return dispatch;
}

export function createWebsocketHandler(ws: WebSocket, ctx: ChatCtx, update: SetStoreFunction<Data>) {	
  return function(msg: any) {
		console.log("recv", msg);
		if (msg.type === "ping") {
			ws.send(JSON.stringify({ type: "pong" }));
		} else if (msg.type === "ready") {
			update("user", msg.user);
		} else if (msg.type === "upsert.room") {
			update("rooms", msg.room.id, msg.room);
		} else if (msg.type === "upsert.thread") {
			update("threads", msg.thread.id, msg.thread);
		} else if (msg.type === "upsert.message") {
			solidBatch(() => {
				update("messages", msg.message.id, msg.message);
				const { thread_id } = msg.message;
				if (!ctx.data.timelines[thread_id]) {
					update("timelines", thread_id, [{ type: "hole" }, {
						type: "remote",
						message: msg.message as MessageT,
					}]);
					update("slices", thread_id, { start: 0, end: 2 });
				} else {
					update(
						"timelines",
						msg.message.thread_id,
						(i) => [...i, { type: "remote" as const, message: msg.message }],
					);
					if (
						ctx.data.slices[thread_id].end === ctx.data.timelines[thread_id].length - 1
					) {
						const newEnd = ctx.data.timelines[thread_id].length;
						const newStart = Math.max(newEnd - PAGINATE_LEN, 0);
						update("slices", thread_id, { start: newStart, end: newEnd });
					}
				}
			});
		} else if (msg.type === "upsert.role") {
			const role: RoleT = msg.role;
			const { room_id } = role;
			if (!ctx.data.room_roles[room_id]) update("room_roles", room_id, {});
			update("room_roles", room_id, role.id, role);
		} else if (msg.type === "upsert.member") {
			const member: MemberT = msg.member;
			const { room_id } = member;
			if (!ctx.data.room_members[room_id]) update("room_members", room_id, {});
			update("users", member.user.id, member.user);
			update("room_members", room_id, member.user.id, member);
		} else if (msg.type === "upsert.invite") {
			const invite: InviteT = msg.invite;
			update("invites", invite.code, invite);
		} else if (msg.type === "delete.member") {
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
		} else if (msg.type === "delete.invite") {
			const { code } = msg
			update("invites", produce((obj) => {
				delete obj[code];
			}));
		} else {
			console.warn("unknown message", msg);
			return;
		}
  }
}
