import {
	marked,
	type RendererThis,
	type Token,
	type TokenizerThis,
} from "marked";

type MentionToken = Token & {
	mention_type: "user" | "role" | "channel" | "emoji";
	id: string;
	name?: string;
	animated?: boolean;
};

type SpoilerToken = Token & {
	text: string;
	tokens: Token[];
};

const MENTION_CONFIGS = [
	{ type: "user", prefix: "@", regex: /^<@([0-9a-fA-F-]{36})>/ },
	{ type: "role", prefix: "@&", regex: /^<@&([0-9a-fA-F-]{36})>/ },
	{ type: "channel", prefix: "#", regex: /^<#([0-9a-fA-F-]{36})>/ },
	{
		type: "emoji",
		regex: /^<(a?):(\w+):([0-9a-fA-F-]{32,38})>/,
		process: (m: RegExpExecArray) => ({
			animated: !!m[1],
			name: m[2],
			id: m[3],
		}),
	},
];

const mentionExtension = {
	name: "mention",
	level: "inline" as const,
	start: (src: string) => src.indexOf("<"),
	tokenizer(src: string) {
		for (const config of MENTION_CONFIGS) {
			const match = config.regex.exec(src);
			if (match) {
				return {
					type: "mention",
					raw: match[0],
					mention_type: config.type,
					id: match[3] || match[1],
					...(config.process ? config.process(match) : {}),
				};
			}
		}
	},
	renderer(token: MentionToken) {
		const attrs = Object.entries(token)
			.filter(([k]) => ["id", "name", "animated"].includes(k))
			.map(([k, v]) => `data-emoji-${k}="${v}"`)
			.join(" ");
		return `<span class="mention" data-mention-type="${token.mention_type}" ${attrs}></span>`;
	},
};

const spoilerExtension = {
	name: "spoiler",
	level: "inline" as const,
	start: (src: string) => src.indexOf("||"),
	tokenizer(this: TokenizerThis, src: string) {
		const match = /^\|\|([\s\S]+?)\|\|/.exec(src);
		if (!match) return;
		const token = {
			type: "spoiler",
			raw: match[0],
			text: match[1],
			tokens: [],
		};
		this.lexer.inline(token.text, token.tokens);
		return token;
	},
	renderer(this: RendererThis, token: SpoilerToken) {
		const content = this.parser.parseInline(token.tokens);
		return `<span class="spoiler" onclick="this.classList.toggle('shown')">${content}</span>`;
	},
};

export const md = marked.use({
	breaks: true,
	gfm: true,
	extensions: [mentionExtension, spoilerExtension],
});
