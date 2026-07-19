import { createRequire } from "node:module";
import {
	type CoreEmoji,
	LANGUAGES,
	type LabelsFile,
	type Shortcodes,
} from "./shared.ts";

const require = createRequire(import.meta.url);

async function loadJson(path: string): Promise<any> {
	try {
		return require(path);
	} catch {
		return {};
	}
}

async function processLanguage(lang: string): Promise<LabelsFile> {
	const basePath = `emojibase-data/${lang}`;

	const [data, messages, cldr, cldrNative, joypixels] = await Promise.all([
		loadJson(`${basePath}/data.json`),
		loadJson(`${basePath}/messages.json`),
		loadJson(`${basePath}/shortcodes/cldr.json`),
		loadJson(`${basePath}/shortcodes/cldr-native.json`),
		lang === "en"
			? loadJson(`emojibase-data/en/shortcodes/joypixels.json`)
			: Promise.resolve({}),
	]);

	const shortcodesMap = new Map<string, Set<string>>();

	function addShortcodes(hex: string, s: string | string[]) {
		if (!shortcodesMap.has(hex)) {
			shortcodesMap.set(hex, new Set());
		}

		const m = shortcodesMap.get(hex)!;
		if (Array.isArray(s)) {
			for (const code of s) {
				m.add(code);
			}
		} else {
			m.add(s);
		}
	}

	for (const e of data) {
		if (e.shortcodes) {
			addShortcodes(e.hexcode.toUpperCase(), e.shortcodes);
		}

		if (e.skins) {
			for (const s of e.skins) {
				if (s.shortcodes) {
					addShortcodes(s.hexcode.toUpperCase(), s.shortcodes);
				}
			}
		}
	}

	for (const [code, codes] of Object.entries(cldr)) {
		addShortcodes(code.toUpperCase(), codes as string | string[]);
	}

	for (const [code, codes] of Object.entries(cldrNative)) {
		addShortcodes(code.toUpperCase(), codes as string | string[]);
	}

	for (const [code, codes] of Object.entries(joypixels)) {
		addShortcodes(code.toUpperCase(), codes as string | string[]);
	}

	const shortcodes: Shortcodes[] = [];
	for (const [u, s] of shortcodesMap.entries()) {
		shortcodes.push({ u, s: Array.from(s) });
	}

	return {
		groups: messages.groups,
		skinTones: messages.skinTones,
		shortcodes,
	};
}

async function processLabels(outputDir: string) {
	for (const lang of LANGUAGES) {
		const data = await processLanguage(lang);
		await Deno.writeTextFile(
			`${outputDir}/lang-${lang}.json`,
			JSON.stringify(data),
		);
		console.log("processed labels for language `%s`", lang);
	}
}

async function processSpritesheet(outputDir: string) {
	const spritesheetDir = Deno.env.get("SPRITESHEET_PATH");

	if (!spritesheetDir) {
		throw new Error("SPRITESHEET_PATH not set");
	}

	const files = ["sheet.png", "sheet.webp", "sheet.avif"];
	for (const file of files) {
		await Deno.copyFile(`${spritesheetDir}/${file}`, `${outputDir}/${file}`);
	}

	const emojiData = JSON.parse(
		await Deno.readTextFile(`${spritesheetDir}/data.json`),
	);
	const emojiBaseData = await loadJson("emojibase-data/en/data.json");

	const hexToGroup = new Map<string, number | undefined>();
	const hexToOrder = new Map<string, number | undefined>();
	for (const e of emojiBaseData) {
		hexToGroup.set(e.hexcode.toUpperCase(), e.group);
		hexToOrder.set(e.hexcode.toUpperCase(), e.order);
		if (e.skins) {
			for (const s of e.skins) {
				hexToGroup.set(s.hexcode.toUpperCase(), s.group);
				hexToOrder.set(s.hexcode.toUpperCase(), s.order);
			}
		}
	}

	const coreEmoji = emojiData.map(
		(e: any) =>
			({
				u: e.u.toUpperCase(),
				x: e.x,
				y: e.y,

				// the only emoji that don't have a group are regional indicators
				// i'll put them in group 8 (symbols) because why not i guess
				g: hexToGroup.get(e.u.toUpperCase()) ?? 8,
				o: hexToOrder.get(e.u.toUpperCase()) ?? 0,
			}) as CoreEmoji,
	);

	const COLS = Math.max(...emojiData.map((e: any) => e.x)) + 1;
	const ROWS = Math.max(...emojiData.map((e: any) => e.y)) + 1;

	await Deno.writeTextFile(
		`${outputDir}/emoji.json`,
		JSON.stringify({ emoji: coreEmoji, cols: COLS, rows: ROWS }),
	);
}

async function main() {
	const output = "./generated/";
	await Deno.mkdir(output, { recursive: true });
	await Promise.all([processSpritesheet(output), processLabels(output)]);
}

if (import.meta.main) main();
