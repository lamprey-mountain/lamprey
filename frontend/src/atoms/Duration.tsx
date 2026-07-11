import { Show, type VoidProps } from "solid-js";

const hours = (ms: number) => Math.floor(ms / (1000 * 60 * 60));
const mins = (ms: number) =>
	(Math.floor(ms / (1000 * 60)) % 60).toString().padStart(2, "0");
const secs = (ms: number) =>
	(Math.floor(ms / 1000) % 60).toString().padStart(2, "0");

export const Duration = (props: VoidProps<{ ms: number | null }>) => {
	return (
		<span class="dim">
			<Show when={props.ms} fallback="--:--">
				{(ms) => (
					<>
						<Show when={hours(ms())}>
							{(hr) => (
								<>
									{hr()}
									<span class="">:</span>
								</>
							)}
						</Show>
						{mins(ms())}
						<span class="">:</span>
						{secs(ms())}
					</>
				)}
			</Show>
		</span>
	);
};
