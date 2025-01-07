import { z } from "npm:@hono/zod-openapi";
import {
Invite,
InviteCode,
Member,
	Message,
	MessageId,
	MessageVersionId,
	Role,
	RoleId,
	Room,
	RoomId,
	Session,
	SessionId,
	Thread,
	ThreadId,
	User,
	UserId,
} from "./common.ts";

export const MessageClient = z.union([
	z.object({
		type: z.literal("hello"),
		token: z.string(),
		last_id: z.string().optional(),
	}),
	z.object({ type: z.literal("pong") }),
]);

// const MessageRoom = z.union([
// ]);

export const MessageServer = z.union([
	z.object({ type: z.literal("ping") }),
	z.object({ type: z.literal("ready"), user: User }),
	z.object({ type: z.literal("error"), error: z.string() }),
	z.object({ type: z.literal("upsert.room"), room: Room }),
	z.object({ type: z.literal("upsert.thread"), thread: Thread }),
	z.object({ type: z.literal("upsert.message"), message: Message }),
	z.object({ type: z.literal("upsert.user"), user: User }),
	z.object({ type: z.literal("upsert.member"), member: Member }),
	z.object({ type: z.literal("upsert.session"), session: Session }),
	z.object({ type: z.literal("upsert.role"), role: Role }),
	z.object({ type: z.literal("upsert.invite"), invite: Invite }),
	// z.object({ type: z.literal("create.ban"), ban: Role }),
	z.object({ type: z.literal("delete.message"), id: MessageId }),
	z.object({ type: z.literal("delete.message_version"), id: MessageVersionId }),
	z.object({ type: z.literal("delete.user"), id: UserId }),
	z.object({ type: z.literal("delete.session"), id: SessionId }),
	z.object({ type: z.literal("delete.role"), id: RoleId }),
	z.object({ type: z.literal("delete.member"), room_id: RoomId, user_id: UserId }),
	z.object({ type: z.literal("delete.invite"), code: InviteCode }),
]);

/*
// expose a sse route per room..? would be nice if there was auth though
// return streamSSE(c, async (stream) => {
//   while (true) {
//     const message = `It is ${new Date().toISOString()}`
//     await stream.writeSSE({
//       data: message,
//       event: 'time-update',
//       id: String(id++),
//     })
//     await stream.sleep(1000)
//   }
// })
*/
