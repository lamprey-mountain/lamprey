import type { Accessor, Ref, Setter } from "solid-js";
import icSearch from "@/assets/search.png";
import { Icon } from "./Icon";

export const Search = (props: {
	ref?: Ref<HTMLInputElement>;
	placeholder?: string;
	value?: Accessor<string>;
	onInput?: (s: string) => void;
	onSubmit?: (value: string, e: KeyboardEvent) => void;
	onEscape?: () => void;
}) => {
	return (
		<div class="search">
			<Icon src={icSearch} alt="" color={null} />
			<input
				ref={props.ref}
				type="search"
				placeholder={props.placeholder}
				value={props.value?.() ?? ""}
				onInput={(e) => props.onInput?.(e.currentTarget.value)}
				onKeyDown={(e) => {
					if (e.key === "Enter") props.onSubmit?.(e.currentTarget.value, e);
					if (e.key === "Escape") props.onEscape?.();
				}}
			/>
		</div>
	);
};
