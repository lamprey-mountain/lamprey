import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type Channel = components["schemas"]["Channel"];
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
export type PermissionOverwrite = components["schemas"]["PermissionOverwrite"];
export type Embed = components["schemas"]["Embed"];
export type EmojiCustom = components["schemas"]["EmojiCustom"];
export type RelationshipWithUserId =
	components["schemas"]["RelationshipWithUserId"];
export type UserWithRelationship =
	components["schemas"]["UserWithRelationship"];
export type UserConfig = components["schemas"]["UserConfigGlobal"];
export type Application = components["schemas"]["Application"];
export type RoomMemberOrigin = components["schemas"]["RoomMemberOrigin"];
export type MessageSync = components["schemas"]["MessageSync"];
export type RoomBan = components["schemas"]["RoomBan"];
export type Notification = components["schemas"]["Notification"];

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

export type TrackMetadata = {
	mid: string;
	kind: "Audio" | "Video";
	key: string;
};

export type SignallingMessage =
	| {
		type: "Ready";
	}
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
		state: {
			thread_id: string;
			self_mute: boolean;
			self_deaf: boolean;
			self_video: boolean;
			self_screen: boolean;
		} | null;
	}
	| {
		type: "Reconnect";
	};

export type VoiceState = {
	user_id: string;
	thread_id: string;
	session_id: string | null;
	joined_at: string;
	mute: boolean;
	deaf: boolean;
	self_mute: boolean;
	self_deaf: boolean;
	self_video: boolean;
	self_screen: boolean;
};
