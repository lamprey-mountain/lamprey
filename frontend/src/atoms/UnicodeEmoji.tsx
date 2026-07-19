import { sheetPngUrl } from "@lamprey/emoji";
import { createMemo, Show, type VoidProps } from "solid-js";
import { emojiDimensions as dims, emojiResource } from "@/lib/emoji";

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

	return (
		<Show when={coords()}>
			{(c) => {
				const { cols, rows } = dims();
				return (
					<div
						class="emoji emoji-sprite"
						style={{
							"background-image": `url(${sheetPngUrl})`,
							"background-size": `${cols * 100}% ${rows * 100}%`,
							"background-position": `${(c().x / (cols - 1)) * 100}% ${(c().y / (rows - 1)) * 100}%`,
						}}
					/>
				);
			}}
		</Show>
	);
};
