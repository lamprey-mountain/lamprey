import { MessageT } from "@/types";
import { getMessageOverrideName, getMsgTs } from "@/utils/general";

export function highlight(el: Element) {
	el.getAnimations().forEach((a) => a.cancel());
	el.animate(
		[
			{
				boxShadow: "4px 0 0 -1px inset oklch(var(--color-highlight))",
				backgroundColor: "oklch(var(--color-highlight) / 0.15)",
				offset: 0,
			},
			{
				boxShadow: "4px 0 0 -1px inset oklch(var(--color-highlight))",
				backgroundColor: "oklch(var(--color-highlight) / 0.15)",
				offset: 0.8,
			},
			{
				boxShadow: "none",
				backgroundColor: "transparent",
				offset: 1,
			},
		],
		{
			duration: 2000,
		},
	);
}

export function shouldSplit(a: MessageT, b: MessageT) {
	return shouldSplitInner(a, b);
}

function shouldSplitInner(a: MessageT, b: MessageT) {
	if (a.latest_version.type !== "DefaultMarkdown") return true;
	if (b.latest_version.type !== "DefaultMarkdown") return true;
	if (a.author_id !== b.author_id) return true;
	if (a.latest_version.reply_id) return true;
	if (getMessageOverrideName(a) !== getMessageOverrideName(b)) return true; // TODO: remove?
	const ts_a = getMsgTs(a);
	const ts_b = getMsgTs(b);
	if (+ts_a - +ts_b > 1000 * 60 * 5) return true;
	if (a.thread) return true;
	return false;
}
