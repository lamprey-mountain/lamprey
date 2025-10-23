import { Permission } from "sdk";

type Perm = {
	id: Permission;
	name: string;
	description: string;
	group?: string;
};

export const permissions: Array<Perm> = [
	{
		id: "Admin",
		name: "Admin",
		description: "Allows **everything**. Be careful what you wish for.",
		group: "dangerous",
	},
	{
		id: "IntegrationsManage",
		name: "Manage integrations",
		description: "Can add and remove bots in this room",
		group: "room",
	},
	{
		id: "EmojiManage",
		name: "Manage emoji",
		description: "Can add, remove, and rename emoji",
		group: "room",
	},
	{
		id: "EmojiUseExternal",
		name: "Use external emoji",
		description: "(unimplemented) Can use custom emoji from outside this room",
		group: "room",
	},
	{
		id: "InviteCreate",
		name: "Create Invites",
		description: "Can invite new people to this room",
		group: "members",
	},
	{
		id: "MemberTimeout",
		name: "Timeout members",
		description: "Can timeout other members.",
		group: "members",
	},
	{
		id: "InviteManage",
		name: "Manage invites",
		description: "Can revoke invites and view metadata",
		group: "members",
	},
	{
		id: "MemberBan",
		name: "Ban members",
		description: "Can ban other members permanently or for a period of time",
		group: "members",
	},
	{
		id: "MemberBridge",
		name: "Bridge members",
		description:
			"Can add puppet users and massage timestamps; only usable for bridge bots",
		group: "members",
	},
	{
		id: "MemberKick",
		name: "Kick members",
		description: "Can remove other members from this room or threads",
		group: "members",
	},
	{
		id: "MemberNickname",
		name: "Change nickname",
		description: "Can change their own nickname.",
		group: "members",
	},
	{
		id: "MemberNicknameManage",
		name: "Manage nicknames",
		description: "Can change other's nicknames.",
		group: "members",
	},
	{
		id: "MessageCreate",
		name: "Send messages",
		description: "Can send messages in threads",
		group: "messages",
	},
	{
		id: "MessageAttachments",
		name: "Use message attachments",
		description: "Can attach files and media to their message",
		group: "messages",
	},
	{
		id: "MessageDelete",
		name: "Remove messages",
		description: "Can remove and restore other's messages",
		group: "messages",
	},
	{
		id: "MessageEmbeds",
		name: "Use message embeds",
		description:
			"(unimplemented) Has link previews generated for links in their message, and can send custom embeds",
		group: "messages",
	},
	// { id: "MessageMassMention" },
	// { id: "MessageMove" },
	{
		id: "MessagePin",
		name: "Pin messages",
		description: "(unimplemented) Can pin and unpin messages",
		group: "messages",
	},
	{
		id: "ReactionAdd",
		name: "Add reactions",
		description:
			"Can add and remove new reactions to messages. Everyone can always react with an existing emoji.",
		group: "messages",
	},
	{
		id: "ReactionPurge",
		name: "Purge reactions",
		description: "Can remove all reactions from a message",
		group: "messages",
	},
	{
		id: "RoleApply",
		name: "Apply roles",
		description: "Can apply and remove roles to members.",
		group: "members",
	},
	{
		id: "RoleManage",
		name: "Manage roles",
		description:
			"Can create, edit, delete, and reorder roles. Can set and remove permission overwrites for threads.",
		group: "room",
	},
	{
		id: "RoomManage",
		name: "Manage room",
		description:
			"Can change this room's name, description, and icon. Can make this room public or private.",
		group: "room",
	},
	// { id: "TagApply" },
	// { id: "TagManage" },
	{
		id: "ThreadCreatePublic",
		name: "Create public threads",
		description: "Can create new public threads",
		group: "channels",
	},
	{
		id: "ThreadCreatePrivate",
		name: "Create private threads",
		description: "Can create new private threads",
		group: "channels",
	},
	{
		id: "ThreadManage",
		name: "Manage threads",
		description:
			"remove and archive threads, and move threads between channels. can also view all threads.",
		group: "channels",
	},
	{
		id: "ThreadEdit",
		name: "Edit threads",
		description: "Can edit threads created by others",
		group: "channels",
	},
	{
		id: "ChannelEdit",
		name: "Edit channels",
		description: "can change channel names and topics",
		group: "channels",
	},
	// { id: "ChannelForward" },
	{
		id: "ThreadLock",
		name: "Lock threads",
		description: "Can lock threads",
		group: "channels",
	},
	{
		id: "ChannelManage",
		name: "Manage channels",
		description:
			"can create, remove, and archive channels. can also list all channels.",
		group: "channels",
	},
	{
		id: "ViewChannel",
		name: "View channel",
		description: "Can view channels.",
		group: "channels",
	},
	// { id: "ThreadPublish" },
	{
		id: "ViewAuditLog",
		name: "View audit log",
		description: "Can view the audit log",
		group: "room",
	},
	{
		id: "VoiceConnect",
		name: "Connect",
		description: "Can connect to voice threads",
		group: "voice",
	},
	{
		id: "CalendarEventManage",
		name: "Manage calendar events",
		description: "Can manage calendar events",
		group: "calendar",
	},
	{
		id: "VoiceDeafen",
		name: "Deafen members",
		description: "Can deafen other members",
		group: "voice",
	},
	{
		id: "VoiceDisconnect",
		name: "Disconnect members",
		description: "Can disconnect other members from voice threads",
		group: "voice",
	},
	{
		id: "VoiceMove",
		name: "Move members",
		description: "Can move other members betwixt voice threads",
		group: "voice",
	},
	{
		id: "VoiceMute",
		name: "Mute members",
		description: "Can mute other members",
		group: "voice",
	},
	{
		id: "VoicePriority",
		name: "Priority speaker",
		description: "(unimplemented) Can talk louder",
		group: "voice",
	},
	{
		id: "VoiceSpeak",
		name: "Speak",
		description: "Can talk in voice threads",
		group: "voice",
	},
	{
		id: "VoiceVideo",
		name: "Video",
		description: "Can send video and screenshare in voice threads",
		group: "voice",
	},
	{
		id: "ServerMetrics",
		name: "View metrics",
		description: "(unimplemented) Can access the metrics endpoint",
		group: "server",
	},
	{
		id: "ServerOversee",
		name: "Oversee",
		description: "Can view the server room and all members on the server",
		group: "server",
	},
	{
		id: "ServerReports",
		name: "View reports",
		description: "(unimplemented) Can view server reports",
		group: "server",
	},
	{
		id: "RoleApply",
		name: "Apply roles",
		description: "Can apply and remove roles to members.",
		group: "server members",
	},
	{
		id: "MemberBan",
		name: "Suspend members",
		description:
			"Can suspend and unsuspend server members permanently or for a period of time",
		group: "server members",
	},
];

export const moderatorPermissions: Array<Permission> = [
	"IntegrationsManage",
	"EmojiManage",
	"InviteManage",
	"MemberBan",
	"MemberBridge",
	"MemberKick",
	"MemberNicknameManage",
	"MemberTimeout",
	"MessageDelete",
	"MessageMassMention",
	"MessagePin",
	"ReactionPurge",
	"RoleApply",
	"RoleManage",
	"RoomManage",
	"ThreadManage",
	"ThreadEdit",
	"ThreadLock",
	"ChannelEdit",
	"ChannelManage",
	"ViewAuditLog",
	"VoiceDeafen",
	"VoiceDisconnect",
	"VoiceMove",
	"VoiceMute",
	"VoicePriority",
];

export const permissionGroups = new Map();
for (const p of permissions) {
	const g = permissionGroups.get(p.group ?? "other");
	if (g) {
		g.push(p);
	} else {
		permissionGroups.set(p.group ?? "other", [p]);
	}
}

console.log(permissionGroups);
