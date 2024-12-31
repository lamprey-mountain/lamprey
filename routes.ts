import { z, OpenAPIHono } from "@hono/zod-openapi";
import { RoomCreate, RoomGet, RoomList, RoomUpdate, DmInitialize, DmGet, RoomAck } from "./routes/rooms.ts";
import { ThreadAck, ThreadBulkUpdate, ThreadCreate, ThreadGet, ThreadList, ThreadUpdate } from "./routes/threads.ts";
import { MessageAck, MessageCreate, MessageDelete, MessageList, MessageUpdate, MessageVersionsDelete, MessageVersionsGet, MessageVersionsList } from "./routes/messages.ts";
import { UserCreate, UserDelete, UserGet, UserUpdate } from "./routes/users.ts";
import { SessionCreate, SessionDelete, SessionList, SessionGet, SessionUpdate } from "./routes/sessions.ts";
import { InviteCreate, InviteDelete, InviteResolve, InviteRoomList, InviteUse } from "./routes/invite.ts";
import { RoleCreate, RoleDelete, RoleGet, RoleList, RoleUpdate } from "./routes/roles.ts";
import { MemberGet, MemberKick, MemberList, MemberRoleApply, MemberRoleRemove, MemberUpdate } from "./routes/members.ts";
import { BanCreate, BanDelete } from "./routes/bans.ts";
import { SearchMessages, SearchRooms, SearchThreads } from "./routes/search.ts";
import { MediaClone, MediaCreate } from "./routes/media.ts";
import { SyncInit } from "./routes/sync.ts";
import { ServerInfo } from "./routes/core.ts";
// import { LogList, ReportCreateMedia, ReportCreateMessage, ReportCreateUser, ReportCreateRoom, ReportCreateThread, ReportCreateMember } from "./routes/moderation.ts";
import { withAuth } from "./auth.ts";
import { queries as q, db, Permissions, HonoEnv, SessionStatus } from "./data.ts";
import { uuidv7 } from "uuidv7";
import { upgradeWebSocket } from "npm:hono/deno";
import { Session, Room, Thread, User } from "./types.ts";
import { MessageFromDb, ThreadFromDb, UserFromDb } from "./types/db.ts";
import { MessageClient, MessageServer } from "./types/sync.ts";
import { AuthPasswordSet, AuthTotpSet, AuthPasswordDo, AuthTotpDo, AuthDiscordInfo, AuthDiscordFinish, AuthLogin } from "./routes/auth.ts";
import * as bcrypt from "bcrypt";
import * as otpauth from "otpauth";

const UUID_MIN = "00000000-0000-0000-0000-000000000000";
const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";

const sushi = new Set<(msg: z.infer<typeof MessageServer>) => void>();

function broadcast(msg: z.infer<typeof MessageServer>) {
  for (const listener of sushi) listener(msg);
}

export function setup(app: OpenAPIHono<HonoEnv>) {
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
    broadcast({ type: "upsert.room", room });
    return c.json(room, 200);
  });

  app.openapi(withAuth(ThreadCreate), async (c) => {
    const r = await c.req.json();
    const room_id = c.req.param("room_id");
    const row = q.threadInsert.firstEntry({
      thread_id: uuidv7(),
      room_id,
      name: r.name,
      description: r.description,
      is_closed: r.is_closed ?? 0,
      is_locked: r.is_locked ?? 0
    })!;
    const thread = Thread.parse(ThreadFromDb.parse(row));
    broadcast({ type: "upsert.thread", thread });
    return c.json(thread, 201);
  });

  app.openapi(withAuth(ThreadList), (c) => {
    const room_id = c.req.param("room_id");
    const limit = parseInt(c.req.param("limit") ?? "10", 10);
    const after = c.req.param("after");
    const before = c.req.param("before");
    const [count] = db.prepareQuery("SELECT count(*) FROM threads WHERE room_id = ?").first([room_id])!;
    const rows = db.prepareQuery("SELECT * FROM threads WHERE room_id = ? AND id > ? AND id < ? LIMIT ?")
      .allEntries([room_id, after ?? UUID_MIN, before ?? UUID_MAX, limit + 1]);
    return c.json({
      has_more: rows.length > limit,
      total: count,
      threads: rows.slice(0, limit).map(i => ThreadFromDb.parse(i)),
    });
  });
  
  app.openapi(withAuth(ThreadGet), (c) => {
    const thread_id = c.req.param("thread_id");
    const row = q.threadSelect.firstEntry({ thread_id });
    if (!row) return c.json({ error: "not found" }, 404);
    return c.json(row);
  });
  
  app.openapi(withAuth(ThreadUpdate), async (c) => {
    const patch = await c.req.json();
    const thread_id = c.req.param("thread_id");
    let row;
    db.transaction(() => {
      const old = q.threadSelect.firstEntry({ thread_id });
      if (!old) return;
      row = q.threadUpdate.firstEntry({
        thread_id,
        name: patch.name === undefined ? old.name : patch.name,
        description: patch.description === undefined ? old.description : patch.description,
        is_closed: patch.is_closed === undefined ? old.is_closed : patch.is_closed,
        is_locked: patch.is_locked === undefined ? old.is_locked : patch.is_locked,
      });
    });
    if (!row) return c.json({ error: "not found" }, 404);
    const thread = Thread.parse(ThreadFromDb.parse(row));
    broadcast({ type: "upsert.thread", thread });
    return c.json(thread, 200);
  });

  app.openapi(withAuth(MessageCreate), async (c) => {
    const user_id = c.get("user_id");
    const room_id = c.req.param("room_id");
    const thread_id = c.req.param("thread_id");
    const r = await c.req.json();
    if (!r.content && !r.attachments?.length && !r.embeds?.length) {
      return c.json({ error: "at least one of content, attachments, or embeds must be defined" }, 400);
    }
    const message_id = uuidv7();
    const row = db.prepareQuery(`
    INSERT INTO messages (message_id, thread_id, version_id, ordering, content, metadata, reply, author_id)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    RETURNING *
    `).firstEntry([message_id, thread_id, message_id, 0, r.content, "{}", r.reply, user_id])!;
    const message = MessageFromDb.parse({ ...row, room_id });
    broadcast({ type: "upsert.message", message });
    return c.json(message, 201);
  });

  app.openapi(withAuth(MessageList), (c) => {
    const room_id = c.req.param("room_id");
    const thread_id = c.req.param("thread_id");
    const limit = parseInt(c.req.param("limit") ?? "10", 10);
    const after = c.req.param("after");
    const before = c.req.param("before");
    const rows = db.prepareQuery(`SELECT * FROM messages_coalesced WHERE thread_id = ? AND message_id > ? AND message_id < ? LIMIT ?`)
      .allEntries([thread_id, after ?? UUID_MIN, before ?? UUID_MAX, limit + 1]);
    const [count] = db.prepareQuery(`SELECT count(*) FROM messages_coalesced WHERE thread_id = ?`)
      .first([thread_id])!;
    return c.json({
      has_more: rows.length > limit,
      total: count,
      messages: rows.slice(0, limit).map(i => MessageFromDb.parse({ ...i, room_id })),
    });
  });
  
  app.openapi(withAuth(MessageUpdate), async (c) => {
    const patch = await c.req.json();
    const user_id = c.get("user_id");
    const room_id = c.req.param("room_id");
    const message_id = c.req.param("message_id");
    const thread_id = c.req.param("thread_id");
    let row: unknown;
    db.transaction(() => {
      const old = db.prepareQuery("SELECT * FROM messages WHERE message_id = ?").firstEntry([message_id]);
      if (!old) return;
      row = db.prepareQuery(`
      INSERT INTO messages (message_id, thread_id, version_id, ordering, content, metadata, reply, author_id)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      RETURNING *
      `).firstEntry([message_id, thread_id, uuidv7(), 0, patch.content === undefined ? old.content : patch.content, "{}", patch.reply === undefined ? old.reply : patch.reply, user_id])!;
    });
    if (!row) return c.json({ error: "not found" }, 404);
    const message = MessageFromDb.parse({ ...row, room_id });
    broadcast({ type: "upsert.message", message });
    return c.json(message, 200);
  });
  
  app.openapi(withAuth(MessageDelete), (c) => {
    const message_id = c.req.param("message_id");
    db.prepareQuery("DELETE FROM messages WHERE message_id = ?").execute([message_id]);
    broadcast({ type: "delete.message", message_id });
    return c.json({}, 204);
  });
  
  app.openapi(withAuth(MessageVersionsList), (c) => {
    const room_id = c.req.param("room_id");
    const thread_id = c.req.param("thread_id");
    const message_id = c.req.param("message_id");
    const limit = parseInt(c.req.param("limit") ?? "10", 10);
    const after = c.req.param("after");
    const before = c.req.param("before");
    const [count] = db.prepareQuery(`SELECT COUNT(*) FROM messages WHERE thread_id = ? AND message_id = ?`)
      .first([thread_id, message_id])!;
    const rows = db.prepareQuery(`SELECT * FROM messages WHERE thread_id = ? AND message_id = ? AND version_id > ? AND version_id < ? LIMIT ?`)
      .allEntries([thread_id, message_id, after ?? UUID_MIN, before ?? UUID_MAX, limit + 1]);
    return c.json({
      total: count,
      messages: rows.slice(0, limit).map(i => MessageFromDb.parse({ ...i, room_id })),
      has_more: rows.length > limit,
    });
  });
  
  app.openapi(withAuth(MessageVersionsGet), (c) => {
    const room_id = c.req.param("room_id");
    const version_id = c.req.param("version_id");
    const row = db.prepareQuery("SELECT * FROM messages WHERE version_id = ?").firstEntry([version_id]);
    return c.json(MessageFromDb.parse({ ...row, room_id }), 200);
  });
  
  app.openapi(withAuth(MessageVersionsDelete), (c) => {
    const version_id = c.req.param("version_id");
    db.prepareQuery("DELETE FROM messages WHERE version_id = ?").execute([version_id]);
    broadcast({ type: "delete.message_version", version_id });
    return c.json({}, 204);
  });
  
  app.openapi(withAuth(UserCreate), async (c) => {
    const parent_id = c.get("user_id");
    const patch = await c.req.json();
    const row = db.prepareQuery(`
        INSERT INTO users (user_id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
      )`
    ).firstEntry([uuidv7(), parent_id, patch.name, patch.description, patch.status, patch.is_bot, patch.is_alias, false, false]);
    const user = User.parse(UserFromDb.parse(row));
    broadcast({ type: "upsert.user", user });
    return c.json(user, 201);
  });
  
  app.openapi(withAuth(UserUpdate), async (c) => {
    const patch = await c.req.json();
    const user_id = c.req.param("user_id") === "@me" ? c.get("user_id") : c.req.param("user_id");
    let row;
    db.transaction(() => {
      const old = db.prepareQuery("SELECT * FROM users WHERE user_id = ?").firstEntry([user_id]);
      if (!old) return;
      row = db.prepareQuery(`
        UPDATE users
        SET name = :name, description = :description, status = :status
        WHERE user_id = :user_id
        RETURNING *
      `).firstEntry({
        user_id,
        name: patch.name === undefined ? old.name : patch.name,
        description: patch.description === undefined ? old.description : patch.description,
        status: patch.status === undefined ? old.status : patch.status,
      });
    });
    if (!row) return c.json({ error: "not found" }, 404);
    const user = User.parse(UserFromDb.parse(row));
    broadcast({ type: "upsert.user", user });
    return c.json(user, 200);
  });
  
  app.openapi(withAuth(UserDelete), async (c) => {
    const user_id = c.req.param("user_id") === "@me" ? c.get("user_id") : c.req.param("user_id");
    db.prepareQuery(`DELETE FROM users WHERE user_id = ?`).execute([user_id]);
    broadcast({ type: "delete.user", user_id });
    return c.json({}, 204);
  });
  
  app.openapi(withAuth(UserGet), async (c) => {
    const user_id = c.req.param("user_id") === "@me" ? c.get("user_id") : c.req.param("user_id");
    const row = db.prepareQuery(`SELECT * FROM users WHERE user_id = ?`).firstEntry([user_id]);
    const user = User.parse(UserFromDb.parse(row));
    return c.json(user, 200);
  });
  
  // app.openapi(SessionCreate, async (c) => {
  //   const row = db.prepareQuery(`INSERT INTO sessions WHERE user_id = ?`).firstEntry([user_id]);
  //   const user = User.parse(UserFromDb.parse(row));
  //   return c.json(user, 200);
  //   throw "todo"
  // });
  
  // app.openapi(withAuth(SessionUpdate, { strict: false }), async (c) => {
  //   throw "todo"
  // });
    
  app.openapi(withAuth(SessionDelete), async (c) => {
    const session_id = c.req.param("session_id") === "@me" ? c.get("session_id") : c.req.param("session_id");
    db.prepareQuery(`DELETE FROM sessions WHERE session_id = ?`).execute([session_id]);
    broadcast({ type: "delete.session", session_id });
    return c.json({}, 204);
  });

  app.openapi(withAuth(SessionList), (c) => {
    const uid = c.get("user_id");
    const sessions = db.prepareQuery("SELECT * FROM sessions WHERE user_id = ?").allEntries([uid]).map(i => Session.parse(i));
    return c.json({ sessions }, 200);
  });

  app.openapi(withAuth(SessionGet), (c) => {
    const session_id = c.req.param("session_id") === "@me" ? c.get("session_id") : c.req.param("session_id");
    const row = db.prepareQuery("SELECT * FROM sessions WHERE session_id = ?").firstEntry([session_id]);
    if (!row) return c.json({ error: "not found" }, 404);
    const session = Session.parse(row);
    if (session.user_id !== c.get("user_id")) return c.json({ error: "not found" }, 404);
    return c.json(Session.parse(row), 200);
  });

  app.openapi(AuthLogin, async (c) => {
    const req = await c.req.json();
    const userRow = db.prepareQuery("SELECT email FROM users WHERE email = ?").firstEntry([req.email]);
    if (!userRow) return c.json({ error: "Incorrect password" }, 401);
    const user_id = userRow.user_id as string;
    const pwRow = db.prepareQuery("SELECT data FROM auth WHERE user_id = ? AND type = ?").firstEntry([user_id, "password"]);
    if (!pwRow) return c.json({ error: "Incorrect password" }, 401);
    if (!await bcrypt.compare(req.password, pwRow.data as string)) return c.json({ error: "Incorrect password" }, 403);
    const sessionRow = db.prepareQuery(`
      INSERT INTO sessions (session_id, user_id, token, status)
      VALUES (?, ?, ?, ?)
      RETURNING *
    `).firstEntry([uuidv7(), user_id, crypto.randomUUID(), SessionStatus.Default])!;
    return c.json(sessionRow, 201);
  });

  // TODO: proper auth
//   app.openapi(withAuth(AuthPasswordDo, { strict: false }), async (c) => {
//     const req = await c.req.json();
//     const user_id = c.get("user_id");
//     const row = db.prepareQuery("SELECT data FROM auth WHERE user_id = ? AND type = ?").first([user_id, "password"]);
//     if (!row) return c.json({ error: "Incorrect password" });
//     await bcrypt.compare(req)
//     throw "todo"
//   });
  
//   app.openapi(AuthPasswordSet, async (c) => {
//     const req = await c.req.json();
// // bcrypt.hash
//     throw "todo"
//   });
  
//   app.openapi(AuthTotpDo, async (c) => {
//     const req = await c.req.json();
//     throw "todo"
//   });
  
//   app.openapi(AuthTotpSet, async (c) => {
//     const req = await c.req.json();
//     throw "todo"
//   });
  
//   app.openapi(AuthDiscordInfo, async (c) => {
//     throw "todo"
//   });
  
//   app.openapi(AuthDiscordFinish, async (c) => {
//     throw "todo"
//   });
  
  // app.openapi(InviteCreate, async (c) => { throw "todo" });
  // app.openapi(InviteResolve, async (c) => { throw "todo" });
  // app.openapi(InviteUse, async (c) => { throw "todo" });
  // app.openapi(InviteRoomList, async (c) => { throw "todo" });
  // app.openapi(InviteDelete, async (c) => { throw "todo" });
  // app.openapi(RoleCreate, async (c) => { throw "todo" });
  // app.openapi(RoleList, async (c) => { throw "todo" });
  // app.openapi(RoleGet, async (c) => { throw "todo" });
  // app.openapi(RoleUpdate, async (c) => { throw "todo" });
  // app.openapi(RoleDelete, async (c) => { throw "todo" });
  // app.openapi(MemberList, async (c) => { throw "todo" });
  // app.openapi(MemberGet, async (c) => { throw "todo" });
  // app.openapi(MemberKick, async (c) => { throw "todo" });
  // app.openapi(MemberUpdate, async (c) => { throw "todo" });
  // app.openapi(MemberRoleApply, async (c) => { throw "todo" });
  // app.openapi(MemberRoleRemove, async (c) => { throw "todo" });
  // app.openapi(BanCreate, async (c) => { throw "todo" });
  // app.openapi(BanDelete, async (c) => { throw "todo" });
  // app.openapi(DmInitialize, async (c) => { throw "todo" });
  // app.openapi(DmGet, async (c) => { throw "todo" });
  // app.openapi(SearchMessages, async (c) => { throw "todo" });
  // app.openapi(SearchThreads, async (c) => { throw "todo" });
  // app.openapi(SearchRooms, async (c) => { throw "todo" });
  // app.openapi(ThreadAck, async (c) => { throw "todo" });
  // app.openapi(MessageAck, async (c) => { throw "todo" });
  // app.openapi(RoomAck, async (c) => { throw "todo" });
  // app.openapi(MediaCreate, async (c) => { throw "todo" });
  // app.openapi(MediaClone, async (c) => { throw "todo" });

  app.openapi(SyncInit, async (c, next) => {
    let ws: WebSocket;
    // let state: "closed" | "ready" = "closed";
    const handle = (msg: z.infer<typeof MessageServer>) => {
      ws.send(JSON.stringify(msg));
    };
    const middle = upgradeWebSocket(() => ({
      onOpen(ev) {
        ws = ev.target as WebSocket;
        sushi.add(handle);
      },
      onClose() {
        sushi.delete(handle);
      },
      onMessage(_event, _ws) {
        console.log(_event.data)
      },
    }));
    const r = await middle(c, next);
    return r ?? c.text("error", 500);
  });

  // app.openapi(ServerInfo, async (c) => { throw "todo"; });
}
