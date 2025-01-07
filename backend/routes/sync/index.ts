import { OpenAPIHono, z } from "@hono/zod-openapi";
import { data, events, HonoEnv } from "globals";
import { SyncInit } from "./def.ts";
import { upgradeWebSocket } from "npm:hono/deno";
import { MessageClient, MessageServer } from "../../types/sync.ts";
import { uuidv7 } from "uuidv7";
import { Permissions } from "globals";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(SyncInit, async (c, next) => {
		const id = uuidv7();
		let ws: WebSocket;
		let state: "closed" | "unauth" | "auth" = "closed";
		let heartbeatTimeout: number;
		let closeTimeout: number;
		let user_id: string;

		function send(msg: z.infer<typeof MessageServer>) {
			if (state === "closed") {
				throw new Error("tried to send message to closed websocket");
			}
			console.log(`send websocket ${id} ${msg.type}`);
			ws.send(JSON.stringify(msg));
		}

		function rescheduleHeartbeat() {
			clearTimeout(heartbeatTimeout);
			clearTimeout(closeTimeout);
			heartbeatTimeout = setTimeout(() => {
				if (state === "closed") return;
				ws.send(JSON.stringify({ type: "ping" }));
			}, 1000 * 30);
			closeTimeout = setTimeout(() => {
				if (state === "closed") return;
				ws.close();
			}, 1000 * 45);
		}

		async function handle(msg: z.infer<typeof MessageServer>) {
			if (state === "closed") return;
			// if (state === "unauth") return;
			ws.send(JSON.stringify(msg));
		}

		const permCacheRoom = new Map<string, Permissions>();
		const permCacheThread = new Map<string, Permissions>();

		type Msg = z.infer<typeof MessageServer>;
		
		const poisonsCacheRoom = (m: Msg) => {
			if (m.type === "upsert.invite") return false;
			if (m.type === "upsert.room") return false;
			if (m.type === "upsert.thread") return false; // may need to remove if private threads are impl'd?
			if (m.type === "upsert.member" && m.member.user.id !== user_id) return false; // not sure how correct this is
			return true;
		};
		
		const poisonsCacheThread = (m: Msg) => {
			if (m.type === "upsert.message") return false;
			if (m.type === "delete.message") return false;
			if (m.type === "delete.message_version") return false;
			return true;
		};
		
		async function handleRoom(room_id: string, msg: Msg) {
			console.log(state, msg)
			if (state === "closed") return;
			if (msg.type === "delete.member" && msg.user_id === user_id) {
				ws.send(JSON.stringify(msg));
				return;
			}
			if (poisonsCacheRoom(msg)) permCacheRoom.delete(room_id);
			const perms = permCacheRoom.get(room_id) ?? await data.permissionReadRoom(user_id, room_id);
			permCacheRoom.set(room_id, perms);
			if (!perms.has("View")) return;
			ws.send(JSON.stringify(msg));
		}
		
		async function handleThread(thread_id: string, msg: z.infer<typeof MessageServer>) {
			if (state === "closed") return;
			if (poisonsCacheThread(msg)) permCacheRoom.delete(thread_id);
			const perms = permCacheThread.get(thread_id) ?? await data.permissionReadThread(user_id, thread_id);
			permCacheThread.set(thread_id, perms);
			if (!perms.has("View")) return;
			ws.send(JSON.stringify(msg));
		}
		
		function handleUsers(msg_user_id: string, msg: z.infer<typeof MessageServer>) {
			if (state === "closed") return;
			if (msg_user_id !== user_id) return;
			ws.send(JSON.stringify(msg));
		}

		async function handleHello(token: string, _last_id?: string) {
			const session = await data.sessionSelectByToken(token);
			if (!session) return c.json({ error: "Invalid or expired token" }, 401);
			// if (row.level as number < 1) return c.json({ error: "Unauthorized" }, 403);
			user_id = session.user_id;
			const user = await data.userSelect(user_id);
			if (!user) {
				throw new Error("user doesn't exist, but session does...!?");
			}
			state = "auth";
			send({ type: "ready", user });
		}

		const middle = upgradeWebSocket(() => ({
			onOpen(ev) {
				console.log(`opened websocket ${id}`);
				ws = ev.target as WebSocket;
				state = "unauth";
				events.on("global", handle);
				events.on("rooms", handleRoom);
				events.on("threads", handleThread);
				events.on("users", handleUsers);
				rescheduleHeartbeat();
			},
			onClose() {
				console.log(`closed websocket ${id}`);
				events.off("global", handle);
				events.off("rooms", handleRoom);
				events.off("threads", handleThread);
				events.off("users", handleUsers);
				state = "closed";
			},
			onMessage(event, ws) {
				try {
					// event.data is occasionally null, and i have no idea why
					
					const msg = MessageClient.parse(JSON.parse(event.data as string));
					// console.log(`recv websocket ${id}`, msg);
					if (msg.type === "hello") {
						if (state === "auth") {
							send({ type: "error", error: "already authenticated" });
							return;
						}
						handleHello(msg.token, msg.last_id);
					} else if (msg.type === "pong") {
						rescheduleHeartbeat();
					}
				} catch (err: any) {
					console.log(`websocket recv message error ${id}`, err);
					if (ws.readyState === WebSocket.OPEN) {
						ws.send(JSON.stringify({
							type: "error",
							error: err.format(),
						}));
					}
					ws.close();
					state = "closed";
				}
			},
			onError(evt, ws) {
      	console.log(`websocket error`, evt);
      	ws.close();
      	state = "closed";
      },
		}));

		const r = await middle(c, next);
		return r ?? c.text("error", 500);
	});
}
