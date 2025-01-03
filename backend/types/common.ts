import { z } from "npm:@hono/zod-openapi";

export const Uint = z.number().int().nonnegative();

const roomIdExample = "00000000-0000-0000-0000-00000000room";
const userIdExample = "00000000-0000-0000-0000-00000000user";
const threadIdExample = "00000000-0000-0000-0000-000000thread";
const messageIdExample = "00000000-0000-0000-0000-00000message";
const messageVersionIdExample = "00000000-0000-0000-0000-00messagever";
const memberIdExample = "00000000-0000-0000-0000-000000member";
const roleIdExample = "00000000-0000-0000-0000-00000000role";
const sessionIdExample = "00000000-0000-0000-0000-00000session";
const mediaIdExample = "00000000-0000-0000-0000-0000000media";
const auditLogEntryIdExample = "00000000-0000-0000-0000-0auditlogent";
const sessionTokenExample = "super_secret_session_token";

export const RoomId = z.string().uuid().describe(
	"A unique identifier for this Room",
).openapi({ title: "RoomId", example: roomIdExample });
export const UserId = z.string().uuid().describe(
	"A unique identifier for this User",
).openapi({ title: "UserId", example: userIdExample });
export const ThreadId = z.string().uuid().describe(
	"A unique identifier for this Thread",
).openapi({ title: "ThreadId", example: threadIdExample });
export const MessageId = z.string().uuid().describe(
	"A unique identifier for this Message",
).openapi({ title: "MessageId", example: messageIdExample });
export const MessageVersionId = z.string().uuid().describe(
	"A unique identifier for this MessageVersion",
).openapi({ title: "MessageVersionId", example: messageVersionIdExample });
export const MemberId = z.string().uuid().describe(
	"A unique identifier for this Member",
).openapi({ title: "MemberId", example: memberIdExample });
export const RoleId = z.string().uuid().describe(
	"A unique identifier for this Role",
).openapi({ title: "RoleId", example: roleIdExample });
export const SessionId = z.string().uuid().describe(
	"A unique identifier for this Session",
).openapi({ title: "SessionId", example: sessionIdExample });
export const MediaId = z.string().uuid().describe(
	"A unique identifier for this Media",
).openapi({ title: "MediaId", example: mediaIdExample });
export const AuditLogEntryId = z.string().uuid().describe(
	"A unique identifier for this AuditLogEntry",
).openapi({ title: "AuditLogEntry", example: auditLogEntryIdExample });

export const SessionToken = z.string().describe(
	"A secret token that authorizes this session",
).openapi({ title: "SessionToken", example: sessionTokenExample });
export const InviteCode = z.string().openapi({
	title: "InviteCode",
	example: "arstdhneio",
});

// TODO: media. how to implement this?
export const Media = z.object({
	id: MediaId,
	filename: z.string().describe("The original filename"),
	url: z.string().describe("A url to download this media from"),
	source_url: z.string().optional().describe(
		"The source url this media was downloaded from, if any",
	),
	thumbnail_url: z.string().optional().describe("A thumbnail"),
	mime: z.string().describe("The mime type (file type)"),
	alt: z.string().optional().describe(
		"Descriptive alt text, not entirely unlike a caption",
	),
	size: Uint.optional().describe("The size (in bytes)"),
	height: z.number().positive().optional(),
	width: z.number().positive().optional(),
	duration: z.number().positive().optional(),
}).openapi("Media");

export const Embed = z.object({
	title: z.string().max(256).optional(),
	description: z.string().max(8192).optional(),
	thumbnail: Media.optional(),
	media: Media.array().optional(),
});

export const Room = z.object({
	id: RoomId,
	name: z.string().min(1).max(64),
	description: z.string().min(1).max(2048).nullable(),
	// default_roles: RoleId.array(),
}).describe("a room").openapi({
	title: "Room",
	example: {
		id: roomIdExample,
		name: "inspirational quotes",
		description:
			"i expect i'll be able to solve a lot of my problems once my baby brain falls out and my adult brain grows in",
	},
});

// export const RoomDefault = Room.extend({
//   type: z.literal("default"),
// });

// export const RoomDm = Room.extend({
//   type: z.literal("dm"),
//   user_id: UserId,
// });

// export const RoomReport = Room.extend({
//   type: z.literal("report"),
// });

export const UserBase = z.object({
	id: UserId,
	parent_id: UserId.nullable(),
	name: z.string().min(2).max(64),
	description: z.string().max(8192).nullable(),
	status: z.string().max(8192).nullable(),
	// email: z.string().email().optional(),
	// avatar: z.string().url().or(z.literal("")),
	is_bot: z.boolean().describe("is a bot owned by its parent"),
	is_alias: z.boolean().describe("is considered the same user as its parent"),
	is_system: z.boolean().describe("is an official system user"),
});

export const User = UserBase.openapi("User");

export const Member = z.object({
	id: MemberId,
	user_id: UserId,
	room_id: RoomId,
	override_name: z.string().min(2).max(64),
	override_description: z.string().max(8192),
	// override_avatar: z.string().url().or(z.literal("")),
	roles: RoleId.array(),
}).openapi("Member");

export const MessageBase = z.object({
	id: MessageId,
	thread_id: ThreadId,
	version_id: MessageVersionId,
	nonce: z.string().nullish().transform(i => i ?? null),
	ordering: Uint.describe("the order that this message appears in the room"),
	content: z.string().min(1).max(8192).nullable(),
	attachments: Media.array().default([]),
	embeds: Embed.array().default([]),
	metadata: z.record(z.string(), z.any()),
	mentions_users: UserId.array(),
	mentions_roles: RoleId.array(),
	mentions_everyone: z.boolean(),
	reply_id: MessageId.nullable(),
	// resolve everything here?
	// mentions_threads: ThreadId.array(),
	// mentions_rooms: ThreadId.array(),
	author_id: UserId,
	is_pinned: z.boolean(),
});

export const Message = MessageBase
	.openapi({
		title: "Message",
		// example: {
		//   room_id: "01940a32-9b13-75a3-b890-0460b774d52f",
		//   thread_id: "01940a47-b67c-71ea-a040-e00b92ad51ff",
		//   name: "talkin",
		//   description: "i am quite warm an dunpleasant",
		//   closed: false,
		//   locked: false,
		// }
	});

export const ThreadBase = z.object({
	id: ThreadId,
	room_id: RoomId,
	name: z.string().min(1).max(64),
	description: z.string().min(1).max(2048).nullable(),
	is_closed: z.boolean(),
	is_locked: z.boolean(),
	is_pinned: z.boolean(),
	// is_wiki: z.boolean(), // editable by everyone
	// is_private: z.boolean(),
	// recipients: Member.array(),
})

export const Thread = ThreadBase
.openapi({
	title: "Thread",
	example: {
		id: threadIdExample,
		room_id: roomIdExample,
		name: "talkin",
		description: "i am quite warm an dunpleasant",
		is_closed: false,
		is_locked: false,
		is_pinned: false,
	},
});

// export const ThreadText = Thread.extend({
//   type: z.literal("text"),
//   creator: Member,
// });

// export const ThreadTextPrivate = ThreadText.extend({
//   type: z.literal("text-private"),
// });

// export const ThreadReport = ThreadTextPrivate.extend({
//   type: z.literal("report"),
//   reported_item: Room.or(Member).or(User).or(Thread).or(Message).or(Media),
// });

// export const ThreadVoice = Thread.extend({
//   type: z.literal("voice"),
//   call: z.null().describe("todo"),
// });

export const Session = z.object({
	id: SessionId,
	user_id: UserId,
	token: SessionToken,
	status: Uint.max(2).describe(
		"0 = unauthenticated, 1 = can do basic stuff, 2 = sudo mode",
	),
	name: z.string().nullable(),
}).openapi("Session");

export const Role = z.object({
	id: RoleId,
	name: z.string().min(1).max(64),
	description: z.string().max(2048),
	permissions: Uint,
	is_self_applicable: z.boolean(),
	is_mentionable: z.boolean(),
}).openapi("Role");

export const Invite = z.object({
	code: InviteCode,
	// target: z.union([
	// 	z.object({ user: User }),
	// 	z.object({ room: Room }),
	// 	z.object({ thread: Thread }),
	// ]),
	target_id: z.string(),
	target_type: z.enum(["room", "thread", "user"]),
	creator_id: UserId.optional(),
	// roles: RoleId.array().optional(),
	// expires_at: z.date().optional(),
	// max_uses: Uint.optional(),
	// uses: Uint,
}).openapi("Invite");

export const RoomPatch = Room.pick({ name: true, description: true }).partial();
export const ThreadPatch = Thread.pick({
	name: true,
	description: true,
	is_closed: true,
	is_locked: true,
}).partial();
export const MessagePatch = Message.pick({
	content: true,
	attachments: true,
	embeds: true,
	metadata: true,
	reply_id: true,
	nonce: true,
}).partial();
export const UserPatch = User.pick({
	name: true,
	description: true,
	status: true,
	is_alias: true,
	is_bot: true,
}).partial();
export const SessionPatch = Session.pick({ user_id: true, name: true })
	.partial();
export const RolePatch = Role.pick({
	name: true,
	description: true,
	permissions: true,
}).partial();
// export const InvitePatch = Invite.pick({ expires_at: true, max_uses: true })
// 	.partial();
export const InvitePatch = Invite.pick({ })
	.partial();
export const MemberPatch = Member.pick({
	override_name: true,
	override_description: true,
	roles: true,
}).partial();

const permsdesc = `\`\`\`js
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
\`\`\``;

export const Permissions = Uint.describe("permissions bitflags").openapi({
	title: "bitflags",
	description: permsdesc,
});

export const AuditLogEntry = z.object({
	actor: Member,
	endpoint: z.string(),
	method: z.enum(["GET", "POST", "PUT", "PATCH", "DELETE"]),
	body: z.any(),
});

export const AuthorizationRequest = z.union([
	z.object({ type: z.literal("password"), password: z.string() }),
	z.object({ type: z.literal("totp"), code: z.string().regex(/^[0-9]{6}$/) }),
]);
