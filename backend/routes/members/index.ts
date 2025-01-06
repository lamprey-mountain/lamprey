import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { data, events, HonoEnv } from "globals";
import { MemberRoleApply, MemberRoleRemove, RoomMemberGet, RoomMemberKick, RoomMemberList } from "./def.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(RoomMemberGet), async (c) => {
		const perms = c.get("permissions")!;
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		let user_id = c.req.param("user_id");
		if (user_id === "@self") user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const member = await data.memberSelect(room_id, user_id);
		if (!member) return c.json({ error: "not found" }, 404);
		return c.json(member, 200);
	});
	
	app.openapi(withAuth(RoomMemberList), async (c) => {
		const perms = c.get("permissions")!;
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const room_id = c.req.param("room_id")!;
		const members = await data.memberList(room_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(members, 200);
	});
	
	app.openapi(withAuth(RoomMemberKick), async (c) => {
		let user_id = c.req.param("user_id");
		if (user_id === "@self") user_id = c.get("user_id");
		const room_id = c.req.param("room_id")!;
		const perms = c.get("permissions")!;
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("MemberManage") && c.get("user_id") !== user_id) return c.json({ error: "nope" }, 403);
		await data.memberDelete(room_id, user_id);
		events.emit("rooms", room_id, { type: "delete.member", room_id, user_id });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(MemberRoleApply), async (c) => {
		const perms = c.get("permissions");
		const user_id = c.req.param("user_id");
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("RoleApply")) return c.json({ error: "nope" }, 403);
		await data.roleApplyInsert(role_id, user_id);
		const member = (await data.memberSelect(room_id, user_id))!;
		events.emit("rooms", room_id, { type: "upsert.member", member });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(MemberRoleRemove), async (c) => {
		const perms = c.get("permissions");
		const user_id = c.req.param("user_id");
		const role_id = c.req.param("role_id");
		const room_id = c.req.param("room_id");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("RoleApply")) return c.json({ error: "nope" }, 403);
		await data.roleApplyDelete(role_id, user_id);
		const member = (await data.memberSelect(room_id, user_id))!;
		events.emit("rooms", room_id, { type: "upsert.member", member });
		return new Response(null, { status: 204 });
	});
}
