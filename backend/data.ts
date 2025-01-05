// TODO: rename this to globals.ts
import { DB } from "https://deno.land/x/sqlite@v3.9.1/mod.ts";
import { MessageServer } from "./types/sync.ts";
import { z } from "@hono/zod-openapi";
import { Invite, Member, Message, MessagePatch, Permission, Role, RolePatch, Room, Session, SessionPatch, Thread, ThreadPatch, User, UserPatch } from "./types.ts";
// import { RoomFromDb } from "./types/db.ts";
import EventEmitter from "events";
export * as discord from "./oauth2.ts";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

import { Pool, PoolClient, Transaction } from "postgres";
import { uuidv7 } from "uuidv7";
import { MemberFromDb, MessageFromDb, ThreadFromDb, UserFromDb } from "./types/db.ts";
import { UUID_MAX, UUID_MIN } from "./util.ts";
import { AsyncLocalStorage } from "node:async_hooks";

const db = new Pool({
	database: "chat",
	hostname: "localhost",
	port: 5432,
	user: "chat",
	password: "ce00eebd05027ca1",
}, 8);

{
	const migrations = [...Deno.readDirSync("migrations")].sort((a, b) => a.name > b.name ? 1 : -1)
	using q = await db.connect();
	for (const migration of migrations) {
		const sql = await Deno.readTextFile(`migrations/${migration.name}`);
		await q.queryObject(sql);
	}
}

type MsgServer = z.infer<typeof MessageServer>;

type Events = {
	global: (msg: MsgServer) => void;
	rooms: (room_id: string, msg: MsgServer) => void;
	threads: (thread_id: string, msg: MsgServer) => void;
	users: (user_id: string, msg: MsgServer) => void;
};

export const events = new EventEmitter() as TypedEmitter<Events>;

export type HonoEnv = {
	Variables: {
		session_id: string;
		user_id: string;
		// session_level: number;
		room?: RoomT,
		thread?: ThreadT,
		message?: MessageT,
		member?: MemberT,
		member_self?: MemberT,
		user?: UserT,
		user_self?: UserT,
		permissions: Permissions,
	};
};

export enum SessionStatus {
	Unauthorized = 0,
	Default = 1,
	Sudo = 2,
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

export type RoomT = z.infer<typeof Room>;
export type ThreadT = z.infer<typeof Thread>;
export type SessionT = z.infer<typeof Session>;
export type MessageT = z.infer<typeof Message>;
export type UserT = z.infer<typeof User>;
export type UserPatchT = z.infer<typeof UserPatch>;
export type ThreadPatchT = z.infer<typeof ThreadPatch>;
export type MessagePatchT = z.infer<typeof MessagePatch>;
export type SessionPatchT = z.infer<typeof SessionPatch>;
export type MemberT = z.infer<typeof Member>;
export type RoleT = z.infer<typeof Role>;
export type InviteT = z.infer<typeof Invite>;
export type PermissionT = z.infer<typeof Permission>;
export type RolePatchT = z.infer<typeof RolePatch>;

type UserPatchExtraT = {
	parent_id?: string | null,
	discord_id?: string | null,
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

type SessionInsertT = {
  id: string,
  user_id: string,
  token: string,
  status: number,
}

type RolePatchExtraT = {
	id: string,
	room_id: string,
}

type Database = {
	sessionInsert(patch: SessionInsertT): Awaitable<SessionT>;
	sessionSelect(id: string): Awaitable<SessionT | null>;
	sessionSelectByToken(token: string): Awaitable<SessionT | null>;
	sessionDelete(id: string): Awaitable<void>;
	userSelect(id: string): Awaitable<UserT | null>;
	userSelectByDiscordId(id: string): Awaitable<UserT | null>;
	userInsert(id: string, patch: Required<UserPatchT>, extra: Required<UserPatchExtraT>): Awaitable<UserT>;
	userUpdate(id: string, patch: UserPatchT, extra: UserPatchExtraT): Awaitable<UserT | null>;
	userDelete(id: string): Awaitable<void>;
	roomSelect(id: string): Awaitable<RoomT | null>;
	roomInsert(id: string, name: string, description: string | null): Awaitable<RoomT>;
	roomUpdate(id: string, name?: string | null, description?: string | null): Awaitable<RoomT | null>;
	roomList(user_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<RoomT>>;
	threadSelect(id: string): Awaitable<ThreadT | null>;
	threadInsert(id: string, creator_id: string, room_id: string, patch: Required<ThreadPatchT>): Awaitable<ThreadT>;
	threadUpdate(id: string, patch: ThreadPatchT): Awaitable<ThreadT | null>;
	threadList(room_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<ThreadT>>;
	messageInsert(patch: MessagePatchT, extra: MessageExtraPatchT): Awaitable<MessageT>;
	messageList(thread_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<MessageT>>;
	messageSelect(thread_id: string, message_id: string): Awaitable<MessageT | null>;
	memberInsert(user_id: string, base: Omit<MemberT, "user" | "roles">): Awaitable<MemberT>;
	memberSelect(room_id: string, user_id: string): Awaitable<MemberT | null>;
	memberList(room_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<MemberT>>;
	roleApplyInsert(role_id: string, user_id: string): Awaitable<void>;
	roleApplyDelete(role_id: string, user_id: string): Awaitable<void>;
	inviteInsertRoom(room_id: string, creator_id: string, code: string): Awaitable<InviteT>;
	inviteSelect(code: string): Awaitable<InviteT>;
	inviteDelete(code: string): Awaitable<void>;
	inviteList(room_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<InviteT>>;
	roleList(room_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<RoleT>>;
	roleInsert(patch: RolePatchT, extra: RolePatchExtraT): Awaitable<RoleT>;
	roleDelete(room_id: string, role_id: string): Awaitable<void>;
	roleSelect(room_id: string, role_id: string): Awaitable<RoleT | null>;
	roleUpdate(room_id: string, role_id: string, patch: RolePatchT): Awaitable<RoleT | null>;
	permissionReadRoom(user_id: string, room_id: string): Awaitable<Permissions>;
	permissionReadThread(user_id: string, thread_id: string): Awaitable<Permissions>;
}

// app.use("/api/v1/*", async (c, next) => {
// 	using q = await db.connect();
// 	const tx = q.createTransaction(uuidv7());
// 	c.set("tx", tx);
// 	await next();
// 	tx.rollback();
// });

export class Permissions extends Set<PermissionT> {
	override has(perm: PermissionT) {
		if (super.has("Admin")) return true;
		return super.has(perm);
	}

	static none = new Permissions();
}

export const data: Database = {
	async userSelect(id) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM users WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return UserFromDb.parse(d.rows[0]);
	},
	async userSelectByDiscordId(id) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM users WHERE discord_id = ${id}`;
		if (!d.rows[0]) return null;
		return UserFromDb.parse(d.rows[0]);
	},
	async userInsert(id, patch, extra) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO users (id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork, discord_id)
			VALUES (${id}, ${extra.parent_id}, ${patch.name}, ${patch.description}, ${patch.status}, ${patch.is_bot ?? false}, ${patch.is_alias ?? false}, ${extra.is_system ?? false}, ${extra.can_fork ?? false}, ${extra.discord_id})
			RETURNING *
		`;
		return UserFromDb.parse(d.rows[0]);
	},
  async userUpdate(id, patch) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const oldr = await tx.queryObject`SELECT * FROM users WHERE id = ${id}`;
		if (!oldr.rows[0]) {
			await tx.rollback();
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
		return UserFromDb.parse(d.rows[0]);
  },
	async sessionInsert(patch) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO sessions (id, user_id, token, status)
      VALUES (${patch.id}, ${patch.user_id}, ${patch.token}, ${patch.status})
			RETURNING *
		`;
		return Session.parse(d.rows[0]);
	},
	async userDelete(id) {
		using q = await db.connect();
		await q.queryObject`DELETE FROM users WHERE id = ${id}`;
	},
	async sessionDelete(id) {
		using q = await db.connect();
		await q.queryObject`DELETE FROM users WHERE id = ${id}`;
	},
	async roomSelect(id: string) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM rooms WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return Room.parse(d.rows[0]);
	},
	async roomInsert(id, name, description) {
		using q = await db.connect();
		const d = await q.queryObject`
			INSERT INTO rooms (id, name, description) VALUES (${id}, ${name}, ${description}) RETURNING *
		`;
		return Room.parse(d.rows[0]);
  },
  async roomUpdate(id, name, description) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows: [roomr] } = await tx.queryObject`SELECT * FROM rooms WHERE id = ${id}`;
		if (!roomr) {
			await tx.rollback();
			return null;			
		}
		const room = Room.parse(roomr);
		const d = await tx.queryObject`
			UPDATE rooms SET
				name = ${name === undefined ? room.name : name},
				description = ${description === undefined ? room.description : description},
			WHERE id = ${id}
			RETURNING *
		`;
		await tx.commit();
		return Room.parse(d.rows[0]);
  },
  async roomList(user_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT rooms.* FROM room_members
			JOIN rooms ON room_members.room_id = rooms.id
			WHERE room_members.user_id = ${user_id} AND rooms.id > ${after} AND rooms.id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'f' THEN rooms.id END), rooms.id DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM room_members WHERE room_members.user_id = ${user_id}
		`;
		await tx.rollback();
		const items = rows
			.slice(0, limit)
			.map((i) => Room.parse(i));
		if (dir === "b") items.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items,
		};
  },
	async threadSelect(id: string) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM threads WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return ThreadFromDb.parse(d.rows[0]);
	},
	async threadInsert(id, creator_id, room_id, { name, description, is_closed, is_locked }) {
		using q = await db.connect();
		const d = await q.queryObject`
			INSERT INTO threads (id, creator_id, room_id, name, description, is_closed, is_locked)
			VALUES (${id}, ${creator_id}, ${room_id}, ${name}, ${description}, ${is_closed ?? false}, ${is_locked ?? false})
			RETURNING *
		`;
		return ThreadFromDb.parse(d.rows[0]);
  },
	async threadUpdate(id, { name, description, is_closed, is_locked }) {
		using q = await db.connect();
		const tx = q.createTransaction("threadupdate" + uuidv7());
		await tx.begin();
		const { rows: [threadData] } = await tx.queryObject`SELECT * FROM threads WHERE id = ${id}`;
		if (!threadData) {
			await tx.rollback();
			return null;
		}
		const thread = ThreadFromDb.parse(threadData);
		const d = await tx.queryObject`
			UPDATE threads SET
				name = ${name === undefined ? thread.name : name},
				description = ${description === undefined ? thread.description : description},
				is_locked = ${is_locked === undefined ? thread.is_locked : is_locked},
				is_closed = ${is_closed === undefined ? thread.is_closed : is_closed}
			WHERE id = ${id}
			RETURNING *
		`;
		await tx.commit();
		return ThreadFromDb.parse(d.rows[0]);
  },
  async threadList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
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
		await tx.rollback();
		const threads = rows
			.slice(0, limit)
			.map((i) => ThreadFromDb.parse(i));
		if (dir === "b") threads.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items: threads,
		};
  },
  async sessionSelect(id) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM sessions WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async sessionSelectByToken(token) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM sessions WHERE token = ${token}`;
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async messageInsert(patch, extra) {
		using q = await db.connect();
		const d = await q.queryObject`
			WITH inserted AS (
		    INSERT INTO messages (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id)
		    VALUES (${extra.id}, ${extra.thread_id}, ${extra.version_id}, ${extra.ordering}, ${patch.content}, ${patch.metadata}, ${patch.reply_id}, ${extra.author_id})
				RETURNING *
			)
			SELECT inserted.*, row_to_json(users) AS author FROM inserted
			JOIN users ON users.id = inserted.author_id
		`;
		return MessageFromDb.parse(d.rows[0]);
  },
  async messageSelect(thread_id, message_id) {
		using q = await db.connect();
		const { rows } = await q.queryObject`
			SELECT msg.*, row_to_json(users) as author FROM messages AS msg
			JOIN users ON users.id = msg.author_id
			WHERE thread_id = ${thread_id} AND msg.id = ${message_id}
		`;
		if (!rows[0]) return null;
		return MessageFromDb.parse(rows[0]);
  },
  async messageList(thread_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT msg.*, row_to_json(users) as author FROM messages_coalesced AS msg
			JOIN users ON users.id = msg.author_id
			WHERE thread_id = ${thread_id} AND msg.id > ${after} AND msg.id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN msg.id END) DESC, msg.id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM messages_coalesced WHERE thread_id = ${thread_id}
		`;
		await tx.rollback();
		const messages = rows
			.slice(0, limit)
			.map((i) => MessageFromDb.parse(i));
		if (dir === "b") messages.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items: messages,
		};
  },
  async memberInsert(user_id, base) {
		using q = await db.connect();
		await q.queryObject`
      INSERT INTO room_members (user_id, room_id, membership)
			VALUES (${user_id}, ${base.room_id}, ${base.membership})
		`;
		return (await data.memberSelect(base.room_id, user_id))!;
  },
  async memberSelect(room_id, user_id) {
		using q = await db.connect();
		const d = await q.queryObject`
      SELECT * FROM members_json WHERE user_id = ${user_id} AND room_id = ${room_id}
		`;
		if (!d.rows[0]) return null;
		return MemberFromDb.parse(d.rows[0]);
  },
  async roleInsert(base, extra) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO roles (id, room_id, name, description, permissions)
			VALUES (${extra.id}, ${extra.room_id}, ${base.name}, ${base.description}, ${base.permissions ?? []})
			RETURNING *
		`;
		return Role.parse(d.rows[0]);
  },
  async roleApplyInsert(role_id, user_id) {
		using q = await db.connect();
		await q.queryObject`
      INSERT INTO roles_members (user_id, role_id)
			VALUES (${user_id}, ${role_id})
		`;
  },
  async roleApplyDelete(role_id, user_id) {
		using q = await db.connect();
		await q.queryObject`
      DELETE FROM roles_members
			WHERE user_id = ${user_id} AND role_id = ${role_id}
		`;
  },
  async inviteInsertRoom(room_id, creator_id, code) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO invites (target_type, target_id, code, creator_id)
			VALUES ('room', ${room_id}, ${code}, ${creator_id})
			RETURNING *
		`;
		return Invite.parse(d.rows[0]);
  },
  async inviteSelect(code) {
		using q = await db.connect();
		const d = await q.queryObject`
      SELECT * FROM invites WHERE code = ${code}
		`;
		return Invite.parse(d.rows[0]);
  },
  async memberList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
      SELECT * FROM members_json
			WHERE room_id = ${room_id} AND user_id > ${after} AND user_id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN user_id END) DESC, user_id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM members_json WHERE room_id = ${room_id}
		`;
		await tx.rollback();
		const items = rows
			.slice(0, limit)
			.map((i) => Member.parse(i));
		if (dir === "b") items.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items,
		};
  },
  async roleList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
      SELECT * FROM roles
			WHERE room_id = ${room_id} AND id > ${after} AND id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN id END) DESC, id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM roles WHERE room_id = ${room_id}
		`;
		await tx.rollback();
		const items = rows
			.slice(0, limit)
			.map((i) => Role.parse(i));
		if (dir === "b") items.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items,
		};
  },
  async inviteDelete(code) {
		using q = await db.connect();
		await q.queryObject`DELETE FROM invites WHERE code = ${code}`;
  },
  async inviteList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? "";
		const before = (dir === "f" ? to : from) ?? "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~";
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM invites
			WHERE target_type = 'room' AND target_id = ${room_id} AND code::bytea > ${after} AND code::bytea < ${before}
			ORDER BY (CASE WHEN ${dir} = 'f' THEN code::bytea END), code::bytea DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM invites WHERE target_type = 'room' AND target_id = ${room_id}
		`;
		await tx.rollback();
		const items = rows
			.slice(0, limit)
			.map((i) => Invite.parse(i));
		if (dir === "b") items.reverse();
		return {
			has_more: rows.length > limit,
			total: count,
			items,
		};
  },
  async roleDelete(room_id, role_id) {
		using q = await db.connect();
		await q.queryObject`DELETE FROM roles WHERE room_id = ${room_id} AND id = ${role_id}`;
  },
  async roleSelect(room_id, role_id) {
		using q = await db.connect();
		const d = await q.queryObject`
			SELECT * FROM roles WHERE room_id = ${room_id} AND id = ${role_id}
		`;
		return Role.parse(d.rows[0]);
  },
  async roleUpdate(room_id, role_id, patch) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows: [roler] } = await tx.queryObject`SELECT * FROM roles WHERE room_id = ${room_id} AND id = ${role_id}`;
		if (!roler) {
			await tx.rollback();
			return null;			
		}
		const role = Role.parse(roler);
		const d = await tx.queryObject`
			UPDATE rooms SET
				name = ${patch.name === undefined ? role.name : patch.name},
				description = ${patch.description === undefined ? role.description : patch.description},
				permissions = ${patch.permissions === undefined ? role.permissions : patch.permissions}
			WHERE room_id = ${room_id} AND id = ${role_id}
			RETURNING *
		`;
		await tx.commit();
		return Role.parse(d.rows[0]);
  },
  async permissionReadRoom(user_id, room_id) {
		using q = await db.connect();
  	const { rows } = await q.queryObject<{ permission: PermissionT }>`
  		SELECT permission FROM member_permissions
  		WHERE user_id = ${user_id} AND room_id = ${room_id}
		`;
		return new Permissions(rows.map(i => i.permission));
  },
  async permissionReadThread(user_id, thread_id) {
		using q = await db.connect();
  	const { rows } = await q.queryObject<{ permission: PermissionT }>`
  		SELECT permission FROM member_permissions
  		WHERE user_id = ${user_id} AND thread_id = ${thread_id}
		`;
		return new Permissions(rows.map(i => i.permission));
  },
}
