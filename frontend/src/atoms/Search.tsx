import type { Accessor, JSX, Ref, Setter } from "solid-js";
import icSearch from "@/assets/search.png";
import { Icon } from "./Icon";

export const Search = (props: {
	ref?: Ref<HTMLInputElement>;
	placeholder?: string;
	ariaLabel?: string;
	value?: Accessor<string>;
	onInput?: (s: string) => void;
	onSubmit?: (value: string, e: KeyboardEvent) => void;
	onEscape?: () => void;
	onKeyDown?: JSX.EventHandler<HTMLInputElement, KeyboardEvent>;
	class?: string;
}) => {
	return (
		<div
			class={props.class}
			classList={{ search: true }}
			data-focus-ring-target
		>
			<Icon src={icSearch} alt="" color={null} />
			<input
				ref={props.ref}
				type="search"
				placeholder={props.placeholder}
				aria-label={props.ariaLabel ?? props.placeholder}
				value={props.value?.() ?? ""}
				onInput={(e) => props.onInput?.(e.currentTarget.value)}
				onKeyDown={(e) => {
					if (e.key === "Enter") props.onSubmit?.(e.currentTarget.value, e);
					if (e.key === "Escape") props.onEscape?.();
					props.onKeyDown?.(e);
				}}
			/>
		</div>
	);
};
