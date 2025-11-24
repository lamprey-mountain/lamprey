export const Colorpicker = (
	props: { onInput: (color: string) => void; value: string },
) => {
	return (
		<input
			type="color"
			value={props.value}
			onInput={(e) => props.onInput(e.currentTarget.value)}
		/>
	);
};
