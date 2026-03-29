import { createResource } from "solid-js";
import twemoji from "twemoji";
import { getEmojiUrl } from "./media/util";
import type { ReactionKey } from "sdk";

export type EmojiData = {
	char: string;
	label: string;
	id: string;
	shortcodes: string[];
	hexcode: string;
	order: number;
	group: number;
};

type RawEmoji = {
	unicode: string;
	label: string;
	hexcode: string;
	order: number;
	group?: number;
};

const fetchEmojiData = async (): Promise<EmojiData[]> => {
	const [
		{ default: emojis },
		{ default: shortJoypixels },
		{ default: shortEmojibase },
	] = await Promise.all([
		import("emojibase-data/en/compact.json"),
		import("emojibase-data/en/shortcodes/joypixels.json"),
		import("emojibase-data/en/shortcodes/emojibase.json"),
	]);

	const joy = shortJoypixels as Record<string, string | string[]>;
	const base = shortEmojibase as Record<string, string | string[]>;

	const getShortcodes = (hex: string): string[] => {
		const codes1 = joy[hex];
		const codes2 = base[hex];
		const all = new Set<string>();

		[codes1, codes2].forEach((c) => {
			if (!c) return;
			if (Array.isArray(c)) c.forEach((s) => all.add(s));
			else all.add(c);
		});

		return Array.from(all);
	};

	return (emojis as RawEmoji[]).map((e) => ({
		char: e.unicode,
		label: e.label,
		// Canonical ID for usage in search/lookup
		id: `unicode:${e.label.replace(/ /g, "_")}`,
		shortcodes: getShortcodes(e.hexcode),
		hexcode: e.hexcode,
		order: e.order,
		group: e.group ?? 8,
	}));
};

export const [emojiResource] = createResource(fetchEmojiData);

export const getEmojiByShortcode = (code: string): EmojiData | null => {
	const data = emojiResource();
	if (!data) return null;
	return data.find((e) => e.shortcodes.includes(code)) ?? null;
};

/**
 * Parse a unicode emoji string into twemoji HTML.
 * @param unicode - The unicode emoji character to parse
 * @param options - Optional additional attributes to merge with defaults
 * @returns HTML string with twemoji spans
 */
export const getTwemoji = (
	unicode: string,
	options?: Parameters<typeof twemoji.parse>[1],
): string => {
	return twemoji.parse(unicode, {
		base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
		attributes: () => ({ loading: "lazy" }),
		folder: "svg",
		ext: ".svg",
		...options,
	});
};

export const getTwemojiUrl = (unicode: string): string | null => {
	const codePoint = twemoji.convert.toCodePoint(
		unicode.indexOf("\u200D") < 0 ? unicode.replace(/\uFE0F/g, "") : unicode,
	);
	if (!codePoint) return null;
	return `https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/svg/${codePoint}.svg`;
};

/**
 * Render an emoji reaction key (unicode or custom) as HTML.
 * @param key - The reaction key object
 * @returns HTML string for the emoji
 */
export const renderReactionKey = (key: ReactionKey): string => {
	if (key.type === "Text" && key.content) {
		return getTwemoji(key.content);
	} else if (key.type === "Custom" && key.media_id) {
		return `<img src="${getEmojiUrl(key.media_id)}" class="custom-emoji" alt="${
			key.name ?? ""
		}" />`;
	}
	return "";
};
