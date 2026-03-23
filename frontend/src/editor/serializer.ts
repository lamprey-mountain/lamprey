import { Node } from "prosemirror-model";

/**
 * Serializes a ProseMirror document to markdown format,
 * converting mention nodes back to their <@uuid> syntax.
 */
export function serializeToMarkdown(doc: Node): string {
	let result = "";

	doc.forEach((blockNode) => {
		blockNode.forEach((node) => {
			result += serializeNode(node);
		});
		result += "\n";
	});

	return result.trim();
}

function serializeNode(node: Node): string {
	if (node.isText) {
		return node.text || "";
	}

	switch (node.type.name) {
		case "mention": {
			const userId = node.attrs.user;
			return `<@${userId}>`;
		}
		case "mentionChannel": {
			const channelId = node.attrs.channel;
			return `<#${channelId}>`;
		}
		case "mentionRole": {
			const roleId = node.attrs.role;
			return `<@&${roleId}>`;
		}
		case "mentionEveryone": {
			return "@everyone";
		}
		case "emoji": {
			const emojiId = node.attrs.id;
			const name = node.attrs.name;
			const animated = node.attrs.animated;
			if (animated) {
				return `<a:${name}:${emojiId}>`;
			}
			return `<:${name}:${emojiId}>`;
		}
		default: {
			// For any other node types, try to get text content
			return node.textContent;
		}
	}
}
