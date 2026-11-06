import { createSignal, For, type VoidProps } from "solid-js";

export const OtpInput = (
	props: VoidProps<{
		value: string;
		/** called only once the full code has been entered */
		onInput: (value: string) => void;
		/** number of digits, defaults to 6 */
		length?: number;
		disabled?: boolean;
		error?: boolean;
		autofocus?: boolean;
	}>,
) => {
	const length = () => props.length ?? 6;
	const [display, setDisplay] = createSignal("");
	const [focused, setFocused] = createSignal(false);

	// keep only digits
	const sanitize = (v: string) => v.replace(/\D/g, "").slice(0, length());

	const handleInput = (v: string) => {
		const digits = sanitize(v);
		setDisplay(digits);
		if (digits.length === length()) props.onInput(digits);
	};

	return (
		<div
			class="otp-input"
			classList={{
				disabled: props.disabled,
				focused: focused(),
				error: props.error,
			}}
		>
			<For each={Array.from({ length: length() })}>
				{(_, i) => (
					<span
						class="segment"
						classList={{
							filled: display().length > i(),
							current: display().length === i(),
						}}
					>
						{display()[i()] ?? ""}
					</span>
				)}
			</For>
			<input
				type="text"
				inputmode="numeric"
				autocomplete="one-time-code"
				pattern="[0-9]*"
				maxLength={length()}
				value={display()}
				disabled={props.disabled}
				autofocus={props.autofocus}
				onInput={(e) => handleInput(e.currentTarget.value)}
				onFocus={() => setFocused(true)}
				onBlur={() => setFocused(false)}
			/>
		</div>
	);
};
