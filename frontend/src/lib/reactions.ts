import type { ReactionKey } from "sdk";

export const reactionKeyToParam = (key: ReactionKey): string => {
	if (key.type === "Text") {
		return `t:${key.content}`;
	} else if (key.type === "Custom") {
		return `c:${key.id}`;
	}
	return "";
};

export type CustomEmoji = {
	name: string;
	id: string;
};

export const parseCustomEmoji = (emoji: string): CustomEmoji | null => {
	const match = emoji.match(/^<:(\w+):([\w-]+)>$/);
	if (!match) return null;
	return { name: match[1], id: match[2] };
};
