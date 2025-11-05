import { Show } from "solid-js";

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

export const Checkbox = (props: { checked?: boolean }) => {
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
			<Show when={props.checked}>
				<path
					d="M4.5 8.5L7 11l4.5-5.5"
					fill="none"
					stroke="oklch(var(--color-fg1))"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				/>
			</Show>
		</svg>
	);
};
