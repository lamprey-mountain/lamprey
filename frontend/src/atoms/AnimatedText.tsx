import { For } from "solid-js";

type AnimatedTextAnimation = "float" | "wave";

export const AnimatedText = (props: {
	children: string;
	animation?: AnimatedTextAnimation;
}) => {
	return (
		<span
			class="animated-text"
			data-animation={props.animation ?? "float"}
			role="presentation"
			aria-label={props.children}
		>
			<For each={props.children.split("")}>
				{(char, idx) => (
					<span class="character" style={{ "--index": idx() }}>
						{char}
					</span>
				)}
			</For>
		</span>
	);
};
