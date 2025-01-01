import { OpenAPIHono } from "@hono/zod-openapi";
import { RoomCreate, RoomGet, RoomList, RoomUpdate, DmInitialize, DmGet, RoomAck } from "./def.ts";
import { withAuth } from "../../auth.ts";
import { broadcast, queries as q, db, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
  app.openapi(withAuth(RoomCreate), async (c) => {
    const roomReq = await c.req.json();
    const row = q.roomInsert.firstEntry({
      room_id: uuidv7(),
      name: roomReq.name,
      description: roomReq.description,
    })!;
    const room = Room.parse(row);
    broadcast({ type: "upsert.room", room });
    return c.json(room, 201);
  });

  // FIXME: paginate
  app.openapi(withAuth(RoomList), (c) => {
    const limit = c.req.query("limit") ?? 10;
    const rows = db.prepareQuery("SELECT * FROM rooms LIMIT ?").allEntries([limit]);
    if (!rows) throw new Error("database error");
    return c.json(rows);
  });

  app.openapi(withAuth(RoomUpdate), async (c) => {
    const patch = await c.req.json();
    const room_id = c.req.param("room_id");
    let row;
    db.transaction(() => {
      const old = q.roomSelect.firstEntry({ room_id });
      if (!old) return;
      row = q.roomUpdate.firstEntry({
        room_id,
        name: patch.name === undefined ? old.name : patch.name,
        description: patch.description === undefined ? old.description : patch.description,
      });
    });
    if (!row) return c.json({ error: "not found" }, 404);
    const room = Room.parse(row);
    broadcast({ type: "upsert.room", room });
    return c.json(room, 200);
  });

  app.openapi(withAuth(RoomGet), (c) => {
    const room_id = c.req.param("room_id");
    const row = q.roomSelect.firstEntry({ room_id });
    if (!row) return c.json({ error: "not found" }, 404);
    const room = Room.parse(row);
    return c.json(room, 200);
  });
}
