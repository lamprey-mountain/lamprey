import emojiData from "@twemoji-spritesheets/data.json";
import sheetUrl from "@twemoji-spritesheets/sheet.png";
import { createMemo, type VoidProps } from "solid-js";

const EMOJI_SIZE = 64;

// get the total number of rows/cols in the spritesheet
const COLS = Math.max(...emojiData.map((e) => e.x)) + 1;
const ROWS = Math.max(...emojiData.map((e) => e.y)) + 1;

function getCoords(hex: string) {
	const emoji = emojiData.find((e) => e.u === hex.toUpperCase());
	return emoji ? { x: emoji.x, y: emoji.y } : null;
}

export type UnicodeEmojiProps = {
	hex: string;
};

export const UnicodeEmoji = (props: VoidProps<UnicodeEmojiProps>) => {
	const coords = createMemo(() => getCoords(props.hex) ?? { x: 0, y: 0 });

	return (
		<div
			class="emoji emoji-sprite"
			style={{
				"background-image": `url(${sheetUrl})`,
				"background-size": `${COLS * 100}% ${ROWS * 100}%`,
				"background-position": `${(coords().x / (COLS - 1)) * 100}% ${(coords().y / (ROWS - 1)) * 100}%`,
			}}
		/>
	);
};
