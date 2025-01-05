import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { MemberRoleApply, MemberRoleRemove, RoomMemberGet, RoomMemberGetSelf, RoomMemberList } from "./def.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(RoomMemberGetSelf), async (c) => {
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const member = await data.memberSelect(room_id, user_id);
		if (!member) return c.json({ error: "not found" }, 404);
		return c.json(member, 200);
	});
	
	app.openapi(withAuth(RoomMemberGet), async (c) => {
		const user_id = c.req.param("user_id");
		const room_id = c.req.param("room_id");
		const member = await data.memberSelect(room_id, user_id);
		if (!member) return c.json({ error: "not found" }, 404);
		return c.json(member, 200);
	});
	
	app.openapi(withAuth(RoomMemberList), async (c) => {
		const room_id = c.req.param("room_id")!;
		const members = await data.memberList(room_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(members, 200);
	});
	
	app.openapi(withAuth(MemberRoleApply), async (c) => {
		const perms = c.get("permissions");
		const user_id = c.req.param("user_id");
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		if (!perms.has("RoleApply")) return c.json({ error: "nope" }, 403);
		await data.roleApplyInsert(role_id, user_id);
		broadcast({
			type: "upsert.member",
			member: (await data.memberSelect(room_id, user_id))!,
		});
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(MemberRoleRemove), async (c) => {
		console.log("remove role")
		const perms = c.get("permissions");
		const user_id = c.req.param("user_id");
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		if (!perms.has("RoleApply")) return c.json({ error: "nope" }, 403);
		await data.roleApplyDelete(role_id, user_id);
		broadcast({
			type: "upsert.member",
			member: (await data.memberSelect(room_id, user_id))!,
		});
		return new Response(null, { status: 204 });
	});
}
