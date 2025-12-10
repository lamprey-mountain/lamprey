import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import twemoji from "twemoji";
import { useCtx } from "./context.ts";
import { createTooltip } from "./Tooltip.tsx";
import type { Message } from "sdk";
import { useApi } from "./api.tsx";

type ReactionsProps = {
	message: Message;
};

export const Reactions = (props: ReactionsProps) => {
	const ctx = useCtx();
	const api = useApi();
	const [showPicker, setShowPicker] = createSignal(false);
	let addEl: HTMLDivElement | undefined;

	const getTwemoji = (unicode: string) => {
		if (unicode.type === "Text") {
			return twemoji.parse(unicode.content, {
				base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
				attributes: () => ({ loading: "lazy" }),
				folder: "svg",
				ext: ".svg",
			});
		} else if (unicode.type === "Custom") {
			// FIXME: custom emoji reactions
			return "(unknown emoji)";
		}
	};

	const handleClick = (key: string, self: boolean) => {
		if (self) {
			api.reactions.delete(props.message.channel_id, props.message.id, key);
		} else {
			api.reactions.add(props.message.channel_id, props.message.id, key);
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
							const existing = props.message.reactions?.find((r) =>
								r.key === emoji
							);
							if (!existing || !existing.self) {
								api.reactions.add(
									props.message.channel_id,
									props.message.id,
									emoji,
								);
							}
						}
						if (!keepOpen) setShowPicker(false);
					},
				},
			});
		} else {
			if (ctx.popout().id === "emoji" && ctx.popout().ref === addEl) {
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
					const tip = createTooltip({ tip: () => `:${reaction.key}:` });
					return (
						<div
							ref={tip.setContentEl}
							class="reaction"
							classList={{ self: reaction.self }}
							onClick={() => handleClick(reaction.key, reaction.self)}
						>
							<div class="key" innerHTML={getTwemoji(reaction.key)} />
							<div class="count">{reaction.count}</div>
						</div>
					);
				}}
			</For>
			<div class="add" ref={addEl}>
				<div
					class="icon"
					classList={{ show: showPicker() }}
					onClick={(e) => {
						e.stopPropagation();
						setShowPicker(!showPicker());
					}}
				>
					add_reaction
				</div>
			</div>
		</div>
	);
};
