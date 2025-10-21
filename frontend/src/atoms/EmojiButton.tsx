import { createEffect, createSignal, onCleanup } from "solid-js";
import twemoji from "twemoji";
import { useCtx } from "../context";

type EmojiButtonProps = {
	picked: (emoji: string, keepOpen: boolean) => void;
};

export const EmojiButton = (props: EmojiButtonProps) => {
	const ctx = useCtx();
	const [show, setShow] = createSignal(false);
	let wrapperEl: HTMLDivElement | undefined;

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
			if (ctx.popout().id === "emoji" && ctx.popout().ref === wrapperEl) {
				ctx.setPopout({});
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
			<div
				class="button"
				ref={wrapperEl}
				onClick={handleClick}
				classList={{ shown: show() }}
			>
				<div
					class="icon"
					innerHTML={twemoji.parse(emoji(), {
						base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
						folder: "svg",
						ext: ".svg",
					})}
				/>
			</div>
		</div>
	);
};
