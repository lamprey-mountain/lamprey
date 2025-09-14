import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type Thread = components["schemas"]["Thread"];
export type User = components["schemas"]["User"];
export type Message = components["schemas"]["Message"] & { is_local?: true };
export type Role = components["schemas"]["Role"];
export type Invite = components["schemas"]["Invite"];
export type InviteWithMetadata = components["schemas"]["InviteWithMetadata"];
export type Session = components["schemas"]["Session"];
export type RoomMember = components["schemas"]["RoomMember"];
export type ThreadMember = components["schemas"]["ThreadMember"];
export type Media = components["schemas"]["Media"];
export type MediaTrack = components["schemas"]["MediaTrack"];
export type MessageCreate = components["schemas"]["MessageCreate"];
export type PaginationResponseMessage =
	components["schemas"]["PaginationResponse_Message"];
export type AuditLogEntry = components["schemas"]["AuditLogEntry"];
export type AuditLogChange = components["schemas"]["AuditLogChange"];
export type Permission = components["schemas"]["Permission"];
export type Embed = components["schemas"]["Embed"];
export type EmojiCustom = components["schemas"]["EmojiCustom"];
export type RelationshipWithUserId =
	components["schemas"]["RelationshipWithUserId"];
export type UserWithRelationship =
	components["schemas"]["UserWithRelationship"];
export type UserConfig = components["schemas"]["UserConfig"];
export type Application = components["schemas"]["Application"];

export type OauthInfo = {
	application: Application;
	bot_user: User;
	auth_user: User;
	authorized: boolean;
};

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
	| { type: "RoomCreate"; room: Room }
	| { type: "RoomUpdate"; room: Room }
	| { type: "ThreadCreate"; thread: Thread }
	| { type: "ThreadUpdate"; thread: Thread }
	| { type: "ThreadTyping"; thread_id: string; user_id: string; until: string }
	| {
		type: "ThreadAck";
		thread_id: string;
		message_id: string;
		version_id: string;
	}
	| { type: "MessageCreate"; message: Message }
	| { type: "MessageUpdate"; message: Message }
	| {
		type: "MessageDelete";
		room_id?: string; // deprecated
		thread_id: string;
		message_id: string;
	}
	| {
		type: "MessageVersionDelete";
		room_id?: string; // deprecated
		thread_id: string;
		message_id: string;
		version_id: string;
	}
	| { type: "MessageDeleteBulk"; thread_id: string; message_ids: string[] }
	| { type: "RoomMemberUpsert"; member: RoomMember }
	| { type: "ThreadMemberUpsert"; member: ThreadMember }
	| { type: "RoleCreate"; role: Role }
	| { type: "RoleUpdate"; role: Role }
	| { type: "RoleDelete"; room_id: string; role_id: string }
	| { type: "InviteCreate"; invite: InviteWithMetadata }
	| { type: "InviteUpdate"; invite: InviteWithMetadata }
	| { type: "InviteDelete"; code: string; target: string }
	| {
		type: "ReactionCreate";
		user_id: string;
		thread_id: string;
		message_id: string;
		key: string;
	}
	| {
		type: "ReactionDelete";
		user_id: string;
		thread_id: string;
		message_id: string;
		key: string;
	}
	| { type: "ReactionPurge"; thread_id: string; message_id: string }
	| { type: "EmojiCreate"; emoji: EmojiCustom }
	| { type: "EmojiDelete"; emoji_id: string; room_id: string }
	| { type: "VoiceDispatch"; user_id: string; payload: any }
	| { type: "VoiceState"; user_id: string; state: any }
	| { type: "UserCreate"; user: User }
	| { type: "UserUpdate"; user: User }
	| { type: "UserConfig"; user_id: string; config: any }
	| { type: "UserDelete"; id: string }
	| { type: "SessionCreate"; session: Session }
	| { type: "SessionUpdate"; session: Session }
	| { type: "SessionDelete"; id: string; user_id?: string }
	| { type: "RelationshipUpsert"; user_id: string; relationship: unknown }
	| { type: "RelationshipDelete"; user_id: string };

export type TrackMetadata = {
	mid: string;
	kind: "Audio" | "Video";
	key: string;
};

export type SignallingMessage =
	| {
		type: "Offer";
		sdp: string;
		tracks: TrackMetadata[];
	}
	| {
		type: "Answer";
		sdp: string;
	}
	| {
		type: "Candidate";
		candidate: string;
	}
	| {
		// only sent by the server
		type: "Have";
		thread_id: string;
		user_id: string;
		tracks: TrackMetadata[];
	}
	| {
		type: "Want";
		tracks: string[];
	}
	| {
		// only sent from client
		// TODO: move this to a top level event
		type: "VoiceState";
		state: { thread_id: string } | null;
	}
	| {
		type: "Reconnect";
	};

export type VoiceState = {
	user_id: string;
	thread_id: string;
	joined_at: string;
};
