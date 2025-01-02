import { OpenAPIHono, z } from "@hono/zod-openapi";
import { db, events, HonoEnv } from "globals";
import { SyncInit } from "./def.ts";
import { upgradeWebSocket } from "npm:hono/deno";
import { MessageClient, MessageServer } from "../../types/sync.ts";
import { User } from "../../types.ts";
import { UserFromDb } from "../../types/db.ts";
import { uuidv7 } from "uuidv7";

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
			console.log(`send websocket ${id}`, msg);
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

		const handle = (msg: z.infer<typeof MessageServer>) => {
			if (state === "closed") return;
			// if (state === "unauth") return;
			ws.send(JSON.stringify(msg));
		};

		const middle = upgradeWebSocket(() => ({
			onOpen(ev) {
				console.log(`opened websocket ${id}`);
				ws = ev.target as WebSocket;
				state = "unauth";
				events.on("sushi", handle);
				rescheduleHeartbeat();
			},
			onClose() {
				console.log(`closed websocket ${id}`);
				events.off("sushi", handle);
				state = "closed";
			},
			onMessage(event, ws) {
				try {
					console.log(`recv websocket ${id}`, event.data);
					const msg = MessageClient.parse(JSON.parse(event.data as string));
					// console.log(`recv websocket ${id}`, msg);
					if (msg.type === "hello") {
						if (state === "auth") {
							send({ type: "error", error: "already authenticated" });
							return;
						}
						const rowSession = db.prepareQuery(
							"SELECT * FROM sessions WHERE token = ?",
						).firstEntry([msg.token]);
						if (!rowSession) {
							return c.json({ error: "Invalid or expired token" }, 401);
						}
						// if (row.level as number < 1) return c.json({ error: "Unauthorized" }, 403);
						user_id = rowSession.user_id as string;
						const rowUser = db.prepareQuery(
							"SELECT * FROM users WHERE id = ?",
						).firstEntry([user_id]);
						if (!rowUser) {
							throw new Error("user doesn't exist, but session does...!?");
						}
						state = "auth";
						send({
							type: "ready",
							user: User.parse(UserFromDb.parse(rowUser)),
						});
					} else if (msg.type === "pong") {
						rescheduleHeartbeat();
					}
				} catch (err) {
					console.log(`websocket error ${id}`, err);
					ws.close();
					state = "closed";
				}
			},
		}));

		const r = await middle(c, next);
		return r ?? c.text("error", 500);
	});
}
