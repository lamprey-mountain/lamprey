import { OpenAPIHono } from "@hono/zod-openapi";
import { events, data, HonoEnv } from "globals";
import {
	MessageCreate,
	MessageDelete,
	MessageGet,
	MessageList,
	MessageUpdate,
	MessageVersionsDelete,
	MessageVersionsGet,
	MessageVersionsList,
} from "./def.ts";
import { withAuth } from "../auth.ts";
import { uuidv7 } from "uuidv7";
import { MessageFromDb } from "../../types/db.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { Message, MessageType } from "../../types.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(MessageCreate), async (c) => {
		const user_id = c.get("user_id");
		const thread_id = c.req.param("thread_id");
		const r = await c.req.json();
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("MessageCreate")) return c.json({ error: "permission denied" }, 403);
		if (r.attachments?.length || r.embeds?.length) {
			if (!perms.has("MessageFilesEmbeds")) return c.json({ error: "permission denied" }, 403);
		}
		if (typeof r.override_name === "string") {
			if (!perms.has("MessageMasquerade")) return c.json({ error: "permission denied" }, 403);
		}
		if (!r.content && !r.attachments?.length && !r.embeds?.length) {
			return c.json({
				error:
					"at least one of content, attachments, or embeds must be defined",
			}, 400);
		}
		const message_id = uuidv7();
		const message = await data.messageInsert(r, {
			type: MessageType.Default,
			id: message_id,
			thread_id,
			version_id: message_id,
			author_id: user_id,
		});
		events.emit("threads", thread_id, { type: "upsert.message", message: { ...message, nonce: r.nonce } });
		return c.json(message, 201);
	});

	app.openapi(withAuth(MessageList), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const messages = await data.messageList(thread_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(messages, 200);
	});
	
	app.openapi(withAuth(MessageGet), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const message_id = c.req.param("message_id")!;
		const message = await data.messageSelect(thread_id, message_id);
		if (!message) return c.json({ error: "not found" }, 404);
		return c.json(message, 200);
	});
	
	// app.openapi(withAuth(MessageUpdate), async (c) => {});
	// app.openapi(withAuth(MessageDelete), async (c) => {});
	// app.openapi(withAuth(MessageGet), async (c) => {});
	// app.openapi(withAuth(MessageVersionsList), async (c) => {});
	// app.openapi(withAuth(MessageVersionsGet), async (c) => {});
	// app.openapi(withAuth(MessageVersionsDelete), async (c) => {});
	// app.openapi(withAuth(MessageAck), async (c) => {});
	// TODO: bulk ack?
}
