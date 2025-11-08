import { Channel } from "sdk";
import type { Api } from "./api.tsx";
import type { ChatCtx } from "./context.ts";

export type CommandOption = {
	name: string;
	description: string;
	type: "string" | "user";
	required?: boolean;
};

export type Command = {
	id: string;
	name: string;
	description: string;
	options: CommandOption[];
};

export const commands: Command[] = [
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
	},
	{
		id: "archive",
		name: "archive",
		description: "Archive the current thread",
		options: [],
	},
	{
		id: "unarchive",
		name: "unarchive",
		description: "Unarchive the current thread",
		options: [],
	},
	{
		id: "remove",
		name: "remove",
		description: "Remove the current thread",
		options: [],
	},
	{
		id: "unremove",
		name: "unremove",
		description: "Restore the current thread",
		options: [],
	},
	{
		id: "desc",
		name: "desc",
		description: "Set the description of the current thread",
		options: [
			{
				name: "description",
				description: "The new description",
				type: "string",
				required: true,
			},
		],
	},
	{
		id: "name",
		name: "name",
		description: "Set the name of the current thread",
		options: [
			{
				name: "name",
				description: "The new name",
				type: "string",
				required: true,
			},
		],
	},
	{
		id: "desc-room",
		name: "desc-room",
		description: "Set the description of the current room",
		options: [
			{
				name: "description",
				description: "The new description",
				type: "string",
				required: true,
			},
		],
	},
	{
		id: "name-room",
		name: "name-room",
		description: "Set the name of the current room",
		options: [
			{
				name: "name",
				description: "The new name",
				type: "string",
				required: true,
			},
		],
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
				description: "The duration of the timeout in seconds",
				type: "string",
				required: true,
			},
			{
				name: "reason",
				description: "The reason for the timeout",
				type: "string",
				required: false,
			},
		],
	},
	{
		id: "part",
		name: "part",
		description: "Leave a room",
		options: [],
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
	},
];

export async function handleSlashCommand(
	ctx: ChatCtx,
	api: Api,
	channel_id: string,
	text: string,
) {
	const [cmd, ...args] = text.slice(1).split(" ");
	const channel = api.channels.cache.get(channel_id);
	if (!channel) return;
	const { room_id } = channel;

	switch (cmd) {
		case "thread": {
			if (!room_id) return;
			const name = args.join(" ");
			await ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
				params: { path: { room_id } },
				body: { name, ty: "Text" },
			});
			break;
		}
		case "archive": {
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				body: { archived: true },
			});
			break;
		}
		case "unarchive": {
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				body: { archived: false },
			});
			break;
		}
		case "remove": {
			await ctx.client.http.PUT("/api/v1/channel/{channel_id}/remove", {
				params: { path: { channel_id: channel_id } },
			});
			break;
		}
		case "unremove": {
			await ctx.client.http.DELETE("/api/v1/channel/{channel_id}/remove", {
				params: { path: { channel_id: channel_id } },
			});
			break;
		}
		case "desc": {
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				body: {
					description: description || null,
				},
			});
			break;
		}
		case "name": {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				body: { name },
			});
			break;
		}
		case "desc-room": {
			if (!room_id) return;
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: {
					description: description || null,
				},
			});
			break;
		}
		case "name-room": {
			if (!room_id) return;
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: { name },
			});
			break;
		}
		case "nick": {
			if (!room_id) return;
			const name = args.join(" ");
			if (!name) return;
			const self_id = api.users.cache.get("@self")?.id;
			if (!self_id) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
				params: { path: { room_id, user_id: self_id } },
				body: { override_name: name },
			});
			break;
		}
		case "ban": {
			if (!room_id) return;
			const userId = args[0];
			const reason = args.slice(1).join(" ") || undefined;
			if (!userId) return;
			await ctx.client.http.PUT("/api/v1/room/{room_id}/ban/{user_id}", {
				params: { path: { room_id, user_id: userId } },
				headers: reason ? { "X-Reason": reason } : {},
				body: {},
			});
			break;
		}
		case "kick": {
			if (!room_id) return;
			const userId = args[0];
			const reason = args.slice(1).join(" ") || undefined;
			if (!userId) return;
			await ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
				params: { path: { room_id, user_id: userId } },
				headers: reason ? { "X-Reason": reason } : {},
			});
			break;
		}
		case "me": {
			const message = args.join(" ");
			if (!message) return;
			const body = `* ${message}`;
			await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id: channel_id } },
				body: { content: body },
			});
			break;
		}
		case "msg": {
			const userId = args[0];
			const message = args.slice(1).join(" ");
			if (!userId || !message) return;
			const { data: dm, error } = await ctx.client.http.POST("/api/v1/user/@self/dm/{target_id}", {
				params: { path: { target_id: userId } },
			});

			if (!dm) {
				console.error(error);
				return;
			}

			await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id: (dm as Channel).id } },
				body: { content: message },
			});
			break;
		}
		case "shrug": {
			const message = args.join(" ");
			const fullMessage = message ? `${message} ¯\\_(ツ)_/¯` : "¯\\_(ツ)_/¯";
			await ctx.client.http.POST("/api/v1/channel/{channel_id}/message", {
				params: { path: { channel_id: channel_id } },
				body: { content: fullMessage },
			});
			break;
		}
		case "timeout": {
			if (!room_id) return;
			const userId = args[0];
			const duration = args[1];
			const reason = args.slice(2).join(" ") || undefined;
			if (!userId || !duration) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
				params: { path: { room_id, user_id: userId } },
				headers: reason ? { "X-Reason": reason } : {},
				body: { timeout_until: new Date(Date.now() + (parseInt(duration) * 1000)).toISOString() },
			});
			break;
		}
		case "part": {
			if (!room_id) return;
			await ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
				params: { path: { room_id, user_id: "@self" } },
			});
			break;
		}
		case "lock": {
			const reason = args.join(" ") || undefined;
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				headers: reason ? { "X-Reason": reason } : {},
				body: { locked: true },
			});
			break;
		}
		case "unlock": {
			const reason = args.join(" ") || undefined;
			await ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
				params: { path: { channel_id: channel_id } },
				headers: reason ? { "X-Reason": reason } : {},
				body: { locked: false },
			});
			break;
		}
	}
}
