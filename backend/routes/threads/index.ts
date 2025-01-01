import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, db, HonoEnv, queries as q } from "globals";
import { withAuth } from "../auth.ts";
import { ThreadCreate, ThreadGet, ThreadList, ThreadUpdate } from "./def.ts";
import { uuidv7 } from "uuidv7";
import { Thread } from "../../types.ts";
import { ThreadFromDb } from "../../types/db.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(ThreadCreate), async (c) => {
		const r = await c.req.json();
		const room_id = c.req.param("room_id");
		const row = q.threadInsert.firstEntry({
			thread_id: uuidv7(),
			room_id,
			name: r.name,
			description: r.description,
			is_closed: r.is_closed ?? 0,
			is_locked: r.is_locked ?? 0,
		})!;
		const thread = Thread.parse(ThreadFromDb.parse(row));
		broadcast({ type: "upsert.thread", thread });
		return c.json(thread, 201);
	});

	app.openapi(withAuth(ThreadList), (c) => {
		const room_id = c.req.param("room_id");
		const limit = parseInt(c.req.param("limit") ?? "10", 10);
		const after = c.req.param("after");
		const before = c.req.param("before");
		const [count] = db.prepareQuery(
			"SELECT count(*) FROM threads WHERE room_id = ?",
		).first([room_id])!;
		const rows = db.prepareQuery(
			"SELECT * FROM threads WHERE room_id = ? AND thread_id > ? AND thread_id < ? LIMIT ?",
		)
			.allEntries([room_id, after ?? UUID_MIN, before ?? UUID_MAX, limit + 1]);
		return c.json({
			has_more: rows.length > limit,
			total: count,
			threads: rows.slice(0, limit).map((i) => ThreadFromDb.parse(i)),
		});
	});

	app.openapi(withAuth(ThreadGet), (c) => {
		const thread_id = c.req.param("thread_id");
		const row = q.threadSelect.firstEntry({ thread_id });
		if (!row) return c.json({ error: "not found" }, 404);
		return c.json(row);
	});

	app.openapi(withAuth(ThreadUpdate), async (c) => {
		const patch = await c.req.json();
		const thread_id = c.req.param("thread_id");
		let row;
		db.transaction(() => {
			const old = q.threadSelect.firstEntry({ thread_id });
			if (!old) return;
			row = q.threadUpdate.firstEntry({
				thread_id,
				name: patch.name === undefined ? old.name : patch.name,
				description: patch.description === undefined
					? old.description
					: patch.description,
				is_closed: patch.is_closed === undefined
					? old.is_closed
					: patch.is_closed,
				is_locked: patch.is_locked === undefined
					? old.is_locked
					: patch.is_locked,
			});
		});
		if (!row) return c.json({ error: "not found" }, 404);
		const thread = Thread.parse(ThreadFromDb.parse(row));
		broadcast({ type: "upsert.thread", thread });
		return c.json(thread, 200);
	});
}
