import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, db, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { Session } from "../../types.ts";
import { SessionDelete, SessionGet, SessionList } from "./def.ts";

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

	app.openapi(withAuth(SessionDelete), (c) => {
		const session_id = c.req.param("session_id") === "@me"
			? c.get("session_id")
			: c.req.param("session_id");
		db.prepareQuery(`DELETE FROM sessions WHERE session_id = ?`).execute([
			session_id,
		]);
		broadcast({ type: "delete.session", session_id });
		return c.json({}, 204);
	});

	app.openapi(withAuth(SessionList), (c) => {
		const uid = c.get("user_id");
		const sessions = db.prepareQuery("SELECT * FROM sessions WHERE user_id = ?")
			.allEntries([uid]).map((i) => Session.parse(i));
		return c.json({ sessions }, 200);
	});

	app.openapi(withAuth(SessionGet), (c) => {
		const session_id = c.req.param("session_id") === "@me"
			? c.get("session_id")
			: c.req.param("session_id");
		const row = db.prepareQuery("SELECT * FROM sessions WHERE session_id = ?")
			.firstEntry([session_id]);
		if (!row) return c.json({ error: "not found" }, 404);
		const session = Session.parse(row);
		if (session.user_id !== c.get("user_id")) {
			return c.json({ error: "not found" }, 404);
		}
		return c.json(Session.parse(row), 200);
	});
}
