use serde::{Deserialize, Serialize};
// use serde::Serialize;
use uuid7::Uuid;
use utoipa::ToSchema;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00000000room"))]
pub struct RoomId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00000000user"))]
pub struct UserId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-000000thread"))]
pub struct ThreadId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00000message"))]
pub struct MessageId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00messagever"))]
pub struct MessageVersionId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00000000role"))]
pub struct RoleId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00000session"))]
pub struct SessionId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-0000000media"))]
pub struct MediaId(Uuid);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-0auditlogent"))]
pub struct AuditLogEntryId(Uuid);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[schema(examples("a1b2c3"))]
pub struct InviteCode(String);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[schema(examples("super_secret_session_token"))]
pub struct SessionToken(String);

/// A room
#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Room {
    #[schema(read_only)]
	id: RoomId,
    #[schema(read_only)]
	name: String,
    #[schema(read_only, required = false)]
	description: Option<String>,
	// default_roles: Vec<RoleId>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct RoomPatch {
    #[schema(write_only, nullable = false)]
	name: Option<String>,
    #[schema(write_only)]
	description: Option<Option<String>>,
	// default_roles: Option<Vec<RoleId>>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Role {
	id: RoleId,
	room_id: RoomId,
	name: String,
	description: Option<String>,
	permissions: Vec<Permission>,
	// is_self_applicable: bool,
	// is_mentionable: bool,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum Permission {
    Admin,
    RoomManage,
    ThreadCreate,
    ThreadManage,
    ThreadDelete,
    MessageCreate,
    MessageFilesEmbeds,
    MessagePin,
    MessageDelete,
    MessageMassMention,
    MemberKick,
    MemberBan,
    MemberManage,
    InviteCreate,
    InviteManage,
    RoleManage,
    RoleApply,
    
	View,
	MessageEdit,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct User {
	id: UserId,
	parent_id: Option<UserId>,
	name: String,
	description: Option<String>,
	status: Option<String>,
	// email: Option<String>,
	// avatar: Option<String>,
	is_bot: bool,
	is_alias: bool,
	is_system: bool,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Member {
	user: User,
	room_id: RoomId,
	membership: Membership,
	override_name: Option<String>,
	override_description: Option<String>,
	// override_avatar: z.string().url().or(z.literal("")),
	roles: Vec<Role>,
}

#[derive(Debug, PartialEq, Eq, Default, ToSchema, Serialize, Deserialize)]
pub enum Membership {
    #[default]
    Join,
    Ban,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Message {
	id: MessageId,
	thread_id: ThreadId,
	version_id: MessageVersionId,
	nonce: Option<String>,
	ordering: u64,
	content: Option<String>,
	// attachments: Media.array().default([]),
	// embeds: Embed.array().default([]),
	// metadata: z.record(z.string(), z.any()).nullable(),
	// mentions_users: UserId.array(),
	// mentions_roles: RoleId.array(),
	// mentions_everyone: z.boolean(),
	// reply_id: MessageId.nullable(),
	// resolve everything here?
	// mentions_threads: ThreadId.array(),
	// mentions_rooms: ThreadId.array(),
	// author: Member, // TODO: future? how to represent users who have left?
	author: User,
	is_pinned: bool,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Thread {
	id: ThreadId,
	room_id: RoomId,
	creator_id: UserId,
	#[schema(max_length = 1, min_length = 64)]
	name: String,
	#[schema(max_length = 1, min_length = 2048)]
	description: Option<String>,
	is_closed: bool,
	is_locked: bool,
	is_pinned: bool,
	// is_wiki: z.boolean(), // editable by everyone
	// is_private: z.boolean(),
	// recipients: Member.array(),
	#[serde(flatten)]
	info: ThreadInfo,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThreadInfo {
	Foo { a: u64 },
	Bar { b: bool },
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct ThreadPatch {
	name: Option<String>,
	description: Option<Option<String>>,
	is_closed: Option<bool>,
	is_locked: Option<bool>,
	is_pinned: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Session {
	id: SessionId,
	user_id: UserId,
	token: SessionToken,
	// status: 
	name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Invite {
	code: InviteCode,
	target: InviteTarget,
	creator_id: UserId,
	// roles: RoleId.array().optional(),
	// expires_at: z.date().optional(),
	// max_uses: Uint.optional(),
	// uses: Uint,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum InviteTarget {
	User(User),
	Room(Room),
	Thread(Thread),
}

// enum MessageServer {
// 	Ping,
// 	Ready,
// 	Error,
// 	UpsertMessage,
// 	UpsertSession,
// 	DeleteUser,
// 	DeleteMember,
// 	DeleteSession,
// 	Auditable(MessageAuditable),
// }

// enum MessageAuditable {
// 	UpsertRoom,
// 	UpsertThread,
// 	UpsertMember,
// 	DeleteMessage,
// 	DeleteVersion,
// }

// struct AuditLogEntry {
// 	user_id: UserId,
// 	reason: <String>,
// 	event: MessageAuditable,
// }

// enum MessageClient {
// 	Hello,
// 	Pong,
// }
