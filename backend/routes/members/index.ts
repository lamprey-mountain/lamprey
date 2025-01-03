import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { RoomMemberGet, RoomMemberGetSelf } from "./def.ts";

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
}
