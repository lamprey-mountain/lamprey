import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { RoleCreate, RoleDelete, RoleGet, RoleList, RoleListMembers, RoleUpdate, ThreadOverwriteRoleDelete, ThreadOverwriteRoleSet, ThreadOverwriteUserDelete, ThreadOverwriteUserSet } from "./def.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(RoleList), async (c) => {
		const room_id = c.req.param("room_id")!;
		const roles = await data.roleList(room_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(roles, 200);
	});

	app.openapi(withAuth(RoleCreate), async (c) => {
		const perms = c.get("permissions")!;
		if (!perms.has("RoleManage")) return c.json({ error: "nope" }, 403);
		const room_id = c.req.param("room_id")!;
		const r = await c.req.json();
		const role = await data.roleInsert(r, {
			room_id,
			id: uuidv7(),
		});
		return c.json(role, 201);
	});
	
	app.openapi(withAuth(RoleUpdate), async (c) => {
		const perms = c.get("permissions")!;
		if (!perms.has("RoleManage")) return c.json({ error: "nope" }, 403);
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		const patch = await c.req.json();
		const role = await data.roleUpdate(room_id, role_id, patch);
		if (!role) return c.json({ error: "not found" }, 404);
		return c.json(role, 200);
	});
	
	app.openapi(withAuth(RoleDelete), async (c) => {
		const perms = c.get("permissions")!;
		if (!perms.has("RoleManage")) return c.json({ error: "nope" }, 403);
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		await data.roleDelete(room_id, role_id);
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(RoleGet), async (c) => {
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		const role = await data.roleSelect(room_id, role_id);
		if (!role) return c.json({ error: "not found" }, 404);
		return c.json(role, 200);
	});
	
	// app.openapi(withAuth(ThreadOverwriteRoleSet), async (c) => {});
	// app.openapi(withAuth(ThreadOverwriteRoleDelete), async (c) => {});
	// app.openapi(withAuth(ThreadOverwriteUserSet), async (c) => {});
	// app.openapi(withAuth(ThreadOverwriteUserDelete), async (c) => {});
	// app.openapi(withAuth(RoleListMembers), async (c) => {});
}
