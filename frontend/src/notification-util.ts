import type { Message } from "sdk";

export async function stripMarkdownAndResolveMentions(
	content: string,
	thread_id: string,
	api: any,
	mentions?: Message["mentions"],
) {
	const { users, channels, roles, client } = api;
	let processedContent = content;

	// Replace user mentions <@user-id> with user names
	const userMentionRegex =
		/<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	processedContent = processedContent.replace(
		userMentionRegex,
		(match, userId) => {
			const mentioned = (mentions?.users as any[])?.find((u) =>
				u.id === userId
			);
			if (mentioned) return `@${mentioned.resolved_name}`;
			const user = users.cache.get(userId);
			return user ? `@${user.name}` : match; // Keep original if user not found
		},
	);

	// Replace channel mentions <#channel-id> with channel names
	const channelMentionRegex =
		/<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	processedContent = processedContent.replace(
		channelMentionRegex,
		(match, channelId) => {
			const mentioned = (mentions?.channels as any[])?.find((c) =>
				c.id === channelId
			);
			if (mentioned) return `#${mentioned.name}`;
			const channel = channels.cache.get(channelId);
			return channel ? `#${channel.name}` : match; // Keep original if channel not found
		},
	);

	// Replace role mentions <@&role-id> with role names
	const roleMentionRegex =
		/<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
	const roleMatches = Array.from(processedContent.matchAll(roleMentionRegex));
	for (const match of roleMatches) {
		const roleId = match[1];
		const thread = channels.cache.get(thread_id);
		if (!thread?.room_id) continue;

		let roleName: string | undefined;
		const cached = roles.cache.get(roleId);
		if (cached) {
			roleName = cached.name;
		} else {
			const { data } = await client.http.GET(
				"/api/v1/room/{room_id}/role/{role_id}",
				{
					params: { path: { room_id: thread.room_id, role_id: roleId } },
				},
			);
			if (data) {
				roles.cache.set(roleId, data);
				roleName = data.name;
			}
		}

		if (roleName) {
			processedContent = processedContent.replace(match[0], `@${roleName}`);
		}
	}

	// Replace emoji mentions <:name:id> with emoji name
	const emojiMentionRegex =
		/<:(\w+):[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}>/g;
	processedContent = processedContent.replace(
		emojiMentionRegex,
		(match, emojiName) => {
			return `:${emojiName}:`;
		},
	);

	// Remove basic markdown formatting
	// Bold: **text** -> text
	processedContent = processedContent.replace(/\*\*(.*?)\*\*/g, "$1");
	// Italic: *text* or _text_ -> text
	processedContent = processedContent.replace(/([*_])(.*?)\1/g, "$2");
	// Strikethrough: ~~text~~ -> text
	processedContent = processedContent.replace(/~~(.*?)~~/g, "$1");
	// Code: `text` -> text
	processedContent = processedContent.replace(/`(.*?)`/g, "$1");
	// Code blocks: ```language\ntext\n``` -> text
	processedContent = processedContent.replace(
		/```(?:\w+\n)?\n?([\s\S]*?)\n?```/g,
		"$1",
	);
	// Blockquotes: > text on new lines -> text
	processedContent = processedContent.replace(/^ *>(.*)$/gm, "$1");
	// Links: [text](url) -> text
	processedContent = processedContent.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");

	// Clean up extra whitespace
	processedContent = processedContent.replace(/\s+/g, " ").trim();

	return processedContent;
}
