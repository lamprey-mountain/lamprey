import type { Message, ReactionCount, ReactionKey as ReactionKeyT } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
	type VoidProps,
} from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { useReactions } from "@/api";
import { useCtx } from "@/app/context";
import icReactionAdd from "@/assets/reaction-add.png";
import { Icon } from "@/atoms/Icon";
import { createTooltip } from "@/atoms/Tooltip.tsx";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { getEmojiHex } from "@/lib/emoji";
import { getEmojiUrl } from "@/media/util";

export type ReactionsProps = {
	message: Message;

	/** whether to prompt to add a reaction */
	prompt?: boolean;
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

	document.addEventListener("click", closePicker);
	onCleanup(() => document.removeEventListener("click", closePicker));

	const prompt = () =>
		(props.prompt ?? false) && !props.message.reactions?.length;

	const [reactions, updateReactions] = createStore<
		Array<ReactionCount & { id: string }>
	>([]);

	createEffect(() => {
		const raw = props.message.reactions ?? [];
		const withIds = raw.map((r) => ({ ...r, id: reactionKeyToParam(r.key) }));
		updateReactions(reconcile(withIds, { key: "id" }));
	});

	return (
		<div class="reactions">
			<For each={reactions}>
				{(reaction) => {
					const tip = createTooltip({
						tip() {
							const k = reaction.key;
							const t = k.type;
							if (t === "Text") {
								return `${k.content}`;
							} else if (t === "Custom") {
								return `${k.name}`;
							} else {
								console.warn("unhandled reaction key type", reaction.key);
							}
						},
					});

					const [oldCount, setOldCount] = createSignal<number | null>(null);

					let currentEl!: HTMLDivElement;
					let oldEl: HTMLDivElement | undefined;

					// TODO: animate reaction keys getting created or removed (count changes to/from 0?)

					// animation for newly created reactions:
					// currentEl.animate(
					// 	[{ scale: 0, opacity: 0 }, { scale: 1, opacity: 1 }],
					// 	{ duration: DURATION, easing: EASING, fill: "backwards" },
					// );
					// reverse keyframes for when reaction is removed

					const handleCountChange = (newCount: number, prevCount: number) => {
						if (newCount === prevCount) return;

						// 1 = slide up, -1 = slide down
						const dir = newCount > prevCount ? 1 : -1;

						setOldCount(prevCount);

						// wait for .old to be mounted in the dom
						queueMicrotask(() => {
							if (!oldEl) return;

							const DURATION = 200;
							const EASING = "cubic-bezier(0.42, 1.31, 0.52, 1.09)";

							currentEl.animate(
								[{ translate: `0 ${dir * 100}%` }, { translate: `0 0%` }],
								{ duration: DURATION, easing: EASING, fill: "backwards" },
							);

							const oldAnim = oldEl.animate(
								[{ translate: `0 0%` }, { translate: `0 ${-dir * 100}%` }],
								{ duration: DURATION, easing: EASING, fill: "forwards" },
							);

							oldAnim.finished.finally(() => setOldCount(null));
						});
					};

					createEffect(
						on(
							() => reaction.count,
							(newCount, prevCount) => {
								if (prevCount !== undefined)
									handleCountChange(newCount, prevCount);
							},
						),
					);

					return (
						<div
							ref={tip.content}
							class="reaction"
							classList={{ self: reaction.self }}
							onClick={() => handleClick(reaction.key, !!reaction.self)}
						>
							<ReactionKey key={reaction.key} />
							<div class="count">
								<div class="current" ref={currentEl!}>
									{reaction.count}
								</div>
								<Show when={oldCount() !== null}>
									<div class="old" ref={oldEl!}>
										{oldCount()?.toString()}
									</div>
								</Show>
							</div>
						</div>
					);
				}}
			</For>
			<button
				type="button"
				class="button icon-button add-reaction"
				classList={{ show: showPicker(), prompt: prompt() }}
				ref={addEl!}
				onClick={(e) => {
					e.stopPropagation();
					e.stopImmediatePropagation();
					setShowPicker(!showPicker());
				}}
			>
				<Icon src={icReactionAdd} />
				<Show when={prompt()}>add a reaction</Show>
			</button>
		</div>
	);
};

export const ReactionKey = (props: VoidProps<{ key: ReactionKeyT }>) => {
	return (
		<div class="key">
			<Switch>
				<Match when={props.key.type === "Text" && props.key}>
					{(key) => <UnicodeEmoji hex={getEmojiHex(key().content)} />}
				</Match>
				<Match when={props.key.type === "Custom" && props.key}>
					{(key) => (
						<img
							src={getEmojiUrl(key().id)}
							class="custom-emoji"
							alt={key().name ?? ""}
						/>
					)}
				</Match>
			</Switch>
		</div>
	);
};
