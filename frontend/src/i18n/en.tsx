import type { JSX } from "solid-js";

export default {
	loading: "loading...",
	not_found: "not found",
	page: {
		settings_channel: (name: string) => `${name} settings`,
		settings_thread: (name: string) => `${name} settings`,
		settings_room: (name: string) => `${name} settings`,
		settings_user: () => `user settings`,
		home: "Home",
	},
	user_settings: {
		theme: "Theme",
		appearance: "Appearance",
		underline_links: "Always underline links",
		show_send_button: "Show send message button",
		message_style: "Message style",
		message_group_spacing: "Message group spacing",
		chat_font_scale: "Chat font scale",
		application_scale: "Application scale",
		saturation: "Saturation",
		reduced_motion: "Reduced motion",
		reduced_motion_sync: "Sync reduced motion with system settings",
		autoplay_gifs: "Autoplay GIFs in messages",
		autoplay_emoji: "Autoplay animated emoji",
		notifications: "Notifications",
		notifications_permission_text:
			"You haven't given lamprey permission to send notifications",
		notifications_permission_button: "Allow notifications",
		desktop_notifs: "Enable desktop notifications",
		desktop_notifs_description: "Show desktop notifications for messages",
		push_notifs: "Enable push notifications",
		push_notifs_description: "Receive push notifications when away",
		tts_notifs: "Enable text to speech for notifications",
		tts_notifs_description: "Read notification messages aloud",
		notifications_more_stuff: "More notification settings",
		messages: "Messages",
		messages_description:
			"Configure how you want to be notified of new messages",
		threads: "Threads",
		threads_description: "Configure how you want to be notified of new threads",
		reactions: "Reactions",
		reactions_description: "Configure how you want to be notified of reactions",
		tts: "Text-to-speech",
		tts_description: "Configure when to use text-to-speech for notifications",
		notify: "Notify",
		watching: "Watching",
		ignore: "Ignore",
		everything: "Everything",
		mentions_only: "Mentions",
		nothing: "Nothing",
		inbox: "Inbox",
		always: "Always",
		restricted: "Restricted",
		direct_messages_only: "Direct Messages",
		chat: "Chat",
		media: "Media",
		preview_attachments: "Preview attachments",
		preview_attachments_descriptions: "Show attachment descriptions (alt text)",
		link_previews: "Enable link previews",
		input: "Input",
		typing_indicators: "Show typing indicators",
		content: "Content",
		show_spoilers: "Show spoilers",
		show_spoilers_description: "Show when to show spoilers",
		spoilers_click: "On click",
		spoilers_hover: "On hover",
		spoilers_always: "Always",
		threads_sidebar: "Threads sidebar",
		threads_sidebar_text: "Open text channels in split view",
		threads_sidebar_document: "Open document channels in split view",
		threads_sidebar_forum: "Open forum channels in split view",
		theme_description: "Choose your preferred theme",
		message_style_description: "Choose how messages are displayed",
		message_group_spacing_description:
			"Adjust the spacing between message groups",
		chat_font_scale_description: "Adjust the size of chat text",
		application_scale_description: "Adjust the overall application size",
		saturation_description: "Adjust the color saturation",
		theme_auto: "Auto (system)",
		theme_auto_highcontrast: "Auto (system) (high contrast)",
		theme_light: "Light",
		theme_dark: "Dark",
		theme_light_highcontrast: "Light (high contrast)",
		theme_dark_highcontrast: "Dark (high contrast)",
		message_style_cozy: "Cozy",
		message_style_compact: "Compact",
	},
	permissions: {
		Admin: {
			name: "Admin",
			description: "Allows **everything**. Be careful what you wish for.",
		},
		IntegrationsManage: {
			name: "Manage integrations",
			description: "Can add and remove bots in this room",
		},
		IntegrationsBridge: {
			name: "Bridge integrations",
			description:
				"Can add puppet users and massage timestamps; only usable for bridge bots",
		},
		EmojiManage: {
			name: "Manage emoji",
			description: "Can add, remove, and rename emoji",
		},
		EmojiUseExternal: {
			name: "Use external emoji",
			description: "Can use custom emoji from outside this room",
		},
		InviteCreate: {
			name: "Create Invites",
			description: "Can invite new people to this room",
		},
		MemberTimeout: {
			name: "Timeout members",
			description: "Can timeout other members.",
		},
		InviteManage: {
			name: "Manage invites",
			description: "Can revoke invites and view metadata",
		},
		MemberBan: {
			name: "Ban members",
			description: "Can ban other members permanently or for a period of time",
		},
		MemberKick: {
			name: "Kick members",
			description: "Can remove other members from this room or threads",
		},
		MemberNickname: {
			name: "Change nickname",
			description: "Can change their own nickname.",
		},
		MemberNicknameManage: {
			name: "Manage nicknames",
			description: "Can change other's nicknames.",
		},
		MessageCreate: {
			name: "Send messages",
			description: "Can send messages in threads",
		},
		MessageCreateThread: {
			name: "Send messages in threads",
			description:
				"Can send messages in threads. MessageCreate has no effect in threads.",
		},
		MessageAttachments: {
			name: "Use message attachments",
			description: "Can attach files and media to their message",
		},
		MessageDelete: {
			name: "Remove messages",
			description: "Can remove and restore other's messages",
		},
		MessageEmbeds: {
			name: "Use message embeds",
			description:
				"Has link previews generated for links in their message, and can send custom embeds",
		},
		MessagePin: {
			name: "Pin messages",
			description: "Can pin and unpin messages",
		},
		ReactionAdd: {
			name: "Add reactions",
			description:
				"Can add and remove new reactions to messages. Everyone can always react with an existing emoji.",
		},
		ReactionManage: {
			name: "Manage reactions",
			description: "Can remove all reactions from a message",
		},
		RoleApply: {
			name: "Apply roles",
			description: "Can apply and remove roles to members.",
		},
		RoleManage: {
			name: "Manage roles",
			description:
				"Can create, edit, delete, and reorder roles. Can set and remove permission overwrites for threads.",
		},
		RoomEdit: {
			name: "Edit room",
			description:
				"Can change this room's name, description, and icon. Can make this room public or private.",
		},
		ThreadCreatePublic: {
			name: "Create public threads",
			description: "Can create new public threads",
		},
		ThreadCreatePrivate: {
			name: "Create private threads",
			description: "Can create new private threads",
		},
		ThreadManage: {
			name: "Manage threads",
			description:
				"remove and archive threads, and move threads between channels. can also view all threads.",
		},
		ThreadEdit: {
			name: "Edit threads",
			description: "Can edit threads created by others",
		},
		ChannelEdit: {
			name: "Edit channels",
			description: "can change channel names and topics",
		},
		ChannelManage: {
			name: "Manage channels",
			description:
				"can create, remove, and archive channels. can also list all channels.",
		},
		ChannelView: {
			name: "View channel",
			description: "Can view channels.",
		},
		AuditLogView: {
			name: "View audit log",
			description: "Can view the audit log",
		},
		CalendarEventCreate: {
			name: "Create calendar events",
			description: "Can create events. Can edit and delete their own events.",
		},
		CalendarEventRsvp: {
			name: "RSVP to calendar events",
			description: "Can RSVP to calendar events",
		},
		CalendarEventManage: {
			name: "Manage calendar events",
			description:
				'Can edit and delete all events. Implies "Create calendar events".',
		},
		VoiceDeafen: {
			name: "Deafen members",
			description: "Can deafen other members",
		},
		VoiceMove: {
			name: "Move members",
			description: "Can move other members betwixt voice threads",
		},
		VoiceMute: {
			name: "Mute members",
			description: "Can mute other members",
		},
		VoicePriority: {
			name: "Priority speaker",
			description: "(unimplemented) Can talk louder",
		},
		VoiceSpeak: {
			name: "Speak",
			description: "Can talk in voice threads",
		},
		VoiceVideo: {
			name: "Video",
			description: "Can send video and screenshare in voice threads",
		},
		VoiceVad: {
			name: "Use voice activity detection",
			description: "(todo) Can use voice activity detection",
		},
		VoiceRequest: {
			name: "Request to speak",
			description: "Can request to speak in broadcast channels",
		},
		VoiceBroadcast: {
			name: "Broadcast voice",
			description: "(todo) Can broadcast voice to all channels in a category",
		},
		ChannelSlowmodeBypass: {
			name: "Bypass slowmode",
			description: "Unaffected by slowmode",
		},
		ServerOversee: {
			name: "Oversee",
			description: "Can view the server room and all members on the server",
		},
		DocumentCreate: {
			name: "Create documents",
			description:
				"Can create, edit, and remove their own documents in wiki channels.",
		},
		DocumentEdit: {
			name: "Edit documents",
			description: "Can edit documents, including documents outside of wikis.",
		},
		DocumentComment: {
			name: "Comment on documents",
			description:
				"Can comment on documents, including documents outside of wikis.",
		},
		ApplicationCreate: {
			name: "Create applications",
			description: "Can create new applications",
		},
		ApplicationManage: {
			name: "Manage applications",
			description:
				"Can edit and delete all applications. Can list all applications on the server.",
		},
		CallUpdate: {
			name: "Update call metadata",
			description: "Can set call metadata (ie. the topic)",
		},
		DmCreate: {
			name: "Create DMs",
			description: "Can create new direct messages and group direct messages",
		},
		FriendCreate: {
			name: "Send friend requests",
			description: "Can send friend requests",
		},
		MessageRemove: {
			name: "Remove messages",
			description: "Remove and restore messages",
		},
		MessageMove: {
			name: "Move messages",
			description: "(unimplemented) Move messages between channels",
		},
		RoomCreate: {
			name: "Create rooms",
			description: "Can create new rooms",
		},
		RoomForceJoin: {
			name: "Force join rooms",
			description:
				"Can forcibly make other users join and leave rooms and gdms. Can join any room and gdm.",
		},
		RoomJoin: {
			name: "Join rooms",
			description: "Can manually join and leave rooms and gdms",
		},
		RoomManage: {
			name: "Manage rooms",
			description:
				"Can delete and quarantine rooms, and view all rooms, room templates, dms, and gdms.",
		},
		UserManageSelf: {
			name: "Manage own account",
			description: "Can disable or delete their own account",
		},
		UserManage: {
			name: "Manage users",
			description: "Can create, edit, and delete users. Can view all users.",
		},
		UserProfileSelf: {
			name: "Edit profile",
			description: "Can edit their own profile",
		},
		ViewAnalytics: {
			name: "View analytics",
			description: "Can view room analytics",
		},
	},
	// overwrite how permissions are rendered in channels
	permission_overwrites: {
		ChannelView: {
			name: "View channel",
			description: "Can view this channel.",
		},
		ChannelEdit: {
			name: "Edit channel",
			description: "can change this channel's name and topic",
		},
		RoleManage: {
			name: "Manage permissions",
			description: "Can set and remove permission overwrites for this channel.",
		},
		IntegrationsManage: {
			name: "Manage webhooks",
			description: "Can add and remove webhooks in this channel",
		},
		InviteCreate: {
			name: "Create Invites",
			description: "Can invite new people to this channel",
		},
		InviteManage: {
			name: "Manage invites",
			description: "Can revoke invites and view metadata",
		},
		IntegrationsBridge: {
			name: "Bridge members",
			description:
				"Can add puppet users and massage timestamps; only usable for bridge bots",
		},
		MemberKick: {
			name: "Manage thread members",
			description: "Can remove other members from threads",
		},
		MessageCreate: {
			name: "Send messages",
			description: "Can send messages in threads",
		},
		MessageCreateThread: {
			name: "Send messages in threads",
			description:
				"Can send messages in threads. MessageCreate has no effect in threads.",
		},
		MessageAttachments: {
			name: "Use message attachments",
			description: "Can attach files and media to their message",
		},
		MessageDelete: {
			name: "Remove messages",
			description: "Can remove and restore other's messages",
		},
		MessageEmbeds: {
			name: "Use message embeds",
			description:
				"Has link previews generated for links in their message, and can send custom embeds",
		},
		MessagePin: {
			name: "Pin messages",
			description: "Can pin and unpin messages",
		},
		EmojiUseExternal: {
			name: "Use external emoji",
			description: "Can use custom emoji from outside this room",
		},
		ReactionAdd: {
			name: "Add reactions",
			description:
				"Can add and remove new reactions to messages. Everyone can always react with an existing emoji.",
		},
		ReactionManage: {
			name: "Manage reactions",
			description: "Can remove all reactions from a message",
		},
		ThreadCreatePublic: {
			name: "Create public threads",
			description: "Can create new public threads",
		},
		ThreadCreatePrivate: {
			name: "Create private threads",
			description: "Can create new private threads",
		},
		ThreadManage: {
			name: "Manage threads",
			description:
				"remove and archive threads, and move threads between channels. can also view all threads.",
		},
		ThreadEdit: {
			name: "Edit threads",
			description: "Can edit threads created by others",
		},
		VoiceDeafen: {
			name: "Deafen members",
			description: "Can deafen other members",
		},
		VoiceMove: {
			name: "Move members",
			description: "Can move other members betwixt voice threads",
		},
		VoiceMute: {
			name: "Mute members",
			description: "Can mute other members",
		},
		VoicePriority: {
			name: "Priority speaker",
			description: "(unimplemented) Can talk louder",
		},
		VoiceSpeak: {
			name: "Speak",
			description: "Can talk in voice threads",
		},
		VoiceVideo: {
			name: "Video",
			description: "Can send video and screenshare in voice threads",
		},
		VoiceVad: {
			name: "Use voice activity detection",
			description: "(todo) Can use voice activity detection",
		},
		VoiceRequest: {
			name: "Request to speak",
			description: "Can request to speak in broadcast channels",
		},
		VoiceBroadcast: {
			name: "Broadcast voice",
			description: "(todo) Can broadcast voice to all channels in a category",
		},
		ChannelSlowmodeBypass: {
			name: "Bypass slowmode",
			description: "Unaffected by slowmode",
		},
		CalendarEventCreate: {
			name: "Create calendar events",
			description: "Can create events. Can edit and delete their own events.",
		},
		CalendarEventRsvp: {
			name: "RSVP to calendar events",
			description: "Can RSVP to calendar events",
		},
		CalendarEventManage: {
			name: "Manage calendar events",
			description:
				'Can edit and delete all events. Implies "Create calendar events".',
		},
		DocumentCreate: {
			name: "Create documents",
			description:
				"Can create, edit, and remove their own documents in wiki channels.",
		},
		DocumentEdit: {
			name: "Edit documents",
			description: "Can edit documents, including documents outside of wikis.",
		},
		DocumentComment: {
			name: "Comment on documents",
			description:
				"Can comment on documents, including documents outside of wikis.",
		},
	},
	permissions_group: {
		dangerous: "Dangerous",
		room: "Room",
		members: "Members",
		messages: "Messages",
		channels: "Channels",
		voice: "Voice",
		calendar: "Calendar",
		server: "Server",
		"server members": "Server Members",
		documents: "Documents",
		general: "General",
		threads: "Threads",
		other: "Other",
	},
	message_content: {
		thread_created: (
			author: JSX.Element,
			Link: (text: string) => JSX.Element,
			ViewAll: (text: string) => JSX.Element,
		): JSX.Element[] => [
			author,
			" created ",
			Link("a thread"),
			". ",
			ViewAll("View all threads"),
		],
		member_add: (author: JSX.Element, target: JSX.Element): JSX.Element[] => [
			author,
			" added ",
			target,
			" to the thread",
		],
		member_remove: (
			author: JSX.Element,
			target: JSX.Element,
		): JSX.Element[] => [
			author,
			" removed ",
			target,
			" from the thread",
		],
		member_join: (
			author: JSX.Element,
		): JSX.Element[] => [author, " joined the room"],
		message_pinned: (
			author: JSX.Element,
			Link: (text: string) => JSX.Element,
		): JSX.Element[] => [
			author,
			" pinned ",
			Link("a message"),
		],
		channel_rename: (
			author: JSX.Element,
			name_new: JSX.Element,
		): JSX.Element[] => [
			author,
			" renamed the thread to ",
			name_new,
		],
		messages_moved: (author: JSX.Element): JSX.Element[] => [
			author,
			" moved messages to a different channel",
		],
		call_started: (author: JSX.Element, count: number): JSX.Element[] => [
			author,
			` started a call with ${count} participant(s)`,
		],
		call_ended: (author: JSX.Element, count: number): JSX.Element[] => [
			author,
			` call ended with ${count} participant(s)`,
		],
		channel_pingback: (author: JSX.Element): JSX.Element[] => [
			author,
			" mentioned this channel from another channel",
		],
		channel_moved: (
			author: JSX.Element,
		): JSX.Element[] => [author, " moved this thread"],
		channel_icon: (
			author: JSX.Element,
		): JSX.Element[] => [author, " changed the channel icon"],
		automod_execution: (author: JSX.Element): JSX.Element[] => [
			author,
			" automod action triggered",
		],
	},
	audit_log: {
		ChannelCreate: "{{actor}} created a channel #{{channel_name}}",
		ChannelUpdate: "{{actor}} updated channel #{{channel_name}}",
		ChannelDelete: "{{actor}} deleted channel #{{channel_name}}",
		RoleCreate: "{{actor}} created role {{role_name}}",
		RoleUpdate: "{{actor}} updated role {{role_name}}",
		RoleDelete: "{{actor}} deleted role {{role_name}}",
		WebhookCreate: "{{actor}} created webhook {{webhook_name}}",
		WebhookUpdate: "{{actor}} updated webhook {{webhook_name}}",
		WebhookDelete: "{{actor}} deleted webhook {{webhook_name}}",
		RoomCreate: "{{actor}} created room {{room_name}}",
		RoomUpdate: "{{actor}} updated room {{room_name}}",
		RoomDelete: "{{actor}} deleted room {{room_name}}",
		ThreadCreate: "{{actor}} created thread {{thread_name}}",
		ThreadUpdate: "{{actor}} updated thread {{thread_name}}",
		ThreadDelete: "{{actor}} deleted thread {{thread_name}}",
		MemberUpdate: "{{actor}} updated {{target}}",
		MemberKick: "{{actor}} kicked {{target}}",
		MemberBan: "{{actor}} banned {{target}}",
		MemberUnban: "{{actor}} unbanned {{target}}",
		InviteCreate: "{{actor}} created invite {{invite_code}}",
		InviteDelete: "{{actor}} deleted invite {{invite_code}}",
		MessageDelete: "{{actor}} deleted a message",
		MessageVersionDelete: "{{actor}} deleted a message version",
		MessageDeleteBulk: "{{actor}} deleted {{count}} messages",
		ReactionPurge: "{{actor}} purged reactions",
		BotAdd: "{{actor}} added bot {{bot_name}}",
		ThreadMemberAdd: "{{actor}} added {{target}} to thread {{thread_name}}",
		ThreadMemberRemove:
			"{{actor}} removed {{target}} from thread {{thread_name}}",
		PermissionOverwriteSet:
			"{{actor}} set a permission overwrite for {{target}}",
		PermissionOverwriteDelete:
			"{{actor}} deleted a permission overwrite for {{target}}",
		RoleApply: "{{actor}} applied role {{role_name}}",
		RoleUnapply: "{{actor}} removed role {{role_name}}",
		changes: {
			in_channel: (
				props: { name: JSX.Element },
			): JSX.Element[] => ["in ", props.name],
			messages_deleted: (
				count: number,
			): JSX.Element[] => [count, " messages were deleted"],
			invite_deleted: (props: { invite_code: JSX.Element }): JSX.Element[] => [
				"invite ",
				<em class="light">{props.invite_code}</em>,
				" was deleted",
			],
			permission_overwrite_for: (
				props: { type: string; target: JSX.Element },
			): JSX.Element[] => ["for ", props.type, " ", props.target],
			role_added: (props: { role_name: JSX.Element }): JSX.Element[] => [
				"added role ",
				props.role_name,
			],
			role_removed: (props: { role_name: JSX.Element }): JSX.Element[] => [
				"removed role ",
				props.role_name,
			],
			bot_added: (props: { bot_name: JSX.Element }): JSX.Element[] => [
				"bot ",
				props.bot_name,
				" was added",
			],
			user_kicked: (props: { user_name: JSX.Element }): JSX.Element[] => [
				"kicked user ",
				props.user_name,
			],
			user_banned: (props: { user_name: JSX.Element }): JSX.Element[] => [
				"banned user ",
				props.user_name,
			],
			user_unbanned: (props: { user_name: JSX.Element }): JSX.Element[] => [
				"unbanned user ",
				props.user_name,
			],
			user_added_to_thread: (
				props: { user_name: JSX.Element },
			): JSX.Element[] => [
				"added user ",
				props.user_name,
			],
			user_removed_from_thread: (
				props: { user_name: JSX.Element },
			): JSX.Element[] => [
				"removed user ",
				props.user_name,
			],
			to_thread: (props: { channel_name: JSX.Element }): JSX.Element[] => [
				"to thread ",
				props.channel_name,
			],
			permission_granted: (
				props: { permission: JSX.Element },
			): JSX.Element[] => [
				"granted permission ",
				<em class="light">{props.permission}</em>,
			],
			permission_revoked: (
				props: { permission: JSX.Element },
			): JSX.Element[] => [
				"revoked permission ",
				<em class="light">{props.permission}</em>,
			],
			permission_denied: (
				props: { permission: JSX.Element },
			): JSX.Element[] => [
				"denied permission ",
				<em class="light">{props.permission}</em>,
			],
			permission_unset: (props: { permission: JSX.Element }): JSX.Element[] => [
				"unset permission ",
				<em class="light">{props.permission}</em>,
			],
			channel_removed: "removed the channel",
			channel_restored: "restored the channel",
			channel_archived: "archived the channel",
			channel_unarchived: "unarchived the channel",
			channel_marked_nsfw: "marked as nsfw",
			channel_unmarked_nsfw: "unmarked as nsfw",
			icon_changed: "changed the icon",
			icon_removed: "removed the icon",
			icon_added: "added an icon",
			set_field: (
				props: { field: JSX.Element; value: JSX.Element },
			): JSX.Element[] => [
				"set ",
				<em class="light">{props.field}</em>,
				" to ",
				props.value,
			],
			removed_field: (props: { field: JSX.Element }): JSX.Element[] => [
				"removed ",
				<em class="light">{props.field}</em>,
			],
		},
	},
};
