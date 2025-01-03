import { OpenAPIHono } from "@hono/zod-openapi";
import { withAuth } from "../auth.ts";
import { broadcast, data, HonoEnv } from "globals";
import { uuidv4, uuidv7 } from "uuidv7";
import { Invite, Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { InviteCreateRoom } from "./def.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	// app.openapi(withAuth(InviteCreateRoom), async (c) => {
	// 	const user_id = c.get("user_id");
	// 	const room_id = c.req.param("room_id");
	// 	const _patch = await c.req.json();
	// 	const row = db.prepareQuery(`
 //    INSERT INTO invites (target_type, target_id, code, creator_id)
 //    VALUES (?, ?, ?, ?)
 //    RETURNING *
 //    `).firstEntry([
 //      "room",
 //      room_id,
 //      // nanoid(),
 //      uuidv4(),
 //      user_id,
	// 	])!;
	// 	const invite = Invite.parse(row);
	// 	// broadcast({ type: "upsert.invite", invite });
	// 	return c.json(invite, 201);
	// });

	// app.openapi(withAuth(RoomList), (c) => {
	// 	const limit = parseInt(c.req.param("limit") ?? "10", 10);
	// 	const after = c.req.param("after");
	// 	const before = c.req.param("before");
	// 	const [count] = db.prepareQuery(
	// 		"SELECT count(*) FROM rooms",
	// 	).first([])!;
	// 	const rows = db.prepareQuery(
	// 		"SELECT * FROM rooms WHERE id > ? AND id < ? LIMIT ?",
	// 	)
	// 		.allEntries([after ?? UUID_MIN, before ?? UUID_MAX, limit + 1]);
	// 	return c.json({
	// 		has_more: rows.length > limit,
	// 		total: count,
	// 		items: rows.slice(0, limit).map((i) => Room.parse(i)),
	// 	});
	// });

	// app.openapi(withAuth(RoomUpdate), async (c) => {
	// 	const patch = await c.req.json();
	// 	const room_id = c.req.param("room_id");
	// 	let row;
	// 	db.transaction(() => {
	// 		const old = q.roomSelect.firstEntry({ id: room_id });
	// 		if (!old) return;
	// 		row = q.roomUpdate.firstEntry({
	// 			id: room_id,
	// 			name: patch.name === undefined ? old.name : patch.name,
	// 			description: patch.description === undefined
	// 				? old.description
	// 				: patch.description,
	// 		});
	// 	});
	// 	if (!row) return c.json({ error: "not found" }, 404);
	// 	const room = Room.parse(row);
	// 	broadcast({ type: "upsert.room", room });
	// 	return c.json(room, 200);
	// });

	// app.openapi(withAuth(RoomGet), (c) => {
	// 	const room_id = c.req.param("room_id");
	// 	const row = q.roomSelect.firstEntry({ id: room_id });
	// 	if (!row) return c.json({ error: "not found" }, 404);
	// 	const room = Room.parse(row);
	// 	return c.json(room, 200);
	// });
}
