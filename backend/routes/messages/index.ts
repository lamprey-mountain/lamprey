import { OpenAPIHono } from "@hono/zod-openapi";
import { events, data, HonoEnv, blobs, MediaLinkType } from "globals";
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
import { MessageType } from "../../types.ts";

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
				await data.mediaLinkInsert(att.id, message_id, MediaLinkType.Message);
				await data.mediaLinkInsert(att.id, message_id, MediaLinkType.MessageVersion);
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
	
	app.openapi(withAuth(MessageUpdate), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const message_id = c.req.param("message_id")!;
		const user_id = c.get("user_id");
		const message = await data.messageSelect(thread_id, message_id);
		if (!message) return c.json({ error: "not found" }, 404);
		if (message.type !== MessageType.Default) return c.json({ error: "invalid edit" }, 400);
		if (message.author.id === user_id) perms.add("MessageEdit");
		if (!perms.has("MessageEdit")) return c.json({ error: "missing permission" }, 403);
		const r = await c.req.json();
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
		const version_id = uuidv7();
		if (r.attachments?.length) {
			for (const att of r.attachments) {
				const existing = await data.mediaLinkSelect(att.id);
				const already_linked = existing.some(i => i.link_type === MediaLinkType.Message && i.target_id !== message_id);
				if (already_linked) return c.json({ error: "cant reuse media" }, 400);
			}
			for (const att of r.attachments) {
				await data.mediaLinkInsert(att.id, version_id, MediaLinkType.MessageVersion);
			}
		}
		const messageNew = await data.messageInsert(r, {
			type: MessageType.Default,
			id: message_id,
			thread_id,
			version_id,
			author_id: user_id,
			ordering: message.ordering,
		});
		for (const a of messageNew.attachments) {
			a.url = await blobs.presignedGetUrl(a.url);
		}
		events.emit("threads", thread_id, { type: "upsert.message", message: { ...messageNew, nonce: r.nonce } });
		return c.json(messageNew, 200);
	});
	
	app.openapi(withAuth(MessageDelete), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const message_id = c.req.param("message_id")!;
		const user_id = c.get("user_id");
		const message = await data.messageSelect(thread_id, message_id);
		if (!message) return c.json({ error: "not found" }, 404);
		if (message.type !== MessageType.Default) return c.json({ error: "invalid delete" }, 400);
		if (message.author.id === user_id) perms.add("MessageDelete");
		if (!perms.has("MessageDelete")) return c.json({ error: "missing permission" }, 403);
		await data.messageDelete(thread_id, message_id);
		await data.mediaLinkDeleteAll(message_id);
		events.emit("threads", thread_id, { type: "delete.message", id: message.id });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(MessageVersionsList), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const message_id = c.req.param("message_id")!;
		const messages = await data.versionList(thread_id, message_id, {
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
	
	app.openapi(withAuth(MessageVersionsGet), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const version_id = c.req.param("version_id")!;
		const message = await data.versionSelect(thread_id, version_id);
		if (!message) return c.json({ error: "not found" }, 404);
		for (const a of message.attachments) {
			a.url = await blobs.presignedGetUrl(a.url);
		}
		return c.json(message, 200);
	});
	
	app.openapi(withAuth(MessageVersionsDelete), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id")!;
		const version_id = c.req.param("version_id")!;
		const user_id = c.get("user_id");
		const message = await data.versionSelect(thread_id, version_id);
		if (!message) return c.json({ error: "not found" }, 404);
		if (message.type !== MessageType.Default) return c.json({ error: "invalid delete" }, 400);
		if (message.author.id === user_id) perms.add("MessageDelete");
		if (!perms.has("MessageDelete")) return c.json({ error: "missing permission" }, 403);
		await data.versionDelete(thread_id, version_id);
		await data.mediaLinkDelete(version_id, MediaLinkType.MessageVersion);
		events.emit("threads", thread_id, { type: "delete.message", id: message.id });
		return new Response(null, { status: 204 });
	});
	
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
