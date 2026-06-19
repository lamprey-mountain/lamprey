import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type RolePatch = components["schemas"]["RolePatch"];
export type Channel = Omit<components["schemas"]["Channel"], "type"> & {
	type: ChannelType;
};
export type ChannelType = components["schemas"]["ChannelType"] | "Scripts";
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
export type MessageSync = components["schemas"]["MessageSync"] | ScriptSync;
export type MessageClient = components["schemas"]["MessageClient"];
export type VoiceSubscription = components["schemas"]["Subscription"];
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
export type DocumentBranchState = components["schemas"]["DocumentBranchState"];
export type DocumentBranch = components["schemas"]["DocumentBranch"];
export type DocumentBranchCreate =
	components["schemas"]["DocumentBranchCreate"];
export type DocumentBranchPatch = components["schemas"]["DocumentBranchPatch"];
export type DocumentBranchMerge = components["schemas"]["DocumentBranchMerge"];
export type DocumentTag = components["schemas"]["DocumentTag"];
export type DocumentTagCreate = components["schemas"]["DocumentTagCreate"];
export type DocumentTagPatch = components["schemas"]["DocumentTagPatch"];
export type DocumentRevisionId = components["schemas"]["DocumentRevisionId"];
export type DocumentVersionId = components["schemas"]["DocumentVersionId"];
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
export type ScriptId = string;
export type RunId = string;
export type ScriptVerId = string;

// TODO: rename types
export type Script = components["schemas"]["Redex"];
export type RunCreateTrigger = components["schemas"]["EvalCreateManual"];
export type RunLogEntry = components["schemas"]["EvalLogEntry"];

export type ScriptStatus = "Creating" | "Active" | "Borked" | "Deleted";

export type ScriptVersion = components["schemas"]["RedexVersion"];
export type ScriptLocation = components["schemas"]["RedexLocation"];
export type ScriptFormat = "Javascript" | "Webassembly";
export type ScriptVersionStatus = "Processing" | "Ready" | "Error";

export type ScriptCreate = {
	format: ScriptFormat;
	location: any;
};

export type Run = components["schemas"]["Eval"];
export type RunStatus = components["schemas"]["EvalStatus"];

export type ScriptSubscribe = {
	type: "ScriptSubscribe";
	channel_id: string;
	script_id: ScriptId;
};

export type ScriptSync =
	| { type: "ScriptCreate"; script: Script }
	| { type: "ScriptUpdate"; script: Script }
	| { type: "ScriptDelete"; script_id: ScriptId; channel_id: string }
	| { type: "ScriptRunCreate"; run: Run; channel_id: string }
	| { type: "ScriptRunUpdate"; run: Run; channel_id: string }
	| {
			type: "ScriptLogCreate";
			entry: RunLogEntry;
			channel_id: string;
			run_id: string;
	  };

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
export type MessageMetadata = components["schemas"]["Metadata"];
export type TrackKey = components["schemas"]["TrackKey"];

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

export type SignallingEvent = components["schemas"]["SignallingEvent"];
export type SignallingCommand = components["schemas"]["SignallingCommand"];
export type TrackMetadata = components["schemas"]["TrackMetadata"];
export type VoiceState = components["schemas"]["VoiceState"];

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
	init?: LampreyComponentCreate[];
	append: FlumeAppend[];
	replace: FlumeReplace[];
	delete: number[];
};

export type FlumeAppend = {
	target: number;
	components: LampreyComponentCreate[];
};

export type FlumeReplace = {
	target: number;
	components: LampreyComponentCreate[];
};

export type LampreyComponentCreate =
	| string
	| ({
			id?: number;
	  } & LampreyComponentCreateType);

export type LampreyComponentCreateType =
	| { type: "Button"; label: string; style: ButtonStyle; custom_id: string }
	| { type: "LinkButton"; label: string; url: string | null }
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
	| { type: "LinkButton"; label: string; url: string | null }
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
