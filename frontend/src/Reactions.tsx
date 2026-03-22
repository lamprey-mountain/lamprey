import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { createTooltip } from "./Tooltip.tsx";
import type { Message } from "sdk";
import { useApi } from "./api.tsx";
import icReactionAdd from "./assets/reaction-add.png";
import { renderReactionKey } from "./emoji";

type ReactionsProps = {
	message: Message;
};

export const Reactions = (props: ReactionsProps) => {
	const ctx = useCtx();
	const api = useApi();
	const [showPicker, setShowPicker] = createSignal(false);
	let addEl: HTMLDivElement | undefined;

	const reactionKeyToParam = (key: any): string => {
		if (key.type === "Text") {
			return `t:${key.content}`;
		} else if (key.type === "Custom") {
			return `c:${key.id}`;
		}
		return "";
	};

	const areKeysEqual = (a: any, b: any): boolean => {
		if (a.type !== b.type) return false;
		if (a.type === "Text") return a.content === b.content;
		if (a.type === "Custom") return a.id === b.id;
		return false;
	};

	const handleClick = (key: any, self: boolean) => {
		const param = reactionKeyToParam(key);
		if (self) {
			api.reactions.delete(props.message.channel_id, props.message.id, param);
		} else {
			api.reactions.add(props.message.channel_id, props.message.id, param);
		}
	};

	createEffect(on(showPicker, () => {
		if (showPicker()) {
			ctx.setPopout({
				id: "emoji",
				ref: addEl,
				placement: "top-start",
				props: {
					selected: (emoji: string | null, keepOpen: boolean) => {
						if (emoji) {
							// Picker returns string (unicode), we need to compare with ReactionKey
							const existing = props.message.reactions?.find((r) =>
								r.key.type === "Text" && r.key.content === emoji
							);
							if (!existing || !existing.self) {
								api.reactions.add(
									props.message.channel_id,
									props.message.id,
									`t:${emoji}`,
								);
							}
						}
						if (!keepOpen) setShowPicker(false);
					},
				},
			});
		} else {
			const popout = ctx.popout();
			if (
				popout &&
				(popout as any).id === "emoji" &&
				(popout as any).ref === addEl
			) {
				ctx.setPopout({});
			}
		}
	}));

	const closePicker = (e: MouseEvent) => {
		const popoutEl = document.querySelector(".popout");
		if (
			addEl && !addEl.contains(e.target as Node) &&
			(!popoutEl || !popoutEl.contains(e.target as Node))
		) {
			setShowPicker(false);
		}
	};

	createEffect(() => {
		if (showPicker()) {
			document.addEventListener("click", closePicker);
		} else {
			document.removeEventListener("click", closePicker);
		}
		onCleanup(() => document.removeEventListener("click", closePicker));
	});

	return (
		<div class="reactions">
			<For each={props.message.reactions}>
				{(reaction) => {
					const tip = createTooltip({
						tip: () =>
							reaction.key.type === "Text"
								? `:${reaction.key.content}:`
								: ":custom:",
					});
					return (
						<div
							ref={tip.setContentEl}
							class="reaction"
							classList={{ self: reaction.self }}
							onClick={() => handleClick(reaction.key, !!reaction.self)}
						>
							<div class="key" innerHTML={renderReactionKey(reaction.key)} />
							<div class="count">{reaction.count}</div>
						</div>
					);
				}}
			</For>
			<button class="add-reaction" ref={addEl as any}>
				<img
					class="icon"
					classList={{ show: showPicker() }}
					onClick={(e) => {
						e.stopPropagation();
						setShowPicker(!showPicker());
					}}
					src={icReactionAdd}
				/>
			</button>
		</div>
	);
};
