// TODO: rename this to globals.ts
import { DB } from "https://deno.land/x/sqlite@v3.9.1/mod.ts";
import { MessageServer } from "./types/sync.ts";
import { z } from "@hono/zod-openapi";
// import { PGlite } from "pglite";
import EventEmitter from "events";
export * as discord from "./oauth2.ts";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

// maybe use postgres?
// await PGlite.create({
//   dataDir: "./data",
// });

// export const db = new DB();
export const db = new DB("test.db");
// db.execute(Deno.readTextFileSync("schema.sql"));

type MsgServer = z.infer<typeof MessageServer>;

type Events = {
  sushi: (msg: MsgServer) => void,
}

export const events = new EventEmitter() as TypedEmitter<Events>;
export const broadcast = (msg: MsgServer) => events.emit("sushi", msg);

export type HonoEnv = {
  Variables: {
    session_id: string,
    user_id: string,
    session_level: number,
  },
};

export enum SessionStatus {
  Unauthorized = 0,
  Default = 1,
  Sudo = 2,
}

export enum Permissions {
  RoomManage         = 1 << 0,
  ThreadCreate       = 1 << 1,
  ThreadManage       = 1 << 2,
  MessageCreate      = 1 << 3,
  MessageFilesEmbeds = 1 << 4,
  MessagePin         = 1 << 5,
  MessageManage      = 1 << 6,
  MessageMassMention = 1 << 7,
  MemberKick         = 1 << 8,
  MemberBan          = 1 << 9,
  MemberManage       = 1 << 10,
  InviteCreate       = 1 << 11,
  InviteManage       = 1 << 12,
  RoleManage         = 1 << 13,
  RoleApply          = 1 << 14,
}

export const queries = {
  roomInsert: db.prepareQuery("INSERT INTO rooms (room_id, name, description) VALUES (:room_id, :name, :description) RETURNING *"),
  roomUpdate: db.prepareQuery("UPDATE rooms SET name = :name, description = :description WHERE room_id = :room_id RETURNING *"),
  roomSelect: db.prepareQuery("SELECT * FROM rooms WHERE room_id = :room_id"),
  threadInsert: db.prepareQuery("INSERT INTO threads (thread_id, room_id, name, description, is_closed, is_locked) VALUES (:thread_id, :room_id, :name, :description, :is_closed, :is_locked) RETURNING *"),
  threadSelect: db.prepareQuery("SELECT * FROM threads WHERE thread_id = :thread_id"),
  threadUpdate: db.prepareQuery("UPDATE threads SET name = :name, description = :description, is_closed = :is_closed, is_locked = :is_locked WHERE thread_id = :thread_id"),
  sessionSelect: db.prepareQuery("SELECT * FROM sessions WHERE session_id = :session_id"),
  sessionSelectUser: db.prepareQuery("SELECT * FROM sessions WHERE user_id = :user_id"),
}
