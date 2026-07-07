import type { Node } from "prosemirror-model";

/**
 * Serializes a ProseMirror document to markdown format,
 * converting mention nodes back to their <@uuid> syntax.
 */
export function serializeToMarkdown(doc: Node): string {
	console.log("DOC", doc);
	let result = "";

	doc.forEach((blockNode) => {
		result += serializeBlock(blockNode);
		result += "\n";
	});

	return result.trim();
}

function serializeBlock(node: Node): string {
	switch (node.type.name) {
		case "blockquote": {
			return "> " + node.children.map((n) => serializeInline(n)).join("");
		}
		default: {
			return node.children.map((n) => serializeInline(n)).join("");
		}
	}
}

function serializeInline(node: Node): string {
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
		case "emojiCustom": {
			const emojiId = node.attrs.id;
			const name = node.attrs.name;
			const animated = node.attrs.animated;
			if (animated) {
				return `<a:${name}:${emojiId}>`;
			}
			return `<:${name}:${emojiId}>`;
		}
		case "emojiUnicode": {
			return node.attrs.char;
		}
		default: {
			// For any other node types, try to get text content
			return node.textContent;
		}
	}
}
