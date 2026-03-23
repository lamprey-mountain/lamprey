import { Schema } from "prosemirror-model";

export const schema = new Schema({
	nodes: {
		doc: {
			content: "block+",
		},
		paragraph: {
			content: "inline*",
			group: "block",
			whitespace: "pre",
			toDOM: () => ["p", 0],
			parseDOM: ["p", "x-html-import"].map((tag) => ({
				tag,
				preserveWhitespace: "full",
			})),
		},
		code_block: {
			content: "text*",
			group: "block",
			code: true,
			defining: true,
			attrs: { language: { default: null } },
			toDOM: (
				node,
			) => ["pre", ["code", { "data-language": node.attrs.language }, 0]],
			parseDOM: [{
				tag: "pre",
				preserveWhitespace: "full",
				getAttrs: (el) => ({
					language:
						(el as HTMLElement).querySelector("code")?.dataset.language ?? null,
				}),
			}],
		},
		blockquote: {
			content: "block+",
			group: "block",
			parseDOM: [{ tag: "blockquote" }],
			toDOM() {
				return ["blockquote", 0];
			},
		},
		heading: {
			attrs: { level: { default: 1 } },
			content: "inline*",
			group: "block",
			defining: true,
			parseDOM: [
				{ tag: "h1", attrs: { level: 1 } },
				{ tag: "h2", attrs: { level: 2 } },
				{ tag: "h3", attrs: { level: 3 } },
				{ tag: "h4", attrs: { level: 4 } },
				{ tag: "h5", attrs: { level: 5 } },
				{ tag: "h6", attrs: { level: 6 } },
			],
			toDOM(node) {
				return ["h" + node.attrs.level, 0];
			},
		},
		mention: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				user: {},
				name: { default: null },
			},
			leafText(node) {
				return node.attrs.name
					? `@${node.attrs.name}`
					: `<@${node.attrs.user}>`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-user-id": n.attrs.user,
				"data-name": n.attrs.name,
				"class": "mention",
			}, n.attrs.name ? `@${n.attrs.name}` : `<@${n.attrs.user}>`],
			parseDOM: [{
				tag: "span.mention[data-user-id]",
				getAttrs: (el) => ({
					user: (el as HTMLElement).dataset.userId,
					name: (el as HTMLElement).dataset.name,
				}),
			}],
		},
		mentionChannel: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				channel: {},
				name: { default: null },
			},
			leafText(node) {
				return node.attrs.name
					? `#${node.attrs.name}`
					: `<#${node.attrs.channel}>`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-channel-id": n.attrs.channel,
				"data-name": n.attrs.name,
				"class": "mention",
			}, n.attrs.name ? `#${n.attrs.name}` : `<#${n.attrs.channel}>`],
			parseDOM: [{
				tag: "span.mention[data-channel-id]",
				getAttrs: (el) => ({
					channel: (el as HTMLElement).dataset.channelId,
					name: (el as HTMLElement).dataset.name,
				}),
			}],
		},
		mentionRole: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				role: {},
				name: { default: null },
			},
			leafText(node) {
				return node.attrs.name
					? `@${node.attrs.name}`
					: `<@&${node.attrs.role}>`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-role-id": n.attrs.role,
				"data-name": n.attrs.name,
				"class": "mention",
			}, n.attrs.name ? `@${n.attrs.name}` : `<@&${n.attrs.role}>`],
			parseDOM: [{
				tag: "span.mention[data-role-id]",
				getAttrs: (el) => ({
					role: (el as HTMLElement).dataset.roleId,
					name: (el as HTMLElement).dataset.name,
				}),
			}],
		},
		mentionEveryone: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {},
			leafText() {
				return "@everyone";
			},
			toDOM: () => ["span", {
				"data-mention": "everyone",
				"class": "mention mention-everyone",
			}, "@everyone"],
			parseDOM: [{
				tag: "span.mention-everyone",
			}],
		},
		emojiCustom: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				id: {},
				name: {},
				animated: { default: false },
			},
			leafText(node) {
				return `:${node.attrs.name}:`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-emoji-id": n.attrs.id,
				"data-emoji-name": n.attrs.name,
				"data-emoji-animated": n.attrs.animated ? "true" : "false",
				"class": "mention",
			}, `:${n.attrs.name}:`],
			parseDOM: [{
				tag: "span[data-emoji-id][data-emoji-name]",
				getAttrs: (el) => ({
					id: (el as HTMLElement).dataset.emojiId,
					name: (el as HTMLElement).dataset.emojiName,
					animated: (el as HTMLElement).dataset.emojiAnimated === "true",
				}),
			}],
		},
		emojiUnicode: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				char: {},
			},
			leafText(node) {
				return node.attrs.char;
			},
			toDOM: (n) => [
				"span",
				{
					"data-emoji-unicode": n.attrs.char,
					"class": "emoji-unicode",
				},
				n.attrs.char,
			],
			parseDOM: [{
				tag: "span[data-emoji-unicode]",
				getAttrs: (el) => ({
					char: (el as HTMLElement).dataset.emojiUnicode,
				}),
			}],
		},
		text: {
			group: "inline",
			inline: true,
		},
	},
});
