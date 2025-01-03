import { z } from "npm:@hono/zod-openapi";
import {
	Embed,
	Media,
	Message,
	MessageBase,
	MessageId,
	MessageVersionId,
	RoleId,
	RoomId,
	Thread,
	ThreadBase,
	ThreadId,
	Uint,
	User,
	UserBase,
	UserId,
} from "./common.ts";

export const MessageFromDb = MessageBase.extend({
	// metadata: z.string().transform((i) =>
	// 	z.record(z.string(), z.any()).parse(JSON.parse(i))
	// ),
	// resolve everything here?
	mentions_users: UserId.array().default([]),
	mentions_roles: RoleId.array().default([]),
	mentions_everyone: z.boolean().default(false),
	mentions_threads: ThreadId.array().default([]),
	mentions_rooms: ThreadId.array().default([]),
	is_pinned: z.boolean().default(false),
	nonce: z.undefined().transform((_) => null),
});

export const ThreadFromDb = ThreadBase.extend({
	is_closed: z.number().transform((i) => !!i),
	is_locked: z.number().transform((i) => !!i),
	is_pinned: z.number().default(0).transform((i) => !!i), // TODO: impl pins
});

export const UserFromDb = UserBase.extend({
	is_bot: z.number().transform((i) => !!i),
	is_alias: z.number().transform((i) => !!i),
	is_system: z.number().transform((i) => !!i),
});
