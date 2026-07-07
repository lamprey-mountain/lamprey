import { LANGUAGES } from "./shared.ts";
export { LANGUAGES } from "./shared.ts";
export type * from "./shared.ts";

export const emojiUrl: string = new URL(
	"./generated/emoji.json",
	import.meta.url,
).href;

export const getLangUrl = (lang: string): string | null => {
	if (LANGUAGES.includes(lang))
		return new URL(`./generated/lang-${lang}.json`, import.meta.url).href;
	return null;
};

export const sheetWebpUrl: string = new URL(
	"./generated/sheet.webp",
	import.meta.url,
).href;
export const sheetAvifUrl: string = new URL(
	"./generated/sheet.avif",
	import.meta.url,
).href;
export const sheetPngUrl: string = new URL(
	"./generated/sheet.png",
	import.meta.url,
).href;

/** get the hex code from an emoji string */
export function getEmojiHex(emojiStr: string): string {
	// NOTE: maybe i don't want to strip the variation selector...?
	return [...emojiStr]
		.map((char) => char.codePointAt(0)!.toString(16))
		.filter((hex) => hex !== "fe0f") // Strip the variation selector-16 (VS16)
		.join("-");
}
