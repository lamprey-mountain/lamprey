import {
	LANGUAGES,
	type CoreFile,
	type LabelsFile,
	type Shortcodes,
} from "./shared.ts";
import { createRequire } from "node:module";
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

	function addShortcodes(hex: string, s: string[]) {
		if (!shortcodesMap.has(hex)) {
			shortcodesMap.set(hex, new Set());
		}
		for (const code of s) {
			shortcodesMap.get(hex)!.add(code);
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
		addShortcodes(code.toUpperCase(), codes as string[]);
	}

	for (const [code, codes] of Object.entries(cldrNative)) {
		addShortcodes(code.toUpperCase(), codes as string[]);
	}

	for (const [code, codes] of Object.entries(joypixels)) {
		addShortcodes(code.toUpperCase(), codes as string[]);
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
	}
}

async function checkCommand(cmd: string) {
	try {
		const process = new Deno.Command("which", {
			args: [cmd],
			stdout: "null",
			stderr: "null",
		});
		const { success } = await process.output();
		if (!success) {
			console.warn("command `%s` not found. are you using `nix develop`?", cmd);
		}
		return success;
	} catch {
		console.warn("command `%s` not found. are you using `nix develop`?", cmd);
		return false;
	}
}

async function runCommand(cmd: string, args: string[]) {
	const process = new Deno.Command(cmd, { args });
	const { success, stderr } = await process.output();
	if (!success) {
		throw new Error(
			`Command \`${cmd} ${args.join(" ")}\` failed: ${new TextDecoder().decode(stderr)}`,
		);
	}
}

async function processSpritesheet(outputDir: string) {
	const EMOJI_DATA_VERSION = "16.0.0";
	const SPRITESHEET_JSON_URL = `https://raw.githubusercontent.com/iamcal/emoji-data/v${EMOJI_DATA_VERSION}/emoji.json`;
	const SPRITESHEET_IMAGE_URL = `https://raw.githubusercontent.com/iamcal/emoji-data/v${EMOJI_DATA_VERSION}/sheets-indexed-256/sheet_twitter_64_indexed_256.png`;

	const emojiSheet = `${outputDir}/sheet.png`;

	const [imageResponse, jsonResponse, emojiBaseData] = await Promise.all([
		fetch(SPRITESHEET_IMAGE_URL),
		fetch(SPRITESHEET_JSON_URL),
		loadJson("emojibase-data/en/data.json"),
	]);

	const [buffer, emojiData] = await Promise.all([
		imageResponse.arrayBuffer(),
		jsonResponse.json(),
	]);

	await Deno.writeFile(emojiSheet, new Uint8Array(buffer));

	const hexToGroup = new Map<string, number | undefined>();
	for (const e of emojiBaseData) {
		hexToGroup.set(e.hexcode.toUpperCase(), e.group);
		if (e.skins) {
			for (const s of e.skins) {
				hexToGroup.set(s.hexcode.toUpperCase(), s.group);
			}
		}
	}

	const coreEmoji = emojiData
		.map((e: any) => ({
			u: e.unified.toUpperCase(),
			x: e.sheet_x,
			y: e.sheet_y,
			g: hexToGroup.get(e.unified.toUpperCase()),
		}))
		.filter((e: any) => e.g !== undefined);

	await Deno.writeTextFile(
		`${outputDir}/emoji.json`,
		JSON.stringify({ emoji: coreEmoji }),
	);

	const [hasCwebp, hasAvifenc, hasMagick] = await Promise.all([
		checkCommand("cwebp"),
		checkCommand("avifenc"),
		checkCommand("magick"),
	]);

	const tasks = [];

	if (hasCwebp) {
		tasks.push(
			runCommand("cwebp", [
				"-q",
				"75",
				"-m",
				"6",
				emojiSheet,
				"-o",
				`${outputDir}/sheet.webp`,
			]),
		);
	}

	if (hasAvifenc) {
		tasks.push(
			runCommand("avifenc", [
				"--jobs",
				"all",
				"--speed",
				"6",
				emojiSheet,
				`${outputDir}/sheet.avif`,
			]),
		);
	}

	if (hasMagick) {
		tasks.push(
			runCommand("magick", [
				emojiSheet,
				"-colors",
				"256",
				"-quality",
				"90",
				`${outputDir}/sheet.png`,
			]),
		);
	}

	await Promise.all(tasks);
}

async function main() {
	const output = "./generated/";
	await Deno.mkdir(output, { recursive: true });
	await Promise.all([processSpritesheet(output), processLabels(output)]);
}

if (import.meta.main) main();
