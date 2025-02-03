import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type Thread = components["schemas"]["Thread"];
export type User = components["schemas"]["User"];
export type Message = components["schemas"]["Message"] & { is_local?: true };
export type Role = components["schemas"]["Role"];
export type Invite = components["schemas"]["Invite"];
export type Session = components["schemas"]["Session"];
export type RoomMember = components["schemas"]["RoomMember"];
export type Media = components["schemas"]["Media"];
export type MessageCreate = components["schemas"]["MessageCreateRequest"];
export type PaginationResponseMessage =
	components["schemas"]["PaginationResponse_Message"];

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
	| { type: "UpsertRoom"; room: Room }
	| { type: "UpsertThread"; thread: Thread }
	| { type: "UpsertMessage"; message: Message }
	| { type: "UpsertUser"; user: User }
	| { type: "UpsertMember"; member: RoomMember }
	| { type: "UpsertSession"; session: Session }
	| { type: "UpsertRole"; role: Role }
	| { type: "UpsertInvite"; invite: Invite }
	| { type: "DeleteMessage"; thread_id: string; message_id: string }
	| {
		type: "DeleteMessageVersion";
		thread_id: string;
		message_id: string;
		version_id: string;
	}
	| { type: "DeleteUser"; id: string }
	| { type: "DeleteSession"; id: string }
	| { type: "DeleteRole"; room_id: string; role_id: string }
	| { type: "DeleteRoomMember"; room_id: string; user_id: string }
	| { type: "DeleteInvite"; code: string }
	| { type: "Webhook"; hook_id: string; data: unknown };
