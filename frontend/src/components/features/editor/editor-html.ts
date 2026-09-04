import type {
	SerializedBlock,
	SerializedDocument,
	SerializedInline,
} from "@lamprey/markdown";

export function serializeToEditorHTML(ast: SerializedDocument): string {
	return ast.blocks.map(serializeBlockToEditorHTML).join("");
}

function serializeBlockToEditorHTML(block: SerializedBlock): string {
	switch (block.type) {
		case "Header":
			return `<h${block.level}>${block.children.map(serializeInlineToEditorHTML).join("")}</h${block.level}>`;
		case "Paragraph":
			return `<p>${block.children.map(serializeInlineToEditorHTML).join("")}</p>`;
		case "Blockquote":
			return `<blockquote>${block.children.map(serializeBlockToEditorHTML).join("")}</blockquote>`;
		case "Codeblock":
			return `<pre><code data-language="${escapeHTML(
				block.language || "",
			)}">${escapeHTML(block.content)}</code></pre>`;
		case "ListOrdered":
			return block.items
				.map(
					(item, i) =>
						`<p>${item.number ?? i + 1}. ${item.children
							.map(serializeInlineToEditorHTML)
							.join("")}</p>`,
				)
				.join("");
		case "ListUnordered":
			return block.items
				.map(
					(item) =>
						`<p>- ${item.children.map(serializeInlineToEditorHTML).join("")}</p>`,
				)
				.join("");
		case "ListTasks":
			return block.items
				.map(
					(item) =>
						`<p>- [${
							(item.mark as unknown as string) === "Complete" ? "x" : " "
						}] ${item.children.map(serializeInlineToEditorHTML).join("")}</p>`,
				)
				.join("");
		case "Table":
			// FIXME: table editing
			return "";
	}
}

function serializeInlineToEditorHTML(inline: SerializedInline): string {
	switch (inline.type) {
		case "Strong":
			return `**${inline.children.map(serializeInlineToEditorHTML).join("")}**`;
		case "Emphasis":
			return `*${inline.children.map(serializeInlineToEditorHTML).join("")}*`;
		case "Strikethrough":
			return `~~${inline.children.map(serializeInlineToEditorHTML).join("")}~~`;
		case "Spoiler":
			return `||${inline.children.map(serializeInlineToEditorHTML).join("")}||`;
		case "Code":
			return `\`${inline.children.map(serializeInlineToEditorHTML).join("")}\``;
		case "Link":
			return `[${inline.children.map(serializeInlineToEditorHTML).join("")}](${
				inline.href
			})`;
		case "Text":
			return escapeHTML(inline.content);
		case "Mention": {
			const m = inline.mention;
			if (m.type === "User") {
				return `<span data-user-id="${m.id}" class="mention">&lt;@${m.id}&gt;</span>`;
			}
			if (m.type === "Role") {
				return `<span data-role-id="${m.id}" class="mention">&lt;@&amp;${m.id}&gt;</span>`;
			}
			if (m.type === "Channel") {
				return `<span data-channel-id="${m.id}" class="mention">&lt;#${m.id}&gt;</span>`;
			}
			if (m.type === "Everyone") {
				return `<span data-mention="everyone" class="mention mention-everyone">@everyone</span>`;
			}
			return "";
		}
		case "CustomEmoji":
			return `<span data-emoji-id="${inline.id}" data-emoji-name="${
				inline.name
			}" data-emoji-animated="${
				inline.animated ? "true" : "false"
			}" class="mention">:${inline.name}:</span>`;
		case "UnicodeEmoji":
			return `<span data-emoji-unicode="${inline.content}" class="emoji-unicode">${inline.content}</span>`;
		case "Timestamp":
			// NOTE: is this correct?
			return `&lt;t:${inline.timestamp}:${inline.style}&gt;`;
	}
}

// TODO: deduplicate this code, move to a utils file
function escapeHTML(str: string): string {
	return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
