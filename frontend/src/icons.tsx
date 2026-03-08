import { Show } from "solid-js";
import icCheck1 from "./assets/check-1.png";
import icCheck2 from "./assets/check-2.png";
import icCheck3 from "./assets/check-3.png";
import icCheck4 from "./assets/check-4.png";
import icX1 from "./assets/x-1.png";
import icX2 from "./assets/x-2.png";
import icX3 from "./assets/x-3.png";
import icX4 from "./assets/x-4.png";
import { cyrb53, LCG } from "./rng";

export const getCheckIcon = (seed: string): string => {
	const rand = LCG(cyrb53(seed));
	const checks = [icCheck1, icCheck2, icCheck3, icCheck4];
	return checks[Math.floor(rand() * checks.length)];
};

export const getXIcon = (seed: string): string => {
	const rand = LCG(cyrb53(seed));
	const xs = [icX1, icX2, icX3, icX4];
	return xs[Math.floor(rand() * xs.length)];
};

export const RadioDot = (props: { checked?: boolean }) => {
	return (
		<svg
			class="radio"
			viewBox="0 0 16 16"
			aria-hidden="true"
			xmlns="http://www.w3.org/2000/svg"
		>
			<circle
				cx="8"
				cy="8"
				r="6"
				fill={props.checked ? "oklch(var(--color-link-200))" : "none"}
				stroke={props.checked ? "oklch(var(--color-link-200))" : "currentColor"}
				stroke-width="1"
			/>
			<Show when={props.checked}>
				<circle cx="8" cy="8" r="3" fill="oklch(var(--color-fg1))" />
			</Show>
		</svg>
	);
};

export const Checkmark = (
	props: { seed: string; class?: string; style?: any },
) => {
	return (
		<img
			src={getCheckIcon(props.seed)}
			class={props.class}
			style={{ height: "1em", width: "1em", ...props.style }}
		/>
	);
};

export const XMark = (
	props: { seed: string; class?: string; style?: any },
) => {
	return (
		<img
			src={getXIcon(props.seed)}
			class={`icon ${props.class || ""}`}
			style={{ height: "1em", width: "1em", ...props.style }}
		/>
	);
};

export const Checkbox = (props: { checked?: boolean; seed?: string }) => {
	return (
		<svg
			class="radio"
			viewBox="0 0 16 16"
			aria-hidden="true"
			xmlns="http://www.w3.org/2000/svg"
		>
			<rect
				x="2"
				y="2"
				width="12"
				height="12"
				rx="2"
				fill={props.checked ? "oklch(var(--color-link-200))" : "none"}
				stroke={props.checked ? "oklch(var(--color-link-200))" : "currentColor"}
				stroke-width="1"
			/>
			<Show when={props.checked && props.seed}>
				<image
					class="icon"
					href={getCheckIcon(props.seed!)}
					style="height:12px;width:12px"
					height="12"
					width="12"
					x="2"
					y="2"
				/>
			</Show>
		</svg>
	);
};
