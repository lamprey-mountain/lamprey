import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv4, uuidv7 } from "uuidv7";
import { Invite, Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { InviteCreateRoom, InviteUse } from "./def.ts";
import { nanoid } from "nanoid";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(InviteCreateRoom), async (c) => {
		const user_id = c.get("user_id");
		const room_id = c.req.param("room_id");
		const perms = c.get("permissions");
		if (!perms.has("InviteCreate")) return c.json({ error: "can't do that" }, 403);
		const invite = await data.inviteInsertRoom(room_id, user_id, nanoid());
		// broadcast({ type: "upsert.invite", invite });
		return c.json(invite, 201);
	});
	
	app.openapi(withAuth(InviteUse), async (c) => {
		const user_id = c.get("user_id");
		const invite_code = c.req.param("invite_code");
		const invite = await data.inviteSelect(invite_code);
		if (invite.target_type !== "room") return c.json({ error: "not yet implemented" }, 501);
		if (await data.memberSelect(invite.target_id, user_id)) return c.json({ error: "already in room" }, 400);
		const _member = await data.memberInsert(user_id, {
			room_id: invite.target_id,
			membership: "join",
			override_name: null,
			override_description: null
		});
		// broadcast({ type: "upsert.member", member });
		return new Response(null, { status: 204 });
	});
}
