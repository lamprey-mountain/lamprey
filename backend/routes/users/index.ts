import { OpenAPIHono } from "@hono/zod-openapi";
import { broadcast, data, HonoEnv } from "globals";
import { withAuth } from "../auth.ts";
import { UserCreate, UserDelete, UserDeleteSelf, UserGet, UserGetSelf, UserUpdate, UserUpdateSelf } from "./def.ts";
import { uuidv7 } from "uuidv7";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(UserCreate), async (c) => {
		const parent_id = c.get("user_id");
		const patch = await c.req.json();
		const user = await data.userInsert(uuidv7(), patch, {
			parent_id,
			is_system: false,
			can_fork: false,
			discord_id: null,
		})
		broadcast({ type: "upsert.user", user });
		return c.json(user, 201);
	});

	app.openapi(withAuth(UserUpdateSelf), async (c) => {
		const user_id = c.get("user_id");
		const patch = await c.req.json();
		const user = await data.userUpdate(user_id, patch, {});
		if (!user) return c.json({ error: "not found" }, 404);
		broadcast({ type: "upsert.user", user });
		return c.json(user, 200);
	});
	
	app.openapi(withAuth(UserUpdate), async (c) => {
		const user_id = c.req.param("user_id");
		const patch = await c.req.json();
		const user = await data.userUpdate(user_id, patch, {});
		if (!user) return c.json({ error: "not found" }, 404);
		broadcast({ type: "upsert.user", user });
		return c.json(user, 200);
	});
	
	app.openapi(withAuth(UserDeleteSelf), async (c) => {
		const user_id = c.get("user_id");
		await data.userDelete(user_id);
		broadcast({ type: "delete.user", id: user_id });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(UserDelete), async (c) => {
		const user_id = c.req.param("user_id");
		await data.userDelete(user_id);
		broadcast({ type: "delete.user", id: user_id });
		return new Response(null, { status: 204 });
	});
	
	// app.openapi(withAuth(UserGetSelf), async (c) => {
	// 	const user_id = c.get("user_id");
	// 	const user = await data.userSelect(user_id);
	// 	if (!user) return c.json({ error: "not found" }, 404);
	// 	return c.json(user, 200);
	// });
	
	app.openapi(withAuth(UserGet), async (c) => {
		let user_id = c.req.param("user_id");
		if (user_id === "@self") user_id = c.get("user_id");
		const user = await data.userSelect(user_id);
		if (!user) return c.json({ error: "not found" }, 404);
		return c.json(user, 200);
	});
}
