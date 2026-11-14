import { For, Show } from "solid-js";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { emojiResource } from "@/lib/emoji";
import { getEmojiUrl } from "@/media/util";

export type EmojiInfoProps =
	| {
			type: "custom";
			emoji_id: string;
			emoji_name: string;
			emoji_animated: boolean;
		}
	| {
			type: "unicode";
			char: string;
			hex: string;
		};

export const EmojiInfo = (props: EmojiInfoProps) => {
	const custom = () => (props.type === "custom" ? props : undefined);
	const unicode = () => (props.type === "unicode" ? props : undefined);

	const unicodeData = () =>
		props.type === "unicode"
			? emojiResource().get(props.hex.toUpperCase())
			: undefined;

	const unicodeNames = () => {
		const data = unicodeData();
		if (!data) return [];
		return [...new Set([data.label, ...data.shortcodes])];
	};

	return (
		<div class="emoji-info">
			<Show
				when={custom()}
				fallback={
					<Show when={unicode()}>
						{(emoji) => (
							<>
								<UnicodeEmoji hex={emoji().hex} class="large" />
								<div class="names">
									<div class="char">{emoji().char}</div>
									<For each={unicodeNames()}>
										{(name) => <div class="name">:{name}:</div>}
									</For>
								</div>
							</>
						)}
					</Show>
				}
			>
				{(emoji) => (
					<>
						<a href={getEmojiUrl(emoji().emoji_id)}>
							<img
								class="emoji custom-emoji large"
								src={getEmojiUrl(emoji().emoji_id)}
								alt={`:${emoji().emoji_name}:`}
								title={`:${emoji().emoji_name}:`}
							/>
						</a>
						<div>{emoji().emoji_name}</div>
					</>
				)}
			</Show>
		</div>
	);
};
