import { Accessor, Setter } from "solid-js";

export const Search = (props: {
	placeholder: string;
	size: string;
	value: Accessor<string>;
	onValue: Setter<string>;
	submitted: (value: string, e: KeyboardEvent) => void;
	escaped: () => void;
}) => {
	return (
		<input
			type="search"
			placeholder={props.placeholder}
			value={props.value()}
			onInput={(e) => props.onValue(e.currentTarget.value)}
			onKeyDown={(e) => {
				if (e.key === "Enter") props.submitted(e.currentTarget.value, e);
				if (e.key === "Escape") props.escaped();
			}}
		/>
	);
};
