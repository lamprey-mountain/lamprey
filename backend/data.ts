// TODO: rename this to globals.ts
import { DB } from "https://deno.land/x/sqlite@v3.9.1/mod.ts";
import { MessageServer } from "./types/sync.ts";
import { z } from "@hono/zod-openapi";
import { Message, MessagePatch, Room, Session, Thread, ThreadPatch, User, UserPatch } from "./types.ts";
// import { RoomFromDb } from "./types/db.ts";
import EventEmitter from "events";
export * as discord from "./oauth2.ts";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

import { Pool, PoolClient } from "postgres";
import { uuidv7 } from "uuidv7";
import { MessageFromDb, ThreadFromDb, UserFromDb } from "./types/db.ts";
import { UUID_MAX, UUID_MIN } from "./util.ts";

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

type PaginateRequest = {
	dir: "b" | "f",
	limit: number,
	from?: string,
	to?: string,
}

type PaginateResponse<T> = {
	has_more: boolean,
	total: number,
	items: Array<T>,
}

type RoomT = z.infer<typeof Room>;
type ThreadT = z.infer<typeof Thread>;
type SessionT = z.infer<typeof Session>;
type MessageT = z.infer<typeof Message>;
type UserT = z.infer<typeof User>;
type UserPatchT = z.infer<typeof UserPatch>;
type ThreadPatchT = z.infer<typeof ThreadPatch>;
type MessagePatchT = z.infer<typeof MessagePatch>;

type UserPatchExtraT = {
	parent_id?: string,
	is_system?: boolean,
	can_fork?: boolean,
}

type MessageExtraPatchT = {
  id: string,
  thread_id: string,
  version_id: string,
  ordering: number,
  author_id: string,
}

type Database = {
	sessionSelect(id: string): Awaitable<SessionT | null>;
	sessionSelectByToken(token: string): Awaitable<SessionT | null>;
	sessionDelete(id: string): Awaitable<null>;
	userSelect(id: string): Awaitable<UserT | null>;
	userInsert(id: string, patch: Required<UserPatchT>, extra: Required<UserPatchExtraT>): Awaitable<UserT>;
	userUpdate(id: string, patch: UserPatchT, extra: UserPatchExtraT): Awaitable<UserT | null>;
	userDelete(id: string): Awaitable<null>;
	roomSelect(id: string): Awaitable<RoomT | null>;
	roomInsert(id: string, name: string, description: string | null): Awaitable<RoomT>;
	roomUpdate(id: string, name?: string | null, description?: string | null): Awaitable<RoomT | null>;
	roomList(paginate: PaginateRequest): Awaitable<PaginateResponse<RoomT>>;
	threadSelect(id: string): Awaitable<ThreadT | null>;
	threadInsert(id: string, room_id: string, patch: Required<ThreadPatchT>): Awaitable<ThreadT>;
	threadUpdate(id: string, patch: ThreadPatchT): Awaitable<ThreadT | null>;
	threadList(room_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<ThreadT>>;
	messageInsert(patch: MessagePatchT, extra: MessageExtraPatchT): Awaitable<MessageT>;
	messageList(thread_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<MessageT>>;
}

const withDb = async <T>(pool: Pool, closure: (q: PoolClient) => Promise<T>) => {
	const q = await db.connect();
	const d = await closure(q);
	q.release();
	return d;
};

export const data: Database = {
	async userSelect(id) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM users WHERE id = ${id}`;
		q.release();
		if (!d.rows[0]) return null;
		return UserFromDb.parse(d.rows[0]);
	},
	async userInsert(id, patch, extra) {
		const q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO users (id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork)
			VALUES (${id}, ${extra.parent_id}, ${patch.name}, ${patch.description}, ${patch.status}, ${patch.is_bot}, ${patch.is_alias}, ${extra.is_system}, ${extra.can_fork})
			RETURNING *
		`;
		q.release();
		return UserFromDb.parse(d.rows[0]);
	},
  async userUpdate(id, patch) {
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const oldr = await tx.queryObject`SELECT * FROM users WHERE id = ${id}`;
		if (!oldr.rows[0]) {
			await tx.rollback();
			q.release();
			return null;			
		}
		const old = UserFromDb.parse(oldr.rows[0]);
		const d = await tx.queryObject`
			UPDATE users SET
				name = ${patch.name === undefined ? old.name : patch.name},
				description = ${patch.description === undefined ? old.description : patch.description},
				status = ${patch.status === undefined ? old.status : patch.status},
			WHERE id = ${id}
			RETURNING *
		`;
		await tx.commit();
		q.release();
		return UserFromDb.parse(d.rows[0]);
  },
	async userDelete(id) {
		const q = await db.connect();
		await q.queryObject`DELETE FROM users WHERE id = ${id}`;
		q.release();
		return null;
	},
	async sessionDelete(id) {
		const q = await db.connect();
		await q.queryObject`DELETE FROM users WHERE id = ${id}`;
		q.release();
		return null;
	},
	async roomSelect(id: string) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM rooms WHERE id = ${id}`;
		q.release();
		if (!d.rows[0]) return null;
		return Room.parse(d.rows[0]);
	},
	async roomInsert(id, name, description) {
		const q = await db.connect();
		const d = await q.queryObject`
			INSERT INTO rooms (id, name, description) VALUES (${id}, ${name}, ${description}) RETURNING *
		`;
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
		const d = await tx.queryObject`
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
  async roomList({ dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM rooms
			WHERE id > ${after} AND id < ${before}
			ORDER BY id ${dir === "b" ? "DESC" : "ASC"} LIMIT ${limit + 1}
		`;
		const { rows: [count] } = await q.queryObject`
			SELECT count(*) FROM rooms
		`;
		await tx.commit();
		const rooms = rows
			.slice(0, limit)
			.map((i) => Room.parse(i));
		if (dir === "b") rooms.reverse();
		return {
			has_more: rows.length > limit,
			total: count as number,
			items: rooms,
		};
  },
	async threadSelect(id: string) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM threads WHERE id = ${id}`;
		q.release();
		if (!d.rows[0]) return null;
		return ThreadFromDb.parse(d.rows[0]);
	},
	async threadInsert(id, room_id, { name, description, is_closed, is_locked }) {
		const q = await db.connect();
		const d = await q.queryObject`
			INSERT INTO threads (id, room_id, name, description, is_closed, is_locked)
			VALUES (${id}, ${room_id}, ${name}, ${description}, ${is_closed}, ${is_locked})
			RETURNING *
		`;
		q.release();
		return ThreadFromDb.parse(d.rows[0]);
  },
	async threadUpdate(id, { name, description, is_closed, is_locked }) {
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const thread = await data.threadSelect(id);
		if (!thread) {
			await tx.rollback();
			q.release();
			return null;			
		}
		const d = await tx.queryObject`
			UPDATE threads SET
				name = ${name === undefined ? thread.name : name},
				description = ${description === undefined ? thread.description : description}
				is_locked = ${is_locked === undefined ? thread.is_locked : is_locked}
				is_closed = ${is_closed === undefined ? thread.is_closed : is_closed}
			WHERE id = ${id}
			RETURNING *
		`;
		await tx.commit();
		q.release();
		return ThreadFromDb.parse(d.rows[0]);
  },
  async threadList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM threads
			WHERE room_id = ${room_id} AND id > ${after} AND id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'F' THEN id END), id DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM threads WHERE room_id = ${room_id}
		`;
		await tx.commit();
		const threads = rows
			.slice(0, limit)
			.map((i) => ThreadFromDb.parse(i));
		if (dir === "b") threads.reverse();
		return {
			has_more: rows.length > limit,
			total: count as number,
			items: threads,
		};
  },
  async sessionSelect(id) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM sessions WHERE id = ${id}`;
		q.release();
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async sessionSelectByToken(token) {
		const q = await db.connect();
		const d = await q.queryObject`SELECT * FROM sessions WHERE token = ${token}`;
		q.release();
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async messageInsert(patch, extra) {
		const q = await db.connect();
		const d = await q.queryObject`
	    INSERT INTO messages (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id)
	    VALUES (${extra.id}, ${extra.thread_id}, ${extra.version_id}, ${extra.ordering}, ${patch.content}, ${patch.metadata}, ${patch.reply_id}, ${extra.author_id})
	    RETURNING *
		`;
		q.release();
		return MessageFromDb.parse(d.rows[0]);
  },
  async messageList(thread_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		const q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM messages_coalesced
			WHERE thread_id = ${thread_id} AND id > ${after} AND id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN id END) DESC, id LIMIT ${limit + 1}
		`;
		const { rows: [count] } = await tx.queryObject`
			SELECT count(*)::int FROM messages_coalesced WHERE thread_id = ${thread_id}
		`;
		await tx.commit();
		const messages = rows
			.slice(0, limit)
			.map((i) => MessageFromDb.parse(i));
		if (dir === "b") messages.reverse();
		return {
			has_more: rows.length > limit,
			total: count as number,
			items: messages,
		};
  },
}
