import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type Thread = components["schemas"]["Thread"];
export type User = components["schemas"]["User"];
export type Message = components["schemas"]["Message"] & { is_local?: true };
export type Role = components["schemas"]["Role"];
export type Invite = components["schemas"]["Invite"];
export type Session = components["schemas"]["Session"];
export type RoomMember = components["schemas"]["RoomMember"];
export type ThreadMember = components["schemas"]["ThreadMember"];
export type Media = components["schemas"]["Media"];
export type MediaTrack = components["schemas"]["MediaTrack"];
export type MessageCreate = components["schemas"]["MessageCreate"];
export type PaginationResponseMessage =
	components["schemas"]["PaginationResponse_Message"];
export type AuditLogEntry = components["schemas"]["AuditLog"];
export type Permission = components["schemas"]["Permission"];
export type Embed = components["schemas"]["UrlEmbed"];
export type TextDocument = components["schemas"]["Document"];
export type EmojiCustom = components["schemas"]["EmojiCustom"];

export type Pagination<T> = {
	total: number;
	items: Array<T>;
	has_more: boolean;
};

export type PaginationQuery = {
	from?: string;
	to?: string;
	limit?: number;
	dir?: "b" | "f";
};

export type MessageReady = {
	op: "Ready";
	user: User | null;
	session: Session;
	conn: string;
	seq: number;
};

export type MessageEnvelope =
	| { op: "Ping" }
	| { op: "Sync"; data: MessageSync; seq: number }
	| { op: "Error"; error: string }
	| MessageReady
	| { op: "Resumed" }
	| { op: "Reconnect"; can_resume: boolean };

export type MessageSync =
	| { type: "InviteDelete"; code: string }
	| { type: "InviteUpsert"; invite: Invite }
	| { type: "MessageDelete"; thread_id: string; message_id: string }
	| { type: "MessageUpsert"; message: Message }
	| {
		type: "MessageVersionDelete";
		thread_id: string;
		message_id: string;
		version_id: string;
	}
	| { type: "RoleDelete"; room_id: string; role_id: string }
	| { type: "RoleUpsert"; role: Role }
	| { type: "RoomMemberDelete"; room_id: string; user_id: string }
	| { type: "RoomMemberUpsert"; member: RoomMember }
	| { type: "RoomUpsert"; room: Room }
	| { type: "SessionDelete"; id: string }
	| { type: "SessionUpsert"; session: Session }
	| { type: "ThreadMemberUpsert"; member: ThreadMember }
	| { type: "ThreadUpsert"; thread: Thread }
	| { type: "Typing"; thread_id: string; user_id: string; until: string }
	| { type: "UserDelete"; id: string }
	| { type: "UserUpsert"; user: User }
	| { type: "VoiceDispatch"; user_id: string; payload: any };
