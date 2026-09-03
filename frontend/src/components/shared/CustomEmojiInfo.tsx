import { getEmojiUrl } from "@/media/util";

export type CustomEmojiInfoProps = {
	emoji_id: string;
	emoji_name: string;
	emoji_animated: boolean;
};

// TODO: show where the emoji is from
// TODO: show button to add emoji to this room if this is external?

export const CustomEmojiInfo = (props: CustomEmojiInfoProps) => {
	return (
		<div class="custom-emoji-info">
			<a href={getEmojiUrl(props.emoji_id)}>
				<img
					class="emoji custom-emoji large"
					src={getEmojiUrl(props.emoji_id)}
					alt={`:${props.emoji_name}:`}
					title={`:${props.emoji_name}:`}
				/>
			</a>
			<div>{props.emoji_name}</div>
		</div>
	);
};
