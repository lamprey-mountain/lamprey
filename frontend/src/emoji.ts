import { createResource } from "solid-js";

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
