import type { Message } from "sdk";
import { useModals } from "@/contexts/modal";

export function createWeaklyMemoized<T extends object, U>(
	fn: (_: T) => U,
): (_: T) => U {
	const cache = new WeakMap();
	return (t: T) => {
		const cached = cache.get(t);
		if (cached) return cached;
		const ran = fn(t);
		cache.set(t, ran);
		return ran;
	};
}

export const getMsgTs = createWeaklyMemoized(
	(m: Message) => new Date(m.created_at),
);

export function getMessageOverrideName(message: Message | undefined) {
	if (!message) return undefined;
	// if (message.latest_version.type === "DefaultMarkdown") {
	// 	return message.override_name;
	// }
	return undefined;
}

export function getMessageContent(message: Message | undefined) {
	if (!message) return undefined;
	if (message.latest_version.type === "DefaultMarkdown") {
		return message.latest_version.content;
	}
	return undefined;
}

// TODO: inline version of copyable that shows "copied!" as tooltip or inline text instead of modal

export const Copyable = (props: { children: string }) => {
	const [, modalctl] = useModals();
	const copy = (e: MouseEvent) => {
		e.stopPropagation();
		navigator.clipboard.writeText(props.children);
		modalctl.alert("copied!");
	};

	return (
		<code class="copyable" onClick={copy}>
			{props.children}
		</code>
	);
};
