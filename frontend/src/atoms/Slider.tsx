import { For, Show } from "solid-js";

export type SliderProps = {
	min: number;
	max: number;
	step?: number;
	value: number;
	valueLabel: string;
	minLabel?: string;
	maxLabel?: string;
	detents?: { value: number; label: string }[];
	onInput: (value: number) => void;
};

export function Slider(props: SliderProps) {
	const listId = `slider-detents-${props.min}-${props.max}`;
	const pos = (value: number) =>
		`${((value - props.min) / (props.max - props.min)) * 100}%`;

	// TODO: don't show slider value in this component?
	return (
		<div class="slider-field">
			<div class="slider-row">
				<div class="slider-track">
					<input
						type="range"
						min={props.min}
						max={props.max}
						step={props.step}
						value={props.value}
						list={props.detents ? listId : undefined}
						onInput={(e) => props.onInput(e.currentTarget.valueAsNumber)}
					/>
					<Show when={props.detents}>
						<datalist id={listId}>
							<For each={props.detents}>
								{(detent) => (
									<option value={detent.value} label={detent.label} />
								)}
							</For>
						</datalist>
					</Show>
					<div class="slider-marks">
						<span class="mark start" style={{ left: pos(props.min) }}>
							{props.minLabel ?? props.min}
						</span>
						<For each={props.detents}>
							{(detent) => (
								<span class="mark" style={{ left: pos(detent.value) }}>
									{detent.label}
								</span>
							)}
						</For>
						<span class="mark end" style={{ left: pos(props.max) }}>
							{props.maxLabel ?? props.max}
						</span>
					</div>
				</div>
				<span class="value">{props.valueLabel}</span>
			</div>
		</div>
	);
}
