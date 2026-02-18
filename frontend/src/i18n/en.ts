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
		mentions: "Mentions",
		mentions_description: "Configure how you want to be notified of mentions",
		threads: "Threads",
		threads_description: "Configure how you want to be notified of new threads",
		public_rooms: "Public Rooms",
		public_rooms_description:
			"Configure how you want to be notified of public room activity",
		private_rooms: "Private Rooms",
		private_rooms_description:
			"Configure how you want to be notified of private room activity",
		direct_messages: "Direct Messages",
		direct_messages_description:
			"Configure how you want to be notified of direct messages",
		notify: "Notify",
		watching: "Watching",
		ignore: "Ignore",
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
		MemberBridge: {
			name: "Bridge members",
			description:
				"Can add puppet users and massage timestamps; only usable for bridge bots",
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
		ReactionPurge: {
			name: "Purge reactions",
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
		RoomManage: {
			name: "Manage room",
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
		ThreadLock: {
			name: "Lock threads",
			description: "Can lock threads",
		},
		ChannelManage: {
			name: "Manage channels",
			description:
				"can create, remove, and archive channels. can also list all channels.",
		},
		ViewChannel: {
			name: "View channel",
			description: "Can view channels.",
		},
		ViewAuditLog: {
			name: "View audit log",
			description: "Can view the audit log",
		},
		VoiceConnect: {
			name: "Connect",
			description: "Can connect to voice threads",
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
		VoiceDisconnect: {
			name: "Disconnect members",
			description: "Can disconnect other members from voice threads",
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
		BypassSlowmode: {
			name: "Bypass slowmode",
			description: "Unaffected by slowmode",
		},
		ServerMetrics: {
			name: "View metrics",
			description: "Can access the metrics endpoint",
		},
		ServerOversee: {
			name: "Oversee",
			description: "Can view the server room and all members on the server",
		},
		ServerReports: {
			name: "View reports",
			description: "(unimplemented) Can view server reports",
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
		MessageMove: {
			name: "Move messages",
			description: "(unimplemented) Move messages between channels",
		},
		MessageRemove: {
			name: "Remove messages",
			description: "Remove and restore messages",
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
		RoomManageServer: {
			name: "Manage server rooms",
			description:
				"Can delete and quarantine rooms, and view all rooms, room templates, dms, and gdms.",
		},
		UserDeleteSelf: {
			name: "Delete own account",
			description: "Can disable or delete their own account",
		},
		UserManage: {
			name: "Manage users",
			description: "Can create, edit, and delete users. Can view all users.",
		},
		UserProfile: {
			name: "Edit profile",
			description: "Can edit their own profile",
		},
		ViewAnalytics: {
			name: "View analytics",
			description: "Can view room analytics",
		},
		TagApply: {
			name: "Apply tags",
			description: "(unimplemented) Apply tags to threads",
		},
		TagManage: {
			name: "Manage tags",
			description: "(unimplemented) Create and delete tags",
		},
	},
	// overwrite how permissions are rendered in channels
	permission_overwrites: {
		ViewChannel: {
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
		MemberBridge: {
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
		ReactionPurge: {
			name: "Purge reactions",
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
		ThreadLock: {
			name: "Lock threads",
			description: "Can lock threads",
		},
		VoiceConnect: {
			name: "Connect",
			description: "Can connect to voice threads",
		},
		VoiceDeafen: {
			name: "Deafen members",
			description: "Can deafen other members",
		},
		VoiceDisconnect: {
			name: "Disconnect members",
			description: "Can disconnect other members from voice threads",
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
		BypassSlowmode: {
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
			author: any,
			Link: (text: string) => any,
			ViewAll: (text: string) => any,
		) => [
			author,
			" created ",
			Link("a thread"),
			". ",
			ViewAll("View all threads"),
		],
	},
};
