import { sheetPngUrl } from "@lamprey/emoji";
import { emojiResource } from "@/lib/emoji";
import { createMemo, type VoidProps, Show } from "solid-js";

export type UnicodeEmojiProps = {
	hex: string;
};

export const UnicodeEmoji = (props: VoidProps<UnicodeEmojiProps>) => {
	const data = emojiResource();

	const coords = createMemo(() => {
		if (!data) return null;
		const emoji = data.get(props.hex.toUpperCase());
		return emoji ? { x: emoji.spritesheetX, y: emoji.spritesheetY } : null;
	});

	const dimensions = createMemo(() => {
		if (!data) return { COLS: 1, ROWS: 1 };
		// PERF: surely there's a better way to do this
		// TODO: probably calculate spritesheet width/height in emoji build script?
		// CoreFile.{height, width}
		const COLS = Math.max(...[...data.values()].map((e) => e.spritesheetX)) + 1;
		const ROWS = Math.max(...[...data.values()].map((e) => e.spritesheetY)) + 1;
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
