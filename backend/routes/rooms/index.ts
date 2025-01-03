import { OpenAPIHono } from "@hono/zod-openapi";
import {
	DmGet,
	DmInitialize,
	RoomAck,
	RoomCreate,
	RoomGet,
	RoomList,
	RoomUpdate,
} from "./def.ts";
import { withAuth } from "../../auth.ts";
import { broadcast, HonoEnv, data } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(RoomCreate), async (c) => {
		const roomReq = await c.req.json();
		const room = await data.roomInsert(uuidv7(), roomReq.name, roomReq.description);
		broadcast({ type: "upsert.room", room });
		return c.json(room, 201);
	});

	app.openapi(withAuth(RoomList), async (c) => {
		const limit = parseInt(c.req.param("limit") ?? "10", 10);
		const after = c.req.param("after");
		const before = c.req.param("before");
		// const c = await db.connect();
		const { rows: [count] } = await sal`
			SELECT count(*) FROM rooms
		`;
		const rows = await sal`
			SELECT * FROM rooms WHERE id > ${after ?? UUID_MIN} AND id < ${before ?? UUID_MAX} LIMIT ${limit + 1}",
		`;
		return c.json({
			has_more: rows.length > limit,
			total: count,
			items: rows.slice(0, limit).map((i) => Room.parse(i)),
		});
	});

	app.openapi(withAuth(RoomUpdate), async (c) => {
		const patch = await c.req.json();
		const room_id = c.req.param("room_id");
		const room = await data.roomUpdate(room_id, patch.name, patch.description);
		if (!room) return c.json({ error: "not found" }, 404);
		broadcast({ type: "upsert.room", room });
		return c.json(room, 200);
	});

	app.openapi(withAuth(RoomGet), async (c) => {
		const room_id = c.req.param("room_id");
		const room = await data.roomSelect(room_id);
		if (!room) return c.json({ error: "not found" }, 404);
		return c.json(room, 200);
	});
}
