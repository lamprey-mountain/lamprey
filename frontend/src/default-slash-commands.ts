import { Channel } from "sdk";
import type { Api } from "./api";
import type { ChatCtx } from "./context";
import { type Command, SlashCommands } from "./contexts/slash-commands";
import {
	createPermissionChecker,
	hasPermission as checkPermission,
} from "./permission-calculator";

export function registerDefaultSlashCommands(provider: SlashCommands) {
	const commands: Command[] = [
		{
			id: "thread",
			name: "thread",
			description: "Create a new thread in the current room",
			options: [
				{
					name: "name",
					description: "The name of the thread to create",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id || !room_id) return false;
				const checker = createPermissionChecker(
					{
						api,
						room_id,
						channel_id: channel.id,
					},
					self_id,
				);
				return checker.has("ThreadCreatePublic") ||
					checker.has("ThreadCreatePrivate");
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;

				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const checker = createPermissionChecker(
					{
						api,
						room_id,
						channel_id: channel.id,
					},
					self_id,
				);

				if (
					!checker.has("ThreadCreatePublic") &&
					!checker.has("ThreadCreatePrivate")
				) {
					console.error("Insufficient permissions to create a thread.");
					return;
				}

				const name = args.join(" ");
				if (!name) return;
				await ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
					params: { path: { room_id } },
					body: { name, ty: "Text" },
				});
			},
		},
		{
			id: "archive",
			name: "archive",
			description: "Archive the current thread",
			options: [],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				return (
					(channel.type === "ThreadPublic" ||
						channel.type === "ThreadPrivate") &&
					checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"ThreadManage",
					)
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					(channel.type !== "ThreadPublic" &&
						channel.type !== "ThreadPrivate") ||
					!checkPermission(
						{ api, room_id, channel_id },
						self_id,
						"ThreadManage",
					)
				) {
					console.error(
						"Cannot archive: not a thread or insufficient permissions.",
					);
					return;
				}

				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					body: { archived: true },
				});
			},
		},
		{
			id: "unarchive",
			name: "unarchive",
			description: "Unarchive the current thread",
			options: [],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				return (
					(channel.type === "ThreadPublic" ||
						channel.type === "ThreadPrivate") &&
					checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"ThreadManage",
					)
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					(channel.type !== "ThreadPublic" &&
						channel.type !== "ThreadPrivate") ||
					!checkPermission(
						{ api, room_id, channel_id },
						self_id,
						"ThreadManage",
					)
				) {
					console.error(
						"Cannot unarchive: not a thread or insufficient permissions.",
					);
					return;
				}
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					body: { archived: false },
				});
			},
		},
		{
			id: "remove",
			name: "remove",
			description: "Remove the current thread",
			options: [],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"ThreadManage",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) {
					console.error("This command can only be used on threads.");
					return;
				}

				if (
					!checkPermission(
						{ api, room_id, channel_id },
						self_id,
						"ThreadManage",
					)
				) {
					console.error("Insufficient permissions to remove thread.");
					return;
				}

				await ctx.client.http.PUT("/api/v1/channel/{channel_id}/remove", {
					params: { path: { channel_id: channel_id } },
				});
			},
		},
		{
			id: "unremove",
			name: "unremove",
			description: "Restore the current thread",
			options: [],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"ThreadManage",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) {
					console.error("This command can only be used on threads.");
					return;
				}

				if (
					!checkPermission(
						{ api, room_id, channel_id },
						self_id,
						"ThreadManage",
					)
				) {
					console.error("Insufficient permissions to restore thread.");
					return;
				}
				await ctx.client.http.DELETE("/api/v1/channel/{channel_id}/remove", {
					params: { path: { channel_id: channel_id } },
				});
			},
		},
		{
			id: "topic",
			name: "topic",
			description: "Set the description of the current thread",
			options: [
				{
					name: "description",
					description: "The new description",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				const permission = isThread ? "ThreadManage" : "ChannelManage";
				if (isThread && channel.creator_id === self_id) return true;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					permission,
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				const permission = isThread ? "ThreadManage" : "ChannelManage";

				if (
					!checkPermission({ api, room_id, channel_id }, self_id, permission)
				) {
					console.error("Insufficient permissions to set topic.");
					return;
				}

				const description = args.join(" ");
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					body: {
						description: description || null,
					},
				});
			},
		},
		{
			id: "tname",
			name: "tname",
			description: "Set the name of the current thread",
			options: [
				{
					name: "name",
					description: "The new name",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (isThread && channel.creator_id === self_id) return true;
				const permission = isThread ? "ThreadManage" : "ChannelManage";
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					permission,
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				const permission = isThread ? "ThreadManage" : "ChannelManage";

				if (
					!checkPermission({ api, room_id, channel_id }, self_id, permission)
				) {
					console.error("Insufficient permissions to set thread name.");
					return;
				}
				const name = args.join(" ");
				if (!name) return;
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					body: { name },
				});
			},
		},
		{
			id: "nick",
			name: "nick",
			description: "Change room override_name",
			options: [
				{
					name: "name",
					description: "The new nickname",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id || !room_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MemberNickname",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MemberNickname",
					)
				) {
					console.error("Insufficient permissions to change nickname.");
					return;
				}

				const name = args.join(" ");
				if (!name) return;
				await ctx.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
					params: { path: { room_id, user_id: self_id } },
					body: { override_name: name },
				});
			},
		},
		{
			id: "ban",
			name: "ban",
			description: "Ban a user",
			options: [
				{
					name: "user",
					description: "The user to ban",
					type: "user",
					required: true,
				},
				{
					name: "reason",
					description: "The reason for the ban",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id || !room_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MemberBan",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MemberBan",
					)
				) {
					console.error("Insufficient permissions to ban users.");
					return;
				}

				const userId = args[0];
				const reason = args.slice(1).join(" ") || undefined;
				if (!userId) return;
				await ctx.client.http.PUT("/api/v1/room/{room_id}/ban/{user_id}", {
					params: { path: { room_id, user_id: userId } },
					headers: reason ? { "X-Reason": reason } : {},
					body: {},
				});
			},
		},
		{
			id: "kick",
			name: "kick",
			description: "Kick a user",
			options: [
				{
					name: "user",
					description: "The user to kick",
					type: "user",
					required: true,
				},
				{
					name: "reason",
					description: "The reason for the kick",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id || !room_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MemberKick",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MemberKick",
					)
				) {
					console.error("Insufficient permissions to kick users.");
					return;
				}

				const userId = args[0];
				const reason = args.slice(1).join(" ") || undefined;
				if (!userId) return;
				await ctx.client.http.DELETE(
					"/api/v1/room/{room_id}/member/{user_id}",
					{
						params: { path: { room_id, user_id: userId } },
						headers: reason ? { "X-Reason": reason } : {},
					},
				);
			},
		},
		{
			id: "me",
			name: "me",
			description: "Send a message with emphasis",
			options: [
				{
					name: "message",
					description: "The message to send with emphasis",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MessageCreate",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MessageCreate",
					)
				) {
					console.error("Insufficient permissions to send messages.");
					return;
				}

				const message = args.join(" ");
				if (!message) return;
				const body = `*${message}*`;
				await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: channel_id } },
					body: { content: body },
				});
			},
		},
		{
			id: "msg",
			name: "msg",
			description: "DM a user",
			options: [
				{
					name: "user",
					description: "The user to message",
					type: "user",
					required: true,
				},
				{
					name: "message",
					description: "The message to send",
					type: "string",
					required: true,
				},
			],
			canUse: (api, room_id, channel) => true,
			execute: async (ctx, api, channel_id, args) => {
				const userId = args[0];
				const message = args.slice(1).join(" ");
				if (!userId || !message) return;
				const { data: dm, error } = await ctx.client.http.POST(
					"/api/v1/user/@self/dm/{target_id}",
					{
						params: { path: { target_id: userId } },
					},
				);

				if (!dm) {
					console.error(error);
					return;
				}

				await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: (dm as Channel).id } },
					body: { content: message },
				});
			},
		},
		{
			id: "shrug",
			name: "shrug",
			description: "Append a shrug emote to a message",
			options: [
				{
					name: "message",
					description: "The message to append the shrug to",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MessageCreate",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MessageCreate",
					)
				) {
					console.error("Insufficient permissions to send messages.");
					return;
				}
				const message = args.join(" ");
				const fullMessage = message ? `${message} ¯\\_(ツ)_/¯` : "¯\\_(ツ)_/¯";
				await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: channel_id } },
					body: { content: fullMessage },
				});
			},
		},
		{
			id: "timeout",
			name: "timeout",
			description: "Timeout a user",
			options: [
				{
					name: "user",
					description: "The user to timeout",
					type: "user",
					required: true,
				},
				{
					name: "duration",
					description: "The duration of the timeout",
					type: "duration",
					required: true,
				},
				{
					name: "reason",
					description: "The reason for the timeout",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id || !room_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"MemberTimeout",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"MemberTimeout",
					)
				) {
					console.error("Insufficient permissions to timeout users.");
					return;
				}
				const userId = args[0];
				const duration = args[1];
				const reason = args.slice(2).join(" ") || undefined;
				if (!userId || !duration) return;
				await ctx.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
					params: { path: { room_id, user_id: userId } },
					headers: reason ? { "X-Reason": reason } : {},
					body: {
						timeout_until: new Date(Date.now() + (parseInt(duration) * 1000))
							.toISOString(),
					},
				});
			},
		},
		{
			id: "slowmode",
			name: "slowmode",
			description: "Set slowmode in a channel/thread",
			options: [
				{
					name: "duration",
					description: "The duration of the slowmode",
					type: "duration",
					required: true,
				},
				{
					name: "reason",
					description: "The reason for the slowmode",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"ChannelManage",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"ChannelManage",
					)
				) {
					console.error("Insufficient permissions to set slowmode.");
					return;
				}
				const duration = args[0];
				const reason = args.slice(1).join(" ") || undefined;
				if (!duration) return;
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { slowmode_message: parseInt(duration) },
				});
			},
		},
		{
			id: "part",
			name: "part",
			description: "Leave a room",
			options: [],
			canUse: (api, room_id, channel) => !!room_id, // Can only leave if in a room
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				if (!room_id) return;
				await ctx.client.http.DELETE(
					"/api/v1/room/{room_id}/member/{user_id}",
					{
						params: { path: { room_id, user_id: "@self" } },
					},
				);
			},
		},
		{
			id: "lock",
			name: "lock",
			description: "Lock a channel/thread",
			options: [
				{
					name: "reason",
					description: "The reason for the lock",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"ThreadLock",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) {
					console.error("This command can only be used on threads.");
					return;
				}

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"ThreadLock",
					)
				) {
					console.error("Insufficient permissions to lock thread.");
					return;
				}
				const reason = args.join(" ") || undefined;
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { locked: true },
				});
			},
		},
		{
			id: "unlock",
			name: "unlock",
			description: "Unlock a channel/thread",
			options: [
				{
					name: "reason",
					description: "The reason for the unlock",
					type: "string",
					required: false,
				},
			],
			canUse: (api, room_id, channel) => {
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;
				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) return false;
				return checkPermission(
					{ api, room_id, channel_id: channel.id },
					self_id,
					"ThreadLock",
				);
			},
			execute: async (ctx, api, channel_id, args) => {
				const channel = api.channels.cache.get(channel_id);
				if (!channel) return;
				const { room_id } = channel;
				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return;

				const isThread = channel.type === "ThreadPublic" ||
					channel.type === "ThreadPrivate";
				if (!isThread) {
					console.error("This command can only be used on threads.");
					return;
				}

				if (
					!checkPermission(
						{ api, room_id, channel_id: channel.id },
						self_id,
						"ThreadLock",
					)
				) {
					console.error("Insufficient permissions to unlock thread.");
					return;
				}
				const reason = args.join(" ") || undefined;
				await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: channel_id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { locked: false },
				});
			},
		},
	];

	for (const command of commands) {
		provider.register(command);
	}
}
