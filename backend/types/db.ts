import { z } from "npm:@hono/zod-openapi";
import { Embed, Media, MessageId, MessageVersionId, RoleId, RoomId, ThreadId, Uint, User, UserId } from "./common.ts";

export const MessageFromDb = z.object({
  room_id: RoomId,
  thread_id: ThreadId,
  message_id: MessageId,
  version_id: MessageVersionId,
  edited_at: z.date().optional(),
  content: z.string().max(8192),
  attachments: Media.array().default([]),
  embeds: Embed.array().default([]),
  reply: MessageId.nullable(),
  metadata: z.string().transform(i => z.record(z.string(), z.any()).parse(JSON.parse(i))),
  mentions_users: UserId.array().default([]),
  mentions_roles: RoleId.array().default([]),
  mentions_everyone: z.boolean().default(false),
  // resolve everything here?
  mentions_threads: ThreadId.array().default([]),
  mentions_rooms: ThreadId.array().default([]),
  author_id: UserId,
  is_pinned: z.boolean().default(false),
  nonce: z.undefined().transform(_ => null),
  ordering: Uint.describe("the order that this message appears in the room"),
});

export const ThreadFromDb = z.object({
  room_id: RoomId,
  thread_id: ThreadId,
  name: z.string().min(1).max(64),
  description: z.string().max(2048).nullable(),
  is_closed: z.number().transform(i => !!i),
  is_locked: z.number().transform(i => !!i),
  is_pinned: z.number().default(0).transform(i => !!i), // TODO
});

export const UserFromDb = User.extend({
  is_bot: z.number().transform(i => !!i),
  is_alias: z.number().transform(i => !!i),
  is_system: z.number().transform(i => !!i),
}).openapi("User");

