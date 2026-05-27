import { Show, type VoidProps } from "solid-js";

export const Duration = (props: VoidProps<{ ms: number }>) => {
	const hours = () => Math.floor(props.ms / (1000 * 60 * 60));
	const mins = () =>
		(Math.floor(props.ms / (1000 * 60)) % 60).toString().padStart(2, "0");
	const secs = () =>
		(Math.floor(props.ms / 1000) % 60).toString().padStart(2, "0");

	return (
		<span class="dim">
			<Show when={hours()}>
				{hours()}
				<span class="">:</span>
			</Show>
			{mins()}
			<span class="">:</span>
			{secs()}
		</span>
	);
};
