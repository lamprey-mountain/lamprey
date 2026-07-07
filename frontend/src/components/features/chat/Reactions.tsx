import type { Message, ReactionKey as ReactionKeyT } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	on,
	onCleanup,
	VoidProps,
} from "solid-js";
import { useReactions } from "@/api";
import { useCtx } from "@/app/context";
import icReactionAdd from "@/assets/reaction-add.png";
import { createTooltip } from "@/atoms/Tooltip.tsx";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { getEmojiHex } from "@/lib/emoji";
import { getEmojiUrl } from "@/media/util";

type ReactionsProps = {
	message: Message;
};

export const Reactions = (props: ReactionsProps) => {
	const ctx = useCtx();
	const reactions2 = useReactions();
	const [showPicker, setShowPicker] = createSignal(false);
	let addEl: HTMLDivElement | undefined;

	const reactionKeyToParam = (key: ReactionKeyT): string => {
		if (key.type === "Text") {
			return `t:${key.content}`;
		} else if (key.type === "Custom") {
			return `c:${(key as ReactionKeyT & { type: "Custom" }).id}`;
		}
		return "";
	};

	const handleClick = (key: ReactionKeyT, self: boolean) => {
		const param = reactionKeyToParam(key);
		if (self) {
			reactions2.remove(props.message.channel_id, props.message.id, param);
		} else {
			reactions2.add(props.message.channel_id, props.message.id, param);
		}
	};

	createEffect(
		on(showPicker, () => {
			if (showPicker()) {
				ctx.setPopout({
					id: "emoji",
					ref: addEl,
					placement: "top-start",
					props: {
						selected: (emoji: string | null, keepOpen: boolean) => {
							if (emoji) {
								// Picker returns string (unicode), we need to compare with ReactionKey
								const existing = props.message.reactions?.find(
									(r) => r.key.type === "Text" && r.key.content === emoji,
								);
								if (!existing || !existing.self) {
									reactions2.add(
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
		}),
	);

	const closePicker = (e: MouseEvent) => {
		const popoutEl = document.querySelector(".popout");
		if (
			addEl &&
			!addEl.contains(e.target as Node) &&
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
							<ReactionKey key={reaction.key} />
							<div class="count">{reaction.count}</div>
						</div>
					);
				}}
			</For>
			<button type="button" class="add-reaction" ref={addEl as any}>
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

export const ReactionKey = (props: VoidProps<{ key: ReactionKeyT }>) => {
	// TODO: use switch/match
	return (
		<div class="key">
			{props.key.type === "Text" && props.key.content ? (
				<UnicodeEmoji hex={getEmojiHex(props.key.content)} />
			) : props.key.type === "Custom" && props.key.media_id ? (
				<img
					src={getEmojiUrl(props.key.media_id)}
					class="custom-emoji"
					alt={props.key.name ?? ""}
				/>
			) : null}
		</div>
	);
};
