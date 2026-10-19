import { Show, type VoidProps } from "solid-js";

const DURATION_UNITS = [
	{ unit: "week", ms: 7 * 24 * 60 * 60 * 1000 },
	{ unit: "day", ms: 24 * 60 * 60 * 1000 },
	{ unit: "hour", ms: 60 * 60 * 1000 },
	{ unit: "minute", ms: 60 * 1000 },
] as const;

/** Formats a duration (in ms) as a localized, human-readable unit, e.g. "15 minutes". */
export function formatDuration(ms: number): string {
	const unit = DURATION_UNITS.find((u) => ms >= u.ms) ?? DURATION_UNITS[3];
	return new Intl.NumberFormat(undefined, {
		style: "unit",
		unit: unit.unit,
		unitDisplay: "long",
	}).format(Math.round(ms / unit.ms));
}

const hours = (ms: number) => Math.floor(ms / (1000 * 60 * 60));
const mins = (ms: number) =>
	(Math.floor(ms / (1000 * 60)) % 60).toString().padStart(2, "0");
const secs = (ms: number) =>
	(Math.floor(ms / 1000) % 60).toString().padStart(2, "0");

export const Duration = (
	props: VoidProps<{ ms: number | null; dim?: boolean }>,
) => {
	return (
		<span class="duration" classList={{ dim: props.dim ?? true }}>
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
