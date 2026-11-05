import type { ReactionKey } from "sdk";

export const reactionKeyToParam = (key: ReactionKey): string => {
	if (key.type === "Text") {
		return `t:${key.content}`;
	} else if (key.type === "Custom") {
		return `c:${key.id}`;
	}
	return "";
};
