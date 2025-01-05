import { OpenAPIHono } from "@hono/zod-openapi";
import { data, events, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { SessionCreate, SessionDelete, SessionDeleteSelf, SessionGet, SessionGetSelf, SessionList } from "./def.ts";
import { uuidv4, uuidv7 } from "uuidv7";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(SessionCreate, async (c) => {
		const r = await c.req.json();
		const session = await data.sessionInsert({
			id: uuidv7(),
			token: uuidv4(),
			user_id: r.user_id,
			status: 0,
		});
	  return c.json(session, 201);
	});

	// app.openapi(withAuth(SessionUpdate, { strict: false }), async (c) => {
	//   throw "todo"
	// });

	app.openapi(withAuth(SessionDelete), async (c) => {
		const session_id = c.req.param("session_id");
		await data.sessionDelete(session_id)
		events.emit("users", c.get("user_id"), { type: "delete.session", id: session_id });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(SessionDeleteSelf), async (c) => {
		const session_id = c.get("session_id");
		await data.sessionDelete(session_id)
		events.emit("users", c.get("user_id"), { type: "delete.session", id: session_id });
		return new Response(null, { status: 204 });
	});

	// app.openapi(withAuth(SessionList), (c) => {
	// 	const uid = c.get("user_id");
	// 	const sessions = db.prepareQuery("SELECT * FROM sessions WHERE user_id = ?")
	// 		.allEntries([uid]).map((i) => Session.parse(i));
	// 	return c.json({ sessions }, 200);
	// });

	app.openapi(withAuth(SessionGet), async (c) => {
		const session = await data.sessionSelect(c.req.param("session_id"))
		if (!session) return c.json({ error: "not found" }, 404);
		if (session.user_id !== c.get("user_id")) {
			return c.json({ error: "not found" }, 404);
		}
		return c.json(session, 200);
	});
	
	app.openapi(withAuth(SessionGetSelf), async (c) => {
		const session = await data.sessionSelect(c.get("session_id"))
		if (!session) return c.json({ error: "not found" }, 404);
		return c.json(session, 200);
	});
}
