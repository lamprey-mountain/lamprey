import { marked, type Token, type Tokens } from "marked";

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
	renderer(token: any) {
		const attrs = Object.entries(token)
			.filter(([k]) => ["id", "name", "animated"].includes(k))
			.map(([k, v]) => `data-emoji-${k}="${v}"`).join(" ");
		return `<span class="mention" data-mention-type="${token.mention_type}" ${attrs}></span>`;
	},
};

const spoilerExtension = {
	name: "spoiler",
	level: "inline" as const,
	start: (src: string) => src.indexOf("||"),
	tokenizer(src: string) {
		const match = /^\|\|([\s\S]+?)\|\|/.exec(src);
		if (!match) return;
		const token = {
			type: "spoiler",
			raw: match[0],
			text: match[1],
			tokens: [],
		};
		(this as any).lexer.inline(token.text, token.tokens);
		return token;
	},
	renderer(token: any) {
		const content = (this as any).parser.parseInline(token.tokens);
		return `<span class="spoiler" onclick="this.classList.toggle('shown')">${content}</span>`;
	},
};

export const md = marked.use({
	breaks: true,
	gfm: true,
	extensions: [mentionExtension, spoilerExtension],
});

const EMOJI_TEST = /\p{Emoji_Presentation}|\p{Extended_Pictographic}/u;
const CUSTOM_EMOJI_REGEX = /<(a?):(\w+):([0-9a-fA-F-]{32,38})>/g;

/**
 * Count emoji in a message content string.
 * Returns the count of emoji (both custom and unicode) if the message
 * contains ONLY emoji and whitespace, otherwise returns 0.
 */
export function countEmojiOnly(content: string): number {
	const trimmed = content.trim();
	if (!trimmed) return 0;

	// Count and remove custom emoji <:name:id> or <a:name:id>
	const customEmojiMatches = trimmed.match(CUSTOM_EMOJI_REGEX) || [];
	let emojiCount = customEmojiMatches.length;
	const withoutCustomEmoji = trimmed.replace(CUSTOM_EMOJI_REGEX, "");

	// Remove whitespace for checking
	const withoutWhitespace = withoutCustomEmoji.replace(/\s+/g, "");

	// If nothing left after removing custom emoji and whitespace, check custom emoji count
	if (withoutWhitespace.length === 0) {
		return emojiCount;
	}

	// Count unicode emoji using grapheme segmentation
	// This properly handles emoji with variation selectors, ZWJ sequences, etc.
	const segmenter = new Intl.Segmenter("en", { granularity: "grapheme" });
	for (const { segment } of segmenter.segment(withoutCustomEmoji)) {
		// Skip whitespace
		if (/\s/.test(segment)) continue;
		// Check if this grapheme contains emoji
		if (EMOJI_TEST.test(segment)) {
			emojiCount++;
		} else {
			// Non-emoji content found
			return 0;
		}
	}

	return emojiCount;
}
