import type { Api } from "./api.tsx";
import type { ChatCtx } from "./context.ts";

export type CommandOption = {
	name: string;
	description: string;
	type: "string"; // For now only string
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
			// Using channel_create_room endpoint as in original implementation
			await ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
				params: { path: { room_id } },
				body: { name, ty: "Text" },
			});
			break;
		}
		case "archive": {
			// endpoint doesn't seem to exist anymore
			console.warn("archive command not implemented");
			break;
		}
		case "unarchive": {
			// endpoint doesn't seem to exist anymore
			console.warn("unarchive command not implemented");
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
	}
}
