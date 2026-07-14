import { ChatCtx } from "@/app/context";
import { Api } from "@/api";
import { command } from "./builder";
import { SlashCommands } from "./registry";
import { Command } from "./types";

export function registerDefaultSlashCommands(
	ctx: ChatCtx,
	api: Api,
	provider: SlashCommands,
) {
	const commands = [
		...threadCommands(ctx, api),
		...membershipCommands(ctx, api),
		...moderationCommands(ctx, api),
		...messageCommands(ctx, api),
	];

	for (const command of commands) {
		provider.register(command);
	}
}

function messageCommands(_ctx: ChatCtx, api: Api): Command[] {
	return [
		command("me")
			.description("Send a message with emphasis")
			.option("string", (b) =>
				b
					.name("message")
					.description("The message to send with emphasis")
					.required(),
			)
			.requires((b) => b.permission("MessageCreate"))
			.executes(async (ctx) => {
				const message = ctx.args.message;
				if (!message) return;
				const body = `*${message}*`;
				await api.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { content: body },
				});
			}),

		command("shrug")
			.description("Append a shrug emote to a message")
			.option("string", (b) =>
				b.name("message").description("The message to append the shrug to"),
			)
			.requires((b) => b.permission("MessageCreate"))
			.executes(async (ctx) => {
				const message = ctx.args.message;
				const fullMessage = message ? `${message} ¯\\_(ツ)_/¯` : "¯\\_(ツ)_/¯";
				await api.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { content: fullMessage },
				});
			}),

		command("msg")
			.description("DM a user")
			.option("user", (b) =>
				b.name("user").description("The user to message").required(),
			)
			.option("string", (b) =>
				b.name("message").description("The message to send").required(),
			)
			.executes(async (ctx) => {
				const userId = ctx.args.user;
				const message = ctx.args.message;
				if (!userId || !message) return;

				const { data: dm } = await api.client.http.POST(
					"/api/v1/user/@self/dm/{target_id}",
					{
						params: { path: { target_id: userId } },
					},
				);

				if (!dm) {
					console.error("Failed to create DM channel");
					return;
				}

				await api.client.http.POST("/api/v1/channel/{channel_id}/message", {
					params: { path: { channel_id: dm.id } },
					body: { content: message },
				});
			}),
	];
}

function moderationCommands(_ctx: ChatCtx, api: Api): Command[] {
	return [
		command("ban")
			.description("Ban a user")
			.option("user", (b) =>
				b.name("user").description("The user to ban").required(),
			)
			.option("string", (b) =>
				b.name("reason").description("The reason for the ban"),
			)
			.requires((b) => b.permission("MemberBan").insideRoom())
			.executes(async (ctx) => {
				const userId = ctx.args.user;
				const reason = ctx.args.reason;
				if (!userId) return;
				await api.client.http.PUT("/api/v1/room/{room_id}/ban/{user_id}", {
					params: { path: { room_id: ctx.room!.id, user_id: userId } },
					headers: reason ? { "X-Reason": reason } : {},
					body: {},
				});
			}),

		command("kick")
			.description("Kick a user")
			.option("user", (b) =>
				b.name("user").description("The user to kick").required(),
			)
			.option("string", (b) =>
				b.name("reason").description("The reason for the kick"),
			)
			.requires((b) => b.permission("MemberKick").insideRoom())
			.executes(async (ctx) => {
				const userId = ctx.args.user;
				const reason = ctx.args.reason;
				if (!userId) return;
				await api.client.http.DELETE(
					"/api/v1/room/{room_id}/member/{user_id}",
					{
						params: {
							path: { room_id: ctx.room!.id, user_id: userId },
							query: { soft: false },
						},
						headers: reason ? { "X-Reason": reason } : {},
					},
				);
			}),

		command("timeout")
			.description("Timeout a user")
			.option("user", (b) =>
				b.name("user").description("The user to timeout").required(),
			)
			.option("duration", (b) =>
				b
					.name("duration")
					.description("The duration of the timeout")
					.required(),
			)
			.option("string", (b) =>
				b.name("reason").description("The reason for the timeout"),
			)
			.requires((b) => b.permission("MemberTimeout").insideRoom())
			.executes(async (ctx) => {
				const userId = ctx.args.user;
				const duration = ctx.args.duration;
				const reason = ctx.args.reason;
				if (!userId || duration === undefined) return;
				await api.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
					params: { path: { room_id: ctx.room!.id, user_id: userId } },
					headers: reason ? { "X-Reason": reason } : {},
					body: {
						timeout_until: new Date(Date.now() + duration * 1000).toISOString(),
					},
				});
			}),

		// TODO: fix permission check (ThreadManage for threads)
		command("slowmode")
			.description("Set slowmode in a channel/thread")
			.option("duration", (b) =>
				b
					.name("duration")
					.description("The duration of the slowmode")
					.required(),
			)
			.option("string", (b) =>
				b.name("reason").description("The reason for the slowmode"),
			)
			.requires((b) => b.permission("ChannelManage"))
			.executes(async (ctx) => {
				const duration = ctx.args.duration;
				const reason = ctx.args.reason;
				if (duration === undefined) return;
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { slowmode_message: duration },
				});
			}),

		// TODO: fix permission check (ThreadManage for threads)
		command("lock")
			.description("Lock a channel/thread")
			.option("string", (b) =>
				b.name("reason").description("The reason for the lock"),
			)
			.requires((b) =>
				b
					.channelType("ThreadPublic", "ThreadPrivate")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				const reason = ctx.args.reason;
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { locked: { until: undefined, allow_roles: [] } },
				});
			}),

		// TODO: fix permission check (ThreadManage for threads)
		command("unlock")
			.description("Unlock a channel/thread")
			.option("string", (b) =>
				b.name("reason").description("The reason for the unlock"),
			)
			.requires((b) =>
				b
					.channelType("ThreadPublic", "ThreadPrivate")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				const reason = ctx.args.reason;
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					headers: reason ? { "X-Reason": reason } : {},
					body: { locked: null },
				});
			}),
	];
}

function threadCommands(_ctx: ChatCtx, api: Api): Command[] {
	const user = () => api.users.cache.get("@self");

	return [
		command("thread")
			.description("Create a new thread in the current channel")
			.option("string", (b) =>
				b
					.name("name")
					.description("The name of the thread to create")
					.required(),
			)
			.requires((b) => b.permission("ThreadCreatePublic"))
			.executes(async (ctx) => {
				const u = user();
				if (!u) return;

				const name = ctx.args.name;
				if (!name) return;
				await api.client.http.POST("/api/v1/channel/{channel_id}/thread", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { name, type: "ThreadPublic" },
				});
			}),

		command("archive")
			.description("Archive the current thread")
			.requires((b) =>
				// TODO: require thread to be unarchived
				b
					.channelType("ThreadPublic", "ThreadPrivate", "ThreadForum2")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { archived: true },
				});
			}),

		command("unarchive")
			.description("Unarchive the current thread")
			.requires((b) =>
				// TODO: require thread to be archived
				b
					.channelType("ThreadPublic", "ThreadPrivate", "ThreadForum2")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { archived: false },
				});
			}),

		// TODO: fix
		command("remove")
			.description("Remove the current thread")
			.requires((b) =>
				b
					.channelType("ThreadPublic", "ThreadPrivate")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				await api.client.http.PUT("/api/v1/channel/{channel_id}/remove", {
					params: { path: { channel_id: ctx.channel.id } },
				});
			}),

		// TODO: fix
		command("unremove")
			.description("Restore the current thread")
			.requires((b) =>
				b
					.channelType("ThreadPublic", "ThreadPrivate")
					.permission("ThreadManage"),
			)
			.executes(async (ctx) => {
				await api.client.http.DELETE("/api/v1/channel/{channel_id}/remove", {
					params: { path: { channel_id: ctx.channel.id } },
				});
			}),

		// TODO: fix permissions
		command("topic")
			.description("Set the description of the current thread")
			.option("string", (b) =>
				b.name("description").description("The new description").required(),
			)
			.requires((b) => b.permission("ThreadManage"))
			.executes(async (ctx) => {
				const description = ctx.args.description;
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					body: {
						description: description || undefined,
					},
				});
			}),

		// TODO: fix permissions
		// TODO: disallow in Dm/Gdm
		command("tname")
			.description("Set the name of the current thread")
			.option("string", (b) =>
				b.name("name").description("The new name").required(),
			)
			.requires((b) => b.permission("ThreadManage"))
			.executes(async (ctx) => {
				const name = ctx.args.name;
				if (!name) return;
				await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: ctx.channel.id } },
					body: { name: name || undefined },
				});
			}),
	];
}

function membershipCommands(_ctx: ChatCtx, api: Api): Command[] {
	const user = () => api.users.cache.get("@self");
	return [
		command("nick")
			.description("Change your nickname for this room")
			.option("string", (b) =>
				b.name("name").description("The new nickname").required(),
			)
			.requires((b) => b.permission("MemberNickname").insideRoom())
			.executes(async (ctx) => {
				const name = ctx.args.name;
				const user_id = user()?.id;
				if (!user_id) return;
				await api.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: ctx.room!.id,
							user_id,
						},
					},
					body: { override_name: name || null },
				});
			}),

		command("part")
			.description("Leave a room")
			.requires((b) => b.insideRoom())
			.executes(async (ctx) => {
				await api.client.http.DELETE(
					"/api/v1/room/{room_id}/member/{user_id}",
					{
						params: {
							path: { room_id: ctx.room!.id, user_id: "@self" },
							query: { soft: false },
						},
					},
				);
			}),
	];
}
