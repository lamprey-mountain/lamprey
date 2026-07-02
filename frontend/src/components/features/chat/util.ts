import { MessageRange } from "@/api/services/MessagesService";
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

export type TimelineItemT2 = { key: string } & (
	| { type: "info"; header: boolean }
	| { type: "skeletons" }
	| { type: "spacer-mini" }
	| { type: "divider"; unread: boolean; date?: Date }
	| {
			type: "message";
			message: MessageT;
			separate: boolean;
			// editing: boolean;
	  }
);

export const estimateSize = (_item: TimelineItemT2): number => {
	// TODO: implement
	return 80;
};

/** render messages to timeline items */
export function renderTimeline2(
	range: MessageRange,
	readMarkerId: string | null,
) {
	const out: Array<TimelineItemT2> = [];

	if (range.has_backwards) {
		out.push({ type: "skeletons", key: "skeletons-top" });
	} else {
		out.push({ type: "info", key: "info", header: true });
	}

	for (let i = 0; i < range.items.length; i++) {
		const msg = range.items[i];
		const prev = range.items[i - 1] as MessageT | undefined;

		const markerTime =
			prev && getMsgTs(msg).getDay() !== getMsgTs(prev).getDay();
		const markerUnread = prev?.id === readMarkerId;
		if (markerTime || markerUnread) {
			out.push({
				type: "divider",
				key: `divider-${msg.id}`,
				unread: markerUnread,
				date: markerTime ? getMsgTs(msg) : undefined,
			});
		}

		const separate = prev ? shouldSplit(msg, prev) : true;
		out.push({
			type: "message",
			key: `message-${msg.id}`,
			message: msg,
			separate,
		});
	}

	if (range.has_forward) {
		out.push({ type: "skeletons", key: "skeletons-bottom" });
	} else {
		out.push({ type: "spacer-mini", key: "spacer-mini" });
	}

	return out;
}
