import { type JSX, type ParentProps } from "solid-js";

type CheckboxOptionProps = {
	id: string;
	checked: boolean;
	onChange: (checked: boolean) => void;
	seed: string;
	class?: string;
	disabled?: boolean;
	style?: JSX.CSSProperties;
};

export const CheckboxOption = (props: ParentProps<CheckboxOptionProps>) => {
	return (
		<div
			class={`option ${props.class ?? ""}`}
			tabindex="0"
			style={props.style}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") {
					e.preventDefault();
					if (!props.disabled) {
						props.onChange(!props.checked);
					}
				}
			}}
		>
			<input
				id={props.id}
				type="checkbox"
				checked={props.checked}
				onInput={(e) => props.onChange(e.currentTarget.checked)}
				style="display:none"
				disabled={props.disabled}
			/>
			{props.children}
		</div>
	);
};

type CheckboxOptionWithLabelProps = {
	id: string;
	checked: boolean;
	onChange: (checked: boolean) => void;
	seed: string;
	label: string;
	description?: string;
	class?: string;
	disabled?: boolean;
};

export const CheckboxOptionWithLabel = (
	props: CheckboxOptionWithLabelProps,
) => {
	const Checkbox = (window as any).Checkbox;
	return (
		<CheckboxOption
			id={props.id}
			checked={props.checked}
			onChange={props.onChange}
			seed={props.seed}
			class={props.class}
			disabled={props.disabled}
		>
			<Checkbox checked={props.checked} seed={props.seed} />
			<label for={props.id}>
				<div>{props.label}</div>
				{props.description && <div class="dim">{props.description}</div>}
			</label>
		</CheckboxOption>
	);
};
