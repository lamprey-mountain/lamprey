import { OpenAPIHono } from "@hono/zod-openapi";
import { data, events, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { ThreadAck, ThreadCreate, ThreadGet, ThreadList, ThreadUpdate } from "./def.ts";
import { uuidv7 } from "uuidv7";
import { MessageType, ThreadType } from "../../types.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(ThreadCreate), async (c) => {
		const r = await c.req.json();
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("ThreadCreate")) return c.json({ error: "permission denied" }, 403);
		const thread = await data.threadInsert(uuidv7(), user_id, room_id, ThreadType.Default, r);
		const message_id = uuidv7();
		const message = await data.messageInsert({
			content: "(thread update)",
			metadata: r,
		}, {
			type: MessageType.ThreadUpdate,
			id: message_id,
			thread_id: thread.id,
			version_id: message_id,
			author_id: user_id,
		});
		events.emit("threads", thread.id, { type: "upsert.message", message });
		events.emit("threads", thread.id, { type: "upsert.thread", thread });
		return c.json(thread, 201);
	});

	app.openapi(withAuth(ThreadList), async (c) => {
		const user_id = c.get("user_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const room_id = c.req.param("room_id")!;
		const threads = await data.threadList(room_id, user_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(threads, 200);
	});
	
	app.openapi(withAuth(ThreadGet), async (c) => {
		const user_id = c.get("user_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const thread_id = c.req.param("thread_id");
		const thread = await data.threadSelect(thread_id, user_id);
		if (!thread) return c.json({ error: "not found" }, 404);
		return c.json(thread, 200);
	});

	app.openapi(withAuth(ThreadUpdate), async (c) => {
		const user_id = c.get("user_id");
		const patch = await c.req.json();
		const thread_id = c.req.param("thread_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("ThreadManage")) return c.json({ error: "forbidden" }, 403);
		const thread = await data.threadUpdate(thread_id, user_id, patch);
		if (!thread) return c.json({ error: "not found" }, 404);
		const message_id = uuidv7();
		const message = await data.messageInsert({
			content: "(thread update)",
			metadata: patch,
		}, {
			type: MessageType.ThreadUpdate,
			id: message_id,
			thread_id,
			version_id: message_id,
			author_id: user_id,
		});
		events.emit("threads", thread_id, { type: "upsert.thread", thread });
		events.emit("threads", thread_id, { type: "upsert.message", message });
		return c.json(thread, 200);
	});

	// app.openapi(withAuth(ThreadBulkUpdate), async (c) => {});
	
	app.openapi(withAuth(ThreadAck), async (c) => {
		const user_id = c.get("user_id");
		const thread_id = c.req.param("thread_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		await data.unreadMarkThread(user_id, thread_id);
		return new Response(null, { status: 204 });
	});
}
