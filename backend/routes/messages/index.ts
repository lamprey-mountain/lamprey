import { OpenAPIHono } from "@hono/zod-openapi";
import { events, data, HonoEnv, blobs } from "globals";
import {
MessageAck,
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
		if (r.attachments?.length) {
			for (const att of r.attachments) {
				const existing = await data.mediaLinkSelect(att.id);
				if (existing.length) return c.json({ error: "cant reuse media" }, 400);
			}
			for (const att of r.attachments) {
				await data.mediaLinkInsert(att.id, message_id);
			}
		}
		const message = await data.messageInsert(r, {
			type: MessageType.Default,
			id: message_id,
			thread_id,
			version_id: message_id,
			author_id: user_id,
		});
		for (const a of message.attachments) {
			a.url = await blobs.presignedGetUrl(a.url);
		}
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
		for (const m of messages.items) {
			for (const a of m.attachments) {
				a.url = await blobs.presignedGetUrl(a.url);
			}
		}
		return c.json(messages, 200);
	});
	
	app.openapi(withAuth(MessageGet), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const message_id = c.req.param("message_id")!;
		const message = await data.messageSelect(thread_id, message_id);
		if (!message) return c.json({ error: "not found" }, 404);
		for (const a of message.attachments) {
			a.url = await blobs.presignedGetUrl(a.url);
		}
		return c.json(message, 200);
	});
	
	// app.openapi(withAuth(MessageUpdate), async (c) => {});
	// app.openapi(withAuth(MessageDelete), async (c) => {});
	// app.openapi(withAuth(MessageGet), async (c) => {});
	// app.openapi(withAuth(MessageVersionsList), async (c) => {});
	// app.openapi(withAuth(MessageVersionsGet), async (c) => {});
	// app.openapi(withAuth(MessageVersionsDelete), async (c) => {});
	
	// TODO: bulk ack?
	app.openapi(withAuth(MessageAck), async (c) => {
		const user_id = c.get("user_id");
		const thread_id = c.req.param("thread_id");
		const message_id = c.req.param("message_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		await data.unreadMarkMessage(user_id, thread_id, message_id);
		return new Response(null, { status: 204 });
	});
}
