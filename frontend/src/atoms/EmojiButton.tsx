import { createEffect, createSignal, onCleanup } from "solid-js";
import { useCtx } from "@/app/context";
import { UnicodeEmoji } from "./UnicodeEmoji";
import { getEmojiHex } from "@/lib/emoji";

type EmojiButtonProps = {
	picked: (emoji: string, keepOpen: boolean) => void;
};

export const EmojiButton = (props: EmojiButtonProps) => {
	const ctx = useCtx();
	const [show, setShow] = createSignal(false);
	let wrapperEl: HTMLButtonElement | undefined;

	const emojis = ["😀", "🤨", "🥰", "🥳", "🥹", "😫", "🤬", "🤓", "🤮"];
	const [emoji, setEmoji] = createSignal(emojis[0]);

	const changeEmoji = () =>
		setEmoji(emojis[Math.floor(Math.random() * emojis.length)]);

	const handleClick = (e: MouseEvent) => {
		e.stopPropagation();
		setShow(!show());
	};

	createEffect(() => {
		if (show()) {
			ctx.setPopout({
				id: "emoji",
				ref: wrapperEl,
				placement: "top-end",
				props: {
					selected: (emoji: string | null, keepOpen: boolean) => {
						if (emoji) props.picked(emoji, keepOpen);
						if (!keepOpen) setShow(false);
					},
				},
			});
		} else {
			const popout = ctx.popout();
			if (
				popout &&
				"id" in popout &&
				popout.id === "emoji" &&
				popout.ref === wrapperEl
			) {
				ctx.setPopout(null);
			}
		}
	});

	const close = () => {
		setShow(false);
	};

	createEffect(() => {
		if (show()) {
			window.addEventListener("click", close);
		} else {
			window.removeEventListener("click", close);
		}
		onCleanup(() => {
			window.removeEventListener("click", close);
		});
	});

	return (
		<div class="emoji-button" onMouseEnter={changeEmoji} onFocus={changeEmoji}>
			<button
				class="emoji-button-inner"
				ref={wrapperEl}
				onClick={handleClick}
				classList={{ shown: show() }}
			>
				<div class="emoji-button-icon">
					<UnicodeEmoji hex={getEmojiHex(emoji())} />
				</div>
			</button>
		</div>
	);
};
