import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, data, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { SessionDelete, SessionDeleteSelf, SessionGet, SessionGetSelf, SessionList } from "./def.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	// app.openapi(SessionCreate, async (c) => {
	//   const row = db.prepareQuery(`INSERT INTO sessions WHERE user_id = ?`).firstEntry([user_id]);
	//   const user = User.parse(UserFromDb.parse(row));
	//   return c.json(user, 200);
	//   throw "todo"
	// });

	// app.openapi(withAuth(SessionUpdate, { strict: false }), async (c) => {
	//   throw "todo"
	// });

	app.openapi(withAuth(SessionDelete), async (c) => {
		const session_id = c.req.param("session_id");
		await data.sessionDelete(session_id)
		broadcast({ type: "delete.session", id: session_id });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(SessionDeleteSelf), async (c) => {
		const session_id = c.get("session_id");
		await data.sessionDelete(session_id)
		broadcast({ type: "delete.session", id: session_id });
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
