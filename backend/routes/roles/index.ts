import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { RoleList } from "./def.ts";

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
}
