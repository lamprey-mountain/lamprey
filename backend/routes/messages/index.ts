import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, db, HonoEnv } from "globals";
import {
	MessageCreate,
	MessageDelete,
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
import { Message } from "../../types.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(MessageCreate), async (c) => {
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const thread_id = c.req.param("thread_id");
		const r = await c.req.json();
		if (!r.content && !r.attachments?.length && !r.embeds?.length) {
			return c.json({
				error:
					"at least one of content, attachments, or embeds must be defined",
			}, 400);
		}
		const message_id = uuidv7();
		const row = db.prepareQuery(`
    INSERT INTO messages (message_id, thread_id, version_id, ordering, content, metadata, reply, author_id)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    RETURNING *
    `).firstEntry([
			message_id,
			thread_id,
			message_id,
			0,
			r.content,
			"{}",
			r.reply,
			user_id,
		])!;
		const message = Message.parse({
			...MessageFromDb.parse({ ...row, room_id }),
			nonce: r.nonce,
		});
		broadcast({ type: "upsert.message", message });
		return c.json(message, 201);
	});

	app.openapi(withAuth(MessageList), (c) => {
		const room_id = c.req.param("room_id");
		const thread_id = c.req.param("thread_id");
		const limit = c.req.query("limit") ?? 10;
		const after = c.req.query("after");
		const before = c.req.query("before");
		const rows = db.prepareQuery(
			`SELECT * FROM messages_coalesced WHERE thread_id = ? AND message_id > ? AND message_id < ? LIMIT ?`,
		)
			.allEntries([
				thread_id,
				after ?? UUID_MIN,
				before ?? UUID_MAX,
				limit + 1,
			]);
		const [count] = db.prepareQuery(
			`SELECT count(*) FROM messages_coalesced WHERE thread_id = ?`,
		)
			.first([thread_id])!;
		return c.json({
			has_more: rows.length > limit,
			total: count,
			messages: rows.slice(0, limit).map((i) =>
				Message.parse(MessageFromDb.parse({ ...i, room_id }))
			),
		});
	});

	app.openapi(withAuth(MessageUpdate), async (c) => {
		const patch = await c.req.json();
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const message_id = c.req.param("message_id");
		const thread_id = c.req.param("thread_id");
		let row: unknown;
		db.transaction(() => {
			const old = db.prepareQuery("SELECT * FROM messages WHERE message_id = ?")
				.firstEntry([message_id]);
			if (!old) return;
			row = db.prepareQuery(`
      INSERT INTO messages (message_id, thread_id, version_id, ordering, content, metadata, reply, author_id)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      RETURNING *
      `).firstEntry([
				message_id,
				thread_id,
				uuidv7(),
				0,
				patch.content === undefined ? old.content : patch.content,
				"{}",
				patch.reply === undefined ? old.reply : patch.reply,
				user_id,
			])!;
		});
		if (!row) return c.json({ error: "not found" }, 404);
		const message = MessageFromDb.parse({ ...row, room_id });
		broadcast({ type: "upsert.message", message });
		return c.json(message, 200);
	});

	app.openapi(withAuth(MessageDelete), (c) => {
		const message_id = c.req.param("message_id");
		db.prepareQuery("DELETE FROM messages WHERE message_id = ?").execute([
			message_id,
		]);
		broadcast({ type: "delete.message", message_id });
		return c.json({}, 204);
	});

	app.openapi(withAuth(MessageVersionsList), (c) => {
		const room_id = c.req.param("room_id");
		const thread_id = c.req.param("thread_id");
		const message_id = c.req.param("message_id");
		const limit = parseInt(c.req.param("limit") ?? "10", 10);
		const after = c.req.param("after");
		const before = c.req.param("before");
		const [count] = db.prepareQuery(
			`SELECT COUNT(*) FROM messages WHERE thread_id = ? AND message_id = ?`,
		)
			.first([thread_id, message_id])!;
		const rows = db.prepareQuery(
			`SELECT * FROM messages WHERE thread_id = ? AND message_id = ? AND version_id > ? AND version_id < ? LIMIT ?`,
		)
			.allEntries([
				thread_id,
				message_id,
				after ?? UUID_MIN,
				before ?? UUID_MAX,
				limit + 1,
			]);
		return c.json({
			total: count,
			messages: rows.slice(0, limit).map((i) =>
				MessageFromDb.parse({ ...i, room_id })
			),
			has_more: rows.length > limit,
		});
	});

	app.openapi(withAuth(MessageVersionsGet), (c) => {
		const room_id = c.req.param("room_id");
		const version_id = c.req.param("version_id");
		const row = db.prepareQuery("SELECT * FROM messages WHERE version_id = ?")
			.firstEntry([version_id]);
		return c.json(MessageFromDb.parse({ ...row, room_id }), 200);
	});

	app.openapi(withAuth(MessageVersionsDelete), (c) => {
		const version_id = c.req.param("version_id");
		db.prepareQuery("DELETE FROM messages WHERE version_id = ?").execute([
			version_id,
		]);
		broadcast({ type: "delete.message_version", version_id });
		return c.json({}, 204);
	});
}
