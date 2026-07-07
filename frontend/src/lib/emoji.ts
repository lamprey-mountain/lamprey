import type { ReactionKey } from "sdk";
import { createResource, createMemo } from "solid-js";
import twemoji from "twemoji";
import { getEmojiUrl } from "@/media/util";
import {
	emojiUrl,
	getLangUrl,
	getEmojiHex,
	getEmojiString,
	CoreFile,
	LabelsFile,
} from "@lamprey/emoji";
export { getEmojiHex, getEmojiString };

export type EmojiData = {
	char: string;
	label: string;
	hexcode: string;
	order: number;
	group: number;
	shortcodes: string[];
};

export const [rawEmojiResource] = createResource(async () => {
	const data: CoreFile = await fetch(emojiUrl).then((r) => r.json());
	return data;
});

export const [emojiLabels] = createResource(async () => {
	const data: LabelsFile = await fetch(getLangUrl("en")!).then((r) => r.json());
	return data;
});

export const emojiResource = createMemo((): EmojiData[] => {
	const data = rawEmojiResource();
	const labels = emojiLabels();
	if (!data || !labels) return [];

	return data.emoji.map((e) => {
		// PERF: make labels.shortcodes a Map, use .get()
		const shortcodes = labels.shortcodes.find((s) => s.u === e.u);
		return {
			char: getEmojiString(e.u),
			label: shortcodes?.s[0] ?? e.u, // Fallback to hexcode
			hexcode: e.u,
			order: e.o,
			group: e.g ?? 8,
			shortcodes: shortcodes?.s ?? [],
		};
	});
});

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
