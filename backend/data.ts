// TODO: rename this to globals.ts
import { DB } from "https://deno.land/x/sqlite@v3.9.1/mod.ts";
import { MessageServer } from "./types/sync.ts";
import { z } from "@hono/zod-openapi";
import { Room } from "./types.ts";
// import { RoomFromDb } from "./types/db.ts";
import EventEmitter from "events";
export * as discord from "./oauth2.ts";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

import { Pool } from "postgres";
import { uuidv7 } from "uuidv7";

const db = new Pool({
	database: "chat",
	hostname: "localhost",
	port: 5432,
	user: "chat",
	password: "ce00eebd05027ca1",
}, 8);

const migrations = [...Deno.readDirSync("migrations")].sort((a, b) => a.name > b.name ? 1 : -1)
const q = await db.connect();
for (const migration of migrations) {
	const sql = await Deno.readTextFile(`migrations/${migration.name}`);
	await q.queryObject(sql);
}
q.release();

type MsgServer = z.infer<typeof MessageServer>;

type Events = {
	sushi: (msg: MsgServer) => void;
};

export const events = new EventEmitter() as TypedEmitter<Events>;
export const broadcast = (msg: MsgServer) => events.emit("sushi", msg);

export type HonoEnv = {
	Variables: {
		session_id: string;
		user_id: string;
		session_level: number;
	};
};

export enum SessionStatus {
	Unauthorized = 0,
	Default = 1,
	Sudo = 2,
}

export enum Permissions {
	RoomManage = 1 << 0,
	ThreadCreate = 1 << 1,
	ThreadManage = 1 << 2,
	MessageCreate = 1 << 3,
	MessageFilesEmbeds = 1 << 4,
	MessagePin = 1 << 5,
	MessageManage = 1 << 6,
	MessageMassMention = 1 << 7,
	MemberKick = 1 << 8,
	MemberBan = 1 << 9,
	MemberManage = 1 << 10,
	InviteCreate = 1 << 11,
	InviteManage = 1 << 12,
	RoleManage = 1 << 13,
	RoleApply = 1 << 14,
}

type Awaitable<T> = T | Promise<T>;

type Database = {
	roomSelect(id: string): Awaitable<z.infer<typeof Room> | null>;
	roomInsert(id: string, name: string, description: string | null): Awaitable<z.infer<typeof Room>>;
	roomUpdate(id: string, name?: string | null, description?: string | null): Awaitable<z.infer<typeof Room> | null>;
// 	threadInsert: db.prepareQuery(
// 		"INSERT INTO threads (id, room_id, name, description, is_closed, is_locked) VALUES (:id, :room_id, :name, :description, :is_closed, :is_locked) RETURNING *",
// 	),
// 	threadSelect: db.prepareQuery(
// 		"SELECT * FROM threads WHERE id = :id",
// 	),
// 	threadUpdate: db.prepareQuery(
// 		"UPDATE threads SET name = :name, description = :description, is_closed = :is_closed, is_locked = :is_locked WHERE id = :id",
// 	),
// 	sessionSelect: db.prepareQuery(
// 		"SELECT * FROM sessions WHERE id = :id",
// 	),
// 	sessionSelectUser: db.prepareQuery(
// 		"SELECT * FROM sessions WHERE user_id = :user_id",
// 	),
}

export const data: Database = {
	async roomSelect(id: string) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM rooms WHERE id = ${id}`;
		q.release();
		if (!d.rows[0]) return null;
		return Room.parse(d.rows[0]);
	},
	async roomInsert(id, name, description) {
		const q = await db.connect();
		const d = await q.queryObject`INSERT INTO rooms (id, name, description) VALUES (${id}, ${name}, ${description}) RETURNING *`;
		q.release();
		return Room.parse(d.rows[0]);
  },
  async roomUpdate(id, name, description) {
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const room = await data.roomSelect(id);
		if (!room) {
			await tx.rollback();
			q.release();
			return null;			
		}
		const d = await q.queryObject`
			UPDATE rooms SET
				name = ${name === undefined ? room.name : name},
				description = ${description === undefined ? room.description : description}
			WHERE id = ${id}
			RETURNING *
		`;
		await tx.commit();
		q.release();
		return Room.parse(d.rows[0]);
  },
}
