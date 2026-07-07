// types vendored from emojibase
export type GroupKey =
	| "activities"
	| "animals-nature"
	| "component"
	| "flags"
	| "food-drink"
	| "objects"
	| "people-body"
	| "smileys-emotion"
	| "symbols"
	| "travel-places";

export type SkinToneKey =
	| "dark"
	| "light"
	| "medium-dark"
	| "medium-light"
	| "medium";

export interface GroupMessage {
	key: GroupKey;
	message: string;
	order: number;
}

export interface SkinToneMessage {
	key: SkinToneKey;
	message: string;
}

/** json file format for spritesheet mappings */
export interface CoreFile {
	emoji: CoreEmoji[];
	cols: number;
	rows: number;
}

export interface CoreEmoji {
	/** Unified Unicode ID (uppercase hex code sequence, e.g., "1F600") */
	u: string;

	/** Spritesheet X coordinate index */
	x: number;

	/** Spritesheet Y coordinate index */
	y: number;

	/** Emoji group */
	g?: number;

	/** Emoji order. What position in the group this emoji appears in. */
	o: number;
}

/** json file format for localized emoji data for a language */
export interface LabelsFile {
	groups: GroupMessage[];
	skinTones: SkinToneMessage[];
	shortcodes: Shortcodes[];
}

/** shortcodes for an emoji */
export interface Shortcodes {
	/** Unified Unicode ID (uppercase hex code sequence, e.g., "1F600") */
	u: string;

	/** Emoji shortcodes (without colons) */
	s: string[];
}

/** languages supported by emojibase */
export const LANGUAGES = [
	"da",
	"de",
	"en",
	"en-gb",
	"es",
	"es-mx",
	"et",
	"fi",
	"fr",
	"hu",
	"it",
	"ja",
	"ko",
	"lt",
	"ms",
	"nb",
	"nl",
	"pl",
	"pt",
	"ru",
	"sv",
	"th",
	"uk",
	"zh",
	"zh-hant",
];
