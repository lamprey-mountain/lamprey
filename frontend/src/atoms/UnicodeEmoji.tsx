import { sheetPngUrl, type CoreEmoji } from "@lamprey/emoji";
import { rawEmojiResource } from "@/lib/emoji";
import { createMemo, type VoidProps, Show } from "solid-js";

function getCoords(hex: string, emojiData: CoreEmoji[]) {
	// PERF: make this a map
	const emoji = emojiData.find((e) => e.u === hex.toUpperCase());
	return emoji ? { x: emoji.x, y: emoji.y } : null;
}

export type UnicodeEmojiProps = {
	hex: string;
};

export const UnicodeEmoji = (props: VoidProps<UnicodeEmojiProps>) => {
	const data = rawEmojiResource();

	const coords = createMemo(() => {
		if (!data) return null;
		return getCoords(props.hex, data.emoji);
	});

	const dimensions = createMemo(() => {
		if (!data) return { COLS: 1, ROWS: 1 };
		const COLS = Math.max(...data.emoji.map((e) => e.x)) + 1;
		const ROWS = Math.max(...data.emoji.map((e) => e.y)) + 1;
		return { COLS, ROWS };
	});

	return (
		<Show when={coords()}>
			{(c) => {
				const { COLS, ROWS } = dimensions();
				return (
					<div
						class="emoji emoji-sprite"
						style={{
							"background-image": `url(${sheetPngUrl})`,
							"background-size": `${COLS * 100}% ${ROWS * 100}%`,
							"background-position": `${(c().x / (COLS - 1)) * 100}% ${(c().y / (ROWS - 1)) * 100}%`,
						}}
					/>
				);
			}}
		</Show>
	);
};
