import { LANGUAGES } from "./shared.ts";

export const emojiUrl = new URL("./generated/emoji.json", import.meta.url).href;

export const getLangUrl = (lang: string) => {
	if (LANGUAGES.includes(lang))
		return new URL(`./generated/lang-${lang}.json`, import.meta.url).href;
	return null;
};

export const sheetWebpUrl = new URL("./generated/sheet.webp", import.meta.url)
	.href;
export const sheetAvifUrl = new URL("./generated/sheet.avif", import.meta.url)
	.href;
export const sheetPngUrl = new URL("./generated/sheet.png", import.meta.url)
	.href;

// async function loadEmoji() {
//   const { default } = await import("./generated/data.json");
// }

/** get the hex code from an emoji string */
export function getEmojiHex(emojiStr: string): string {
	// NOTE: maybe i don't want to strip the variation selector...?
	return [...emojiStr]
		.map((char) => char.codePointAt(0)!.toString(16))
		.filter((hex) => hex !== "fe0f") // Strip the variation selector-16 (VS16)
		.join("-");
}
