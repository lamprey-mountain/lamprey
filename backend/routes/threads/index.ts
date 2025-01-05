import { OpenAPIHono } from "@hono/zod-openapi";
import { data, events, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { ThreadCreate, ThreadGet, ThreadList, ThreadUpdate } from "./def.ts";
import { uuidv7 } from "uuidv7";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(ThreadCreate), async (c) => {
		const r = await c.req.json();
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const perms = c.get("permissions");
		if (!perms.has("ThreadCreate")) return c.json({ error: "permission denied" }, 403);
		const thread = await data.threadInsert(uuidv7(), user_id, room_id, r);
		events.emit("threads", thread.id, { type: "upsert.thread", thread });
		return c.json(thread, 201);
	});

	app.openapi(withAuth(ThreadList), async (c) => {
		const room_id = c.req.param("room_id")!;
		const threads = await data.threadList(room_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(threads, 200);
	});
	
	app.openapi(withAuth(ThreadGet), async (c) => {
		const thread_id = c.req.param("thread_id");
		const thread = await data.threadSelect(thread_id);
		if (!thread) return c.json({ error: "not found" }, 404);
		return c.json(thread, 200);
	});

	app.openapi(withAuth(ThreadUpdate), async (c) => {
		const patch = await c.req.json();
		const thread_id = c.req.param("thread_id");
		const perms = c.get("permissions");
		if (!perms.has("ThreadManage")) return c.json({ error: "forbidden" }, 403);
		const thread = await data.threadUpdate(thread_id, patch);
		if (!thread) return c.json({ error: "not found" }, 404);
		events.emit("threads", thread_id, { type: "upsert.thread", thread });
		return c.json(thread, 200);
	});

	// app.openapi(withAuth(ThreadBulkUpdate), async (c) => {});
	// app.openapi(withAuth(ThreadAck), async (c) => {});
}
