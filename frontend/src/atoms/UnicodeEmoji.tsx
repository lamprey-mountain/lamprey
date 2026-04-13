import emojiData from "@twemoji-spritesheets/data.json";
import sheetUrl from "@twemoji-spritesheets/sheet.png";
import { createMemo, type VoidProps } from "solid-js";

const EMOJI_SIZE = 64;

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
			class="emoji-sprite"
			style={{
				"background-image": `url(${sheetUrl})`,
				"background-position": `${-coords().x * (EMOJI_SIZE + 2) - 1}px ${-coords().y * (EMOJI_SIZE + 2) - 1}px`,
				height: `${EMOJI_SIZE}px`,
				width: `${EMOJI_SIZE}px`,
			}}
		/>
	);
};
