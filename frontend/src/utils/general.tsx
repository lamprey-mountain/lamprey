import { debounce } from "@solid-primitives/scheduled";
import type { Message } from "sdk";
import { createSignal } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { createTooltip } from "@/atoms/Tooltip";
import { isMarkdown } from "@/components/features/chat/Message";
import { useModals } from "@/contexts/modal";
import { icCheck, icCopy } from "./icons";

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
	// if (isMarkdown(message.latest_version.type)) {
	// 	return message.override_name;
	// }
	return undefined;
}

export function getMessageContent(message: Message | undefined) {
	if (!message) return undefined;
	if (isMarkdown(message.latest_version.type)) {
		return message.latest_version.content;
	}
	return undefined;
}

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

export const Copyable2 = (props: { children: string; name: string }) => {
	const [copied, setCopied] = createSignal(false);
	const clearCopied = debounce(() => setCopied(false), 2000);

	const tip = createTooltip({
		tip: () => (
			<div class="copyable2-tip" classList={{ copied: copied() }}>
				<Icon src={copied() ? icCheck : icCopy} color={null} />{" "}
				{copied() ? `${props.name} copied!` : `copy ${props.name}`}
			</div>
		),
	});

	const copy = (e: MouseEvent) => {
		e.stopPropagation();
		navigator.clipboard.writeText(props.children);
		setCopied(true);
		clearCopied();
	};

	return (
		<div
			class="copyable2"
			classList={{ copied: copied() }}
			onClick={copy}
			ref={tip.content}
		>
			{props.children}
		</div>
	);
};
