// TODO: rename this to globals.ts
import { MessageServer } from "./types/sync.ts";
import { z } from "@hono/zod-openapi";
import { Invite, Media, Member, Message, MessagePatch, MessageType, Permission, Role, RolePatch, Room, Session, SessionPatch, Thread, ThreadPatch, ThreadType, User, UserPatch } from "./types.ts";
import EventEmitter from "node:events";
// import { Client as S3Client } from "npm:minio";
export * as discord from "./oauth2.ts";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

import { Pool } from "postgres";
import { uuidv7 } from "uuidv7";
import { MemberFromDb, MessageFromDb, ThreadFromDb, UserFromDb } from "./types/db.ts";
import { UUID_MAX, UUID_MIN } from "./util.ts";

const db = new Pool({
	database: Deno.env.get("PG_DATABASE")!,
	hostname: Deno.env.get("PG_HOSTNAME")!,
	port: Deno.env.get("PG_PORT")!,
	user: Deno.env.get("PG_USER")!,
	password: Deno.env.get("PG_PASSWORD")!,
	controls: {
		debug: {
			notices: true,
			// queries: true,
			queryInError: true,
			// results: true,
		}
	}
}, 8);

{
	const migrations = [...Deno.readDirSync("migrations")].sort((a, b) => a.name > b.name ? 1 : -1)
	using q = await db.connect();
	for (const migration of migrations) {
		console.log(`migrate ${migration.name}`);
		const sql = await Deno.readTextFile(`migrations/${migration.name}`);
		await q.queryObject(sql);
	}
}

import { S3Client } from "jsr:@bradenmacdonald/s3-lite-client";

const s3 = new S3Client({
	endPoint: Deno.env.get("S3_ENDPOINT")!,
	useSSL: Deno.env.get("S3_USESSL")! === "false" ? false : true,
	region: Deno.env.get("S3_REGION")!,
	accessKey: Deno.env.get("S3_ACCESSKEY")!,
	secretKey: Deno.env.get("S3_SECRETKEY")!,
	bucket: Deno.env.get("S3_BUCKET")!,
});

const S3_PRESIGNED_DURATION = 60 * 60 * 24;

type Blobs = {
	presignedGetUrl(path: string): Promise<string>,
	copyObject(from: string, to: string): Promise<void>,
	deleteObject(path: string): Promise<void>,
	putObject(path: string, data: ReadableStream): Promise<void>,
}

export const blobs: Blobs = {
	presignedGetUrl(path) {
		return s3.presignedGetObject(path, { expirySeconds: S3_PRESIGNED_DURATION });
  },
	async copyObject(from, to) {
		await s3.copyObject({ sourceKey: from }, to);
  },
	async deleteObject(path) {
		await s3.deleteObject(path);
  },
	async putObject(path, stream) {
		await s3.putObject(path, stream);
  },
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
		session_status: number;
		user_id: string;
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
export type MediaT = z.infer<typeof Media>;

type UserPatchExtraT = {
	parent_id?: string | null,
	discord_id?: string | null,
	is_system?: boolean,
	can_fork?: boolean,
}

type MessagePatchExtraT = {
  type: MessageType,
  id: string,
  thread_id: string,
  version_id: string,
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
	sessionInsert(patch: SessionInsertT): Awaitable<SessionT & { token: string }>;
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
	threadSelect(id: string, user_id: string): Awaitable<ThreadT | null>;
	threadInsert(id: string, creator_id: string, room_id: string, ttype: ThreadType, patch: Required<ThreadPatchT>): Awaitable<ThreadT>;
	threadUpdate(id: string, user_id: string, patch: ThreadPatchT): Awaitable<ThreadT | null>;
	threadList(room_id: string, user_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<ThreadT>>;
	messageInsert(patch: MessagePatchT, extra: MessagePatchExtraT): Awaitable<MessageT>;
	messageList(thread_id: string, paginate: PaginateRequest): Awaitable<PaginateResponse<MessageT>>;
	messageSelect(thread_id: string, message_id: string): Awaitable<MessageT | null>;
	memberInsert(user_id: string, base: Omit<MemberT, "user" | "roles">): Awaitable<MemberT>;
	memberSelect(room_id: string, user_id: string): Awaitable<MemberT | null>;
	memberDelete(room_id: string, user_id: string): Awaitable<void>;
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
	applyDefaultRoles(user_id: string, room_id: string): Awaitable<void>;
	mediaInsert(user_id: string, media: MediaT): Awaitable<MediaT>;
	mediaLinkInsert(media_id: string, thing_id: string): Awaitable<void>;
	mediaLinkSelect(media_id: string): Awaitable<Array<string>>;
	unreadMarkThread(user_id: string, thread_id: string): Awaitable<void>;
	unreadMarkMessage(user_id: string, thread_id: string, version_id: string): Awaitable<void>;
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
		const d = await q.queryObject`SELECT * FROM usr WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return UserFromDb.parse(d.rows[0]);
	},
	async userSelectByDiscordId(id) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM usr WHERE discord_id = ${id}`;
		if (!d.rows[0]) return null;
		return UserFromDb.parse(d.rows[0]);
	},
	async userInsert(id, patch, extra) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO usr (id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork, discord_id)
			VALUES (${id}, ${extra.parent_id}, ${patch.name}, ${patch.description}, ${patch.status}, ${patch.is_bot ?? false}, ${patch.is_alias ?? false}, ${extra.is_system ?? false}, ${extra.can_fork ?? false}, ${extra.discord_id})
			RETURNING *
		`;
		return UserFromDb.parse(d.rows[0]);
	},
  async userUpdate(id, patch) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const oldr = await tx.queryObject`SELECT * FROM usr WHERE id = ${id}`;
		if (!oldr.rows[0]) {
			await tx.rollback();
			return null;			
		}
		const old = UserFromDb.parse(oldr.rows[0]);
		const d = await tx.queryObject`
			UPDATE usr SET
				name = ${patch.name === undefined ? old.name : patch.name},
				description = ${patch.description === undefined ? old.description : patch.description},
				status = ${patch.status === undefined ? old.status : patch.status}
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
		return { ...Session.parse(d.rows[0]), token: patch.token };
	},
	async userDelete(id) {
		using q = await db.connect();
		await q.queryObject`UPDATE usr SET deleted_at = ${Date.now()} WHERE id = ${id}`;
	},
	async sessionDelete(id) {
		using q = await db.connect();
		await q.queryObject`DELETE FROM session WHERE id = ${id}`;
	},
	async roomSelect(id: string) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM room WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return Room.parse(d.rows[0]);
	},
	async roomInsert(id, name, description) {
		using q = await db.connect();
		const d = await q.queryObject`
			INSERT INTO room (id, name, description) VALUES (${id}, ${name}, ${description}) RETURNING *
		`;
		return Room.parse(d.rows[0]);
  },
  async roomUpdate(id, name, description) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows: [roomr] } = await tx.queryObject`SELECT * FROM room WHERE id = ${id}`;
		if (!roomr) {
			await tx.rollback();
			return null;			
		}
		const room = Room.parse(roomr);
		const d = await tx.queryObject`
			UPDATE room SET
				name = ${name === undefined ? room.name : name},
				description = ${description === undefined ? room.description : description}
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
			SELECT room.* FROM room_member
			JOIN room ON room_member.room_id = room.id
			WHERE room_member.user_id = ${user_id} AND room.id > ${after} AND room.id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'f' THEN room.id END), room.id DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM room_member WHERE room_member.user_id = ${user_id}
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
	async threadSelect(id: string, user_id: string) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM thread_json WHERE id = ${id} and user_id = ${user_id}`;
		if (!d.rows[0]) return null;
		return ThreadFromDb.parse(d.rows[0]);
	},
	async threadInsert(id, creator_id, room_id, ttype, { name, description, is_closed, is_locked }) {
		using q = await db.connect();
		await q.queryObject`
			INSERT INTO thread (id, creator_id, room_id, name, description, is_closed, is_locked, type)
			VALUES (${id}, ${creator_id}, ${room_id}, ${name}, ${description}, ${is_closed ?? false}, ${is_locked ?? false}, ${ttype})
		`;
		const d = await q.queryObject`
			SELECT * FROM thread_json WHERE id = ${id} and user_id = ${creator_id}
		`;
		return ThreadFromDb.parse(d.rows[0]);
  },
	async threadUpdate(id, user_id, { name, description, is_closed, is_locked }) {
		using q = await db.connect();
		const tx = q.createTransaction("threadupdate" + uuidv7());
		await tx.begin();
		const { rows: [threadData] } = await tx.queryObject`SELECT * FROM thread_json WHERE id = ${id} AND user_id = ${user_id}`;
		if (!threadData) {
			await tx.rollback();
			return null;
		}
		const thread = ThreadFromDb.parse(threadData);
		await tx.queryObject`
			UPDATE thread SET
				name = ${name === undefined ? thread.name : name},
				description = ${description === undefined ? thread.description : description},
				is_locked = ${is_locked === undefined ? thread.is_locked : is_locked},
				is_closed = ${is_closed === undefined ? thread.is_closed : is_closed}
			WHERE id = ${id}
		`;
		const { rows: [newThreadData] } = await tx.queryObject`SELECT * FROM thread_json WHERE id = ${id} AND user_id = ${user_id}`;
		await tx.commit();
		return ThreadFromDb.parse(newThreadData);
  },
  async threadList(room_id, user_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? UUID_MIN;
		const before = (dir === "f" ? to : from) ?? UUID_MAX;
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM thread_json
			WHERE room_id = ${room_id} AND user_id = ${user_id} AND id > ${after} AND id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'F' THEN id END), id DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM thread_json WHERE room_id = ${room_id} AND user_id = ${user_id}
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
		const d = await q.queryObject`SELECT * FROM session WHERE id = ${id}`;
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async sessionSelectByToken(token) {
		using q = await db.connect();
		const d = await q.queryObject`SELECT * FROM session WHERE token = ${token}`;
		if (!d.rows[0]) return null;
		return Session.parse(d.rows[0]);
  },
  async messageInsert(patch, extra) {
		using q = await db.connect();
		await q.queryObject`
	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
	    VALUES (${extra.id}, ${extra.thread_id}, ${extra.version_id}, (SELECT coalesce(count, 0) FROM message_count WHERE thread_id = ${extra.thread_id}), ${patch.content}, ${patch.metadata}, ${patch.reply_id}, ${extra.author_id}, ${extra.type}, ${patch.override_name}, ${patch.attachments?.map(i => i.id) ?? []})
		`;
		const d = await q.queryObject`
	    SELECT * FROM message_json WHERE id = ${extra.id}
		`;
		return MessageFromDb.parse(d.rows[0]);
  },
  async messageSelect(thread_id, message_id) {
		using q = await db.connect();
		const { rows } = await q.queryObject`
			SELECT * FROM message_json AS msg
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
			SELECT * FROM message_json AS msg
			WHERE thread_id = ${thread_id} AND msg.id > ${after} AND msg.id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN msg.id END) DESC, msg.id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM message_coalesced WHERE thread_id = ${thread_id}
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
      INSERT INTO room_member (user_id, room_id, membership)
			VALUES (${user_id}, ${base.room_id}, ${base.membership})
		`;
		return (await data.memberSelect(base.room_id, user_id))!;
  },
  async memberSelect(room_id, user_id) {
		using q = await db.connect();
		const d = await q.queryObject`
      SELECT * FROM member_json WHERE user_id = ${user_id} AND room_id = ${room_id}
		`;
		if (!d.rows[0]) return null;
		return MemberFromDb.parse(d.rows[0]);
  },
  async roleInsert(base, extra) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO role (id, room_id, name, description, permissions, is_mentionable, is_self_applicable, is_default)
			VALUES (${extra.id}, ${extra.room_id}, ${base.name}, ${base.description}, ${base.permissions ?? []}, ${base.is_mentionable ?? false}, ${base.is_self_applicable ?? false}, ${base.is_default ?? false})
			RETURNING *
		`;
		return Role.parse(d.rows[0]);
  },
  async roleApplyInsert(role_id, user_id) {
		using q = await db.connect();
		await q.queryObject`
      INSERT INTO role_member (user_id, role_id)
			VALUES (${user_id}, ${role_id})
		`;
  },
  async roleApplyDelete(role_id, user_id) {
		using q = await db.connect();
		await q.queryObject`
      DELETE FROM role_member
			WHERE user_id = ${user_id} AND role_id = ${role_id}
		`;
  },
  async inviteInsertRoom(room_id, creator_id, code) {
		using q = await db.connect();
		const d = await q.queryObject`
      INSERT INTO invite (target_type, target_id, code, creator_id)
			VALUES ('room', ${room_id}, ${code}, ${creator_id})
			RETURNING *
		`;
		return Invite.parse(d.rows[0]);
  },
  async inviteSelect(code) {
		using q = await db.connect();
		const d = await q.queryObject`
      SELECT * FROM invite WHERE code = ${code}
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
      SELECT * FROM member_json
			WHERE room_id = ${room_id} AND user_id > ${after} AND user_id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN user_id END) DESC, user_id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM member_json WHERE room_id = ${room_id}
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
      SELECT * FROM role
			WHERE room_id = ${room_id} AND id > ${after} AND id < ${before}
			ORDER BY (CASE WHEN ${dir} = 'b' THEN id END) DESC, id LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM role WHERE room_id = ${room_id}
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
		await q.queryObject`DELETE FROM invite WHERE code = ${code}`;
  },
  async inviteList(room_id, { dir, from, to, limit }) {
		const after = (dir === "f" ? from : to) ?? "";
		const before = (dir === "f" ? to : from) ?? "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~";
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows } = await tx.queryObject`
			SELECT * FROM invite
			WHERE target_type = 'room' AND target_id = ${room_id} AND code::bytea > ${after} AND code::bytea < ${before}
			ORDER BY (CASE WHEN ${dir} = 'f' THEN code::bytea END), code::bytea DESC LIMIT ${limit + 1}
		`;
		const { rows: [{ count }] } = await tx.queryObject<{ count: number }>`
			SELECT count(*)::int FROM invite WHERE target_type = 'room' AND target_id = ${room_id}
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
		await q.queryObject`DELETE FROM role WHERE room_id = ${room_id} AND id = ${role_id}`;
  },
  async roleSelect(room_id, role_id) {
		using q = await db.connect();
		const d = await q.queryObject`
			SELECT * FROM role WHERE room_id = ${room_id} AND id = ${role_id}
		`;
		return Role.parse(d.rows[0]);
  },
  async roleUpdate(room_id, role_id, patch) {
		using q = await db.connect();
		const tx = q.createTransaction(uuidv7());
		await tx.begin();
		const { rows: [roler] } = await tx.queryObject`SELECT * FROM role WHERE room_id = ${room_id} AND id = ${role_id}`;
		if (!roler) {
			await tx.rollback();
			return null;			
		}
		const role = Role.parse(roler);
		const d = await tx.queryObject`
			UPDATE role SET
				name = ${patch.name === undefined ? role.name : patch.name},
				description = ${patch.description === undefined ? role.description : patch.description},
				permissions = ${patch.permissions === undefined ? role.permissions : patch.permissions},
				is_mentionable = ${patch.is_mentionable === undefined ? role.is_mentionable : patch.is_mentionable},
				is_self_applicable = ${patch.is_self_applicable === undefined ? role.is_self_applicable : patch.is_self_applicable},
				is_default = ${patch.is_default === undefined ? role.is_default : patch.is_default}
			WHERE room_id = ${room_id} AND id = ${role_id}
			RETURNING *
		`;
		await tx.commit();
		return Role.parse(d.rows[0]);
  },
  async permissionReadRoom(user_id, room_id) {
		using q = await db.connect();
  	const { rows } = await q.queryObject<{ permission: PermissionT }>`
  		SELECT DISTINCT permission FROM room_member_permission
  		WHERE user_id = ${user_id} AND room_id = ${room_id}
		`;
		return new Permissions(rows.map(i => i.permission));
  },
  async permissionReadThread(user_id, thread_id) {
		using q = await db.connect();
  	const { rows } = await q.queryObject<{ permission: PermissionT }>`
  		SELECT DISTINCT permission FROM thread_member_permission
  		WHERE user_id = ${user_id} AND thread_id = ${thread_id}
		`;
		return new Permissions(rows.map(i => i.permission));
  },
  async memberDelete(room_id, user_id) {
		using q = await db.connect();
  	await q.queryObject<{ permission: PermissionT }>`
  		DELETE FROM room_member
  		WHERE user_id = ${user_id} AND room_id = ${room_id}
		`;
  },
  async applyDefaultRoles(user_id, room_id) {
		using q = await db.connect();
  	await q.queryObject<{ permission: PermissionT }>`
	  	INSERT INTO role_member (user_id, role_id)
	  	SELECT ${user_id} as u, id FROM role
	  	WHERE room_id = ${room_id} AND is_default = true;
		`;
  },
  async mediaInsert(user_id, { id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration }) {
		using q = await db.connect();
  	const { rows: [media] } = await q.queryObject`
	    INSERT INTO media (id, user_id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration)
	    VALUES (${id}, ${user_id}, ${url}, ${source_url}, ${thumbnail_url}, ${filename}, ${alt}, ${size}, ${mime}, ${height}, ${width}, ${duration})
	    RETURNING *
		`;
		return Media.parse(media);
  },
  async mediaLinkInsert(media_id, target_id) {
		using q = await db.connect();
  	await q.queryObject`
	    INSERT INTO media_link (media_id, target_id)
	    VALUES (${media_id}, ${target_id})
	    RETURNING *
		`;
  },
  async mediaLinkSelect(media_id) {
		using q = await db.connect();
  	const { rows } = await q.queryObject<{ target_id: string }>`
	    SELECT target_id FROM media_link WHERE media_id = ${media_id}
		`;
		return rows.map(i => i.target_id) ?? [];
  },
  async unreadMarkThread(user_id, thread_id) {
		using q = await db.connect();
  	await q.queryObject`
			INSERT INTO unread (thread_id, user_id, version_id)
			VALUES (${thread_id}, ${user_id}, (SELECT max(version_id) FROM message WHERE thread_id = ${thread_id}))
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET version_id = excluded.version_id;
		`;
  },
  async unreadMarkMessage(user_id, thread_id, version_id) {
		using q = await db.connect();
  	await q.queryObject`
			INSERT INTO unread (thread_id, user_id, version_id)
			VALUES (${thread_id}, ${user_id}, ${version_id})
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET version_id = excluded.version_id;
		`;
  },
}
