import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { data, events, HonoEnv } from "globals";
import { InviteCreateRoom, InviteDelete, InviteResolve, InviteRoomList, InviteUse } from "./def.ts";
import { customAlphabet } from "nanoid";
const nanoidInvite = customAlphabet("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", 12);

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(InviteCreateRoom), async (c) => {
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		if (!perms.has("InviteCreate")) return c.json({ error: "can't do that" }, 403);
		const invite = await data.inviteInsertRoom(room_id, user_id, nanoidInvite());
		events.emit("rooms", room_id,{ type: "upsert.invite", invite });
		return c.json(invite, 201);
	});
	
	app.openapi(withAuth(InviteUse), async (c) => {
		const user_id = c.get("user_id");
		const invite_code = c.req.param("invite_code");
		const invite = await data.inviteSelect(invite_code);
		if (invite.target_type !== "room") return c.json({ error: "not yet implemented" }, 501);
		if (await data.memberSelect(invite.target_id, user_id)) return c.json({ error: "already in room" }, 400);
		const member = await data.memberInsert(user_id, {
			room_id: invite.target_id,
			membership: "join",
			override_name: null,
			override_description: null
		});
		await data.applyDefaultRoles(user_id, invite.target_id);
		events.emit("rooms", invite.target_id, { type: "upsert.member", member });
		return new Response(null, { status: 204 });
	});
	
	app.openapi(withAuth(InviteRoomList), async (c) => {
		const perms = c.get("permissions");
		if (!perms.has("View")) return c.json({ error: "not found" }, 404);
		const room_id = c.req.param("room_id")!;
		const invites = await data.inviteList(room_id, {
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(invites, 200);
	});
	
	app.openapi(withAuth(InviteResolve), async (c) => {
		const invite_code = c.req.param("invite_code")!;
		const invite = await data.inviteSelect(invite_code);
		return c.json(invite, 200);
	});
	
	app.openapi(withAuth(InviteDelete), async (c) => {
		const invite_code = c.req.param("invite_code")!;
		const invite = await data.inviteSelect(invite_code);
		if (invite.target_type === "room") {
			const perms = await data.permissionReadRoom(c.get("user_id"), invite.target_id);
			if (!perms.has("View")) return c.json({ error: "not found" }, 404);
			if (!perms.has("InviteManage")) return c.json({ error: "nope" }, 403);
			await data.inviteDelete(invite_code);
			events.emit("rooms", invite.target_id, { type: "delete.invite", code: invite_code });
			return new Response(null, { status: 204 });
		} else {
			return c.json({ error: "todo" }, 501);
		}
	});
}
