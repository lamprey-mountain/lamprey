import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, db, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { UserFromDb } from "../../types/db.ts";
import { User } from "../../types.ts";
import { UserCreate, UserDelete, UserGet, UserUpdate } from "./def.ts";
import { uuidv7 } from "uuidv7";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(UserCreate), async (c) => {
		const parent_id = c.get("user_id");
		const patch = await c.req.json();
		const row = db.prepareQuery(`
        INSERT INTO users (user_id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
      )`).firstEntry([
			uuidv7(),
			parent_id,
			patch.name,
			patch.description,
			patch.status,
			patch.is_bot,
			patch.is_alias,
			false,
			false,
		]);
		const user = User.parse(UserFromDb.parse(row));
		broadcast({ type: "upsert.user", user });
		return c.json(user, 201);
	});

	app.openapi(withAuth(UserUpdate), async (c) => {
		const patch = await c.req.json();
		const user_id = c.req.param("user_id") === "@me"
			? c.get("user_id")
			: c.req.param("user_id");
		let row;
		db.transaction(() => {
			const old = db.prepareQuery("SELECT * FROM users WHERE user_id = ?")
				.firstEntry([user_id]);
			if (!old) return;
			row = db.prepareQuery(`
        UPDATE users
        SET name = :name, description = :description, status = :status
        WHERE user_id = :user_id
        RETURNING *
      `).firstEntry({
				user_id,
				name: patch.name === undefined ? old.name : patch.name,
				description: patch.description === undefined
					? old.description
					: patch.description,
				status: patch.status === undefined ? old.status : patch.status,
			});
		});
		if (!row) return c.json({ error: "not found" }, 404);
		const user = User.parse(UserFromDb.parse(row));
		broadcast({ type: "upsert.user", user });
		return c.json(user, 200);
	});

	app.openapi(withAuth(UserDelete), (c) => {
		const user_id = c.req.param("user_id") === "@me"
			? c.get("user_id")
			: c.req.param("user_id");
		db.prepareQuery(`DELETE FROM users WHERE user_id = ?`).execute([user_id]);
		broadcast({ type: "delete.user", user_id });
		return c.json({}, 204);
	});

	app.openapi(withAuth(UserGet), (c) => {
		const user_id = c.req.param("user_id") === "@me"
			? c.get("user_id")
			: c.req.param("user_id");
		const row = db.prepareQuery(`SELECT * FROM users WHERE user_id = ?`)
			.firstEntry([user_id]);
		const user = User.parse(UserFromDb.parse(row));
		return c.json(user, 200);
	});
}
