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
		mention: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				user: {},
			},
			leafText(node) {
				return `<@${node.attrs.user}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-user-id": n.attrs.user, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-user-id]",
				getAttrs: (el) => ({ user: (el as HTMLElement).dataset.userId }),
			}],
		},
		mentionChannel: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				channel: {},
			},
			leafText(node) {
				return `<#${node.attrs.channel}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-channel-id": n.attrs.channel, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-channel-id]",
				getAttrs: (el) => ({ channel: (el as HTMLElement).dataset.channelId }),
			}],
		},
		mentionRole: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				role: {},
			},
			leafText(node) {
				return `<@&${node.attrs.role}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-role-id": n.attrs.role, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-role-id]",
				getAttrs: (el) => ({ role: (el as HTMLElement).dataset.roleId }),
			}],
		},
		emoji: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				id: {},
				name: {},
			},
			leafText(node) {
				return `<:${node.attrs.name}:${node.attrs.id}>`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-emoji-id": n.attrs.id,
				"data-emoji-name": n.attrs.name,
			}, `:${n.attrs.name}:`],
			parseDOM: [{
				tag: "span[data-emoji-id][data-emoji-name]",
				getAttrs: (el) => ({
					id: (el as HTMLElement).dataset.emojiId,
					name: (el as HTMLElement).dataset.emojiName,
				}),
			}],
		},
		text: {
			group: "inline",
			inline: true,
		},
	},
});
