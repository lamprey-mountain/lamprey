import { createResource, createMemo } from "solid-js";
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
	spritesheetX: number;
	spritesheetY: number;
};

export const [rawEmojiResource] = createResource(async () => {
	const data: CoreFile = await fetch(emojiUrl).then((r) => r.json());
	return data;
});

export const [emojiLabels] = createResource(async () => {
	const data: LabelsFile = await fetch(getLangUrl("en")!).then((r) => r.json());
	return data;
});

export const emojiDimensions = createMemo(() => {
	const data = rawEmojiResource();
	if (!data) return { cols: 1, rows: 1 };
	return { cols: data.cols || 1, rows: data.rows || 1 };
});

export const emojiResource = createMemo((): Map<string, EmojiData> => {
	const data = rawEmojiResource();
	const labels = emojiLabels();
	const emoji = new Map();
	if (!data || !labels) return emoji;

	for (const e of data.emoji) {
		// PERF: make labels.shortcodes a Map, use .get()
		const shortcodes = labels.shortcodes.find((s) => s.u === e.u);
		emoji.set(e.u, {
			char: getEmojiString(e.u),
			label: shortcodes?.s[0] ?? e.u, // Fallback to hexcode
			hexcode: e.u,
			order: e.o,
			group: e.g ?? 8,
			shortcodes: shortcodes?.s ?? [],
			spritesheetX: e.x,
			spritesheetY: e.y,
		});
	}

	return emoji;
});
