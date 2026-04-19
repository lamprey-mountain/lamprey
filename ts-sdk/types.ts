import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type RolePatch = components["schemas"]["RolePatch"];
export type Channel = components["schemas"]["Channel"];
export type ChannelType = components["schemas"]["ChannelType"];
export type User = components["schemas"]["User"] & {
	/** @description relationship with current user (for UserWithRelationship endpoints) */
	relationship?: components["schemas"]["Relationship"];
};
export type Message = components["schemas"]["Message"] & {
	/** @description mentions parsed from message content */
	mentions?: components["schemas"]["Mentions"];
	/** @description mark as local message */
	is_local?: true;
	/** @description idempotency key nonce (client-side only) */
	nonce?: string;
	/** @description lamprey components (for flumes) */
	components?: LampreyComponent[];
};
export type MessageVersion = components["schemas"]["MessageVersion"];
export type Role = components["schemas"]["Role"];
export type Invite = components["schemas"]["Invite"];
export type InviteWithMetadata = components["schemas"]["InviteWithMetadata"];
export type Session = components["schemas"]["Session"];
export type RoomMember = components["schemas"]["RoomMember"];
export type RoomMemberSearchResponse =
	components["schemas"]["RoomMemberSearchResponse"];
export type ThreadMember = components["schemas"]["ThreadMember"];
export type Media = components["schemas"]["Media"];
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
export type Relationship = components["schemas"]["Relationship"] & {
	/** @description user note for relationship (client-side extension) */
	note?: string | null;
	/** @description petname for relationship (client-side extension) */
	petname?: string | null;
};
export type Ignore = components["schemas"]["Ignore"];
export type UserWithRelationship =
	components["schemas"]["UserWithRelationship"] & {
		/** @description relationship with current user (with client-side extensions) */
		relationship: Relationship;
	};
export type Preferences = components["schemas"]["PreferencesGlobal"];
export type PreferencesGlobal = components["schemas"]["PreferencesGlobal"];
export type PreferencesUser = components["schemas"]["PreferencesUser"];
export type PreferencesRoom = components["schemas"]["PreferencesRoom"];
export type PreferencesChannel = components["schemas"]["PreferencesChannel"];
export type Application = components["schemas"]["Application"] & {
	/** @description application avatar (not in canonical schema but used in frontend) */
	avatar?: components["schemas"]["Id"];
	/** @description whether this is a bot */
	bot?: boolean;
	/** @description whether this is a system application */
	system?: boolean;
	/** @description application version id */
	version_id?: components["schemas"]["Id"];
};
export type RoomMemberOrigin = components["schemas"]["RoomMemberOrigin"];
export type MessageSync = components["schemas"]["MessageSync"];
export type RoomBan = components["schemas"]["RoomBan"] & {
	/** @description room id (client-side context, not in canonical schema) */
	room_id?: components["schemas"]["Id"];
};
export type Notification = components["schemas"]["Notification"];
export type Connection = components["schemas"]["Connection"];
export type Scope = components["schemas"]["Scope"];
export type Tag = components["schemas"]["Tag"];
export type TagCreate = components["schemas"]["TagCreate"];
export type TagPatch = components["schemas"]["TagPatch"];
export type ChannelCreate = components["schemas"]["ChannelCreate"];
export type PushCreate = components["schemas"]["PushCreate"];
export type PushInfo = components["schemas"]["PushInfo"];
export type AutomodRule = components["schemas"]["AutomodRule"];
export type AutomodRuleCreate = components["schemas"]["AutomodRuleCreate"];
export type Attachment = components["schemas"]["MessageAttachment"];
export type ReactionKey = components["schemas"]["ReactionKey"];
export type RelationshipType = components["schemas"]["RelationshipType"];
export type MemberListGroup = components["schemas"]["MemberListGroup"];
export type MemberListOp = components["schemas"]["MemberListOp"];
export type ChannelPatch = components["schemas"]["ChannelPatch"];
export type HistoryPagination = components["schemas"]["HistoryPagination"];
export type PaginationResponse<T = unknown> = {
	items: Array<T>;
	total: number;
	has_more: boolean;
	cursor?: string | null;
};
export type Webhook = components["schemas"]["Webhook"];
export type NotifsChannel = components["schemas"]["NotifsChannel"];
export type Time = components["schemas"]["Time"];
export type PermissionOverwriteType =
	components["schemas"]["PermissionOverwriteType"];
export type NotifsRoom = components["schemas"]["NotifsRoom"];

export type RoomAnalyticsChannel =
	components["schemas"]["RoomAnalyticsChannel"];
export type RoomAnalyticsInvites =
	components["schemas"]["RoomAnalyticsInvites"];
export type RoomAnalyticsInvitesOrigin =
	components["schemas"]["RoomAnalyticsInvitesOrigin"];
export type RoomAnalyticsMembersCount =
	components["schemas"]["RoomAnalyticsMembersCount"];
export type RoomAnalyticsMembersJoin =
	components["schemas"]["RoomAnalyticsMembersJoin"];
export type RoomAnalyticsMembersLeave =
	components["schemas"]["RoomAnalyticsMembersLeave"];
export type RoomAnalyticsOverview =
	components["schemas"]["RoomAnalyticsOverview"];
export type AutomodTrigger = components["schemas"]["AutomodTrigger"];
export type AutomodAction = components["schemas"]["AutomodAction"];
export type AutomodTarget = components["schemas"]["AutomodTarget"];
export type ReactionCount = components["schemas"]["ReactionCount"];
export type InviteTarget = components["schemas"]["InviteTarget"];
export type MentionsUser = components["schemas"]["MentionsUser"];
export type MentionsChannel = components["schemas"]["MentionsChannel"];
export type MentionsRole = components["schemas"]["MentionsRole"];
export type MentionsEmoji = components["schemas"]["MentionsEmoji"];
export type ParseMentions = components["schemas"]["ParseMentions"];
export type MessageMetadata = components["schemas"]["MessageMetadata"];

// TODO: use openai schema for all of the types below

export type MessageSearch = {
	results: Array<string>; // MessageId[]
	messages: Array<Message>;
	users: Array<User>;
	threads: Array<Channel>;
	room_members: Array<RoomMember>;
	thread_members: Array<ThreadMember>;
	has_more: boolean;
	approximate_total: number;
};

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
	cursor?: string | null;
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
	| { op: "Sync"; data: MessageSync; seq: number; nonce?: string }
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
	channel_id: string;
	session_id?: string | null;
	joined_at: string;
	mute: boolean;
	deaf: boolean;
	self_mute: boolean;
	self_deaf: boolean;
	self_video: boolean;
	self_screen?: boolean;
	/** @description the thread this voice state is in */
	thread_id?: string;
};

export type InboxListParams = {
	from?: string;
	to?: string;
	dir?: "b" | "f";
	limit?: number;
	room_id?: string[];
	channel_id?: string[];
	include_read?: boolean;
};

export type FlumeState = "Live" | "Committed" | "Autocommitted";

export type MessageFlume = {
	state: FlumeState;
};

export type FlumeCreate = {
	reply_id?: string | null;
	mentions?: ParseMentions;
	metadata?: MessageMetadata;
	components: LampreyComponentCreate[];
};

export type FlumeDelta = {
	init?: LampreyComponent[];
	append?: FlumeAppend[];
	replace?: FlumeReplace[];
	delete?: string[];
};

export type FlumeAppend = {
	target: string;
	components: LampreyComponentCreate[];
};

export type FlumeReplace = {
	target: string;
	components: LampreyComponentCreate[];
};

export type LampreyComponentCreate =
	| string
	| ({
			id?: number;
	  } & LampreyComponentCreateType);

export type LampreyComponentCreateType =
	| { type: "Button"; label: string; style: ButtonStyle; custom_id: string }
	| { type: "LinkButton"; label: string; url: string }
	| {
			type: "Container";
			components: LampreyComponentCreate[];
			color: string | null;
	  }
	| { type: "Text"; content: string }
	| {
			type: "Details";
			open: boolean;
			color: string | null;
			summary: LampreyComponentCreate[];
			details: LampreyComponentCreate[];
	  }
	| {
			type: "Section";
			color: string | null;
			components: LampreyComponentCreate[];
	  }
	| { type: "Media"; items: LampreyComponentMediaCreate[] }
	| { type: "Gallery"; items: LampreyComponentMediaCreate[] };

export type LampreyComponentMediaCreate = {
	media_id: string;
	description: string | null;
	spoiler: boolean;
};

export type LampreyComponent = {
	id: number;
} & LampreyComponentType;

export type LampreyComponentType =
	| { type: "Button"; label: string; style: ButtonStyle; custom_id: string }
	| { type: "LinkButton"; label: string; url: string }
	| { type: "Container"; components: LampreyComponent[]; color: string | null }
	| { type: "Text"; content: string }
	// | { type: "Reference"; reference_id: string }
	| {
			type: "Details";
			open: boolean;
			color: string | null;
			summary: LampreyComponent[];
			details: LampreyComponent[];
	  }
	| { type: "Section"; color: string | null; components: LampreyComponent[] }
	| { type: "Media"; items: LampreyComponentMedia[] }
	| { type: "Gallery"; items: LampreyComponentMedia[] };

export type LampreyComponentMedia = {
	media: Media;
	description: string | null;
	spoiler: boolean;
};

export type ButtonStyle = "Primary" | "Secondary" | "Danger";
