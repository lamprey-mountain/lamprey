import { Show, type VoidProps } from "solid-js";

export function UploadOverlay(
	props: VoidProps<{ progress: number | null; label: string }>,
) {
	return (
		<div class="overlay" classList={{ uploading: props.progress !== null }}>
			<Show when={props.progress !== null} fallback={props.label}>
				<ProgressRing progress={props.progress ?? 0} />
			</Show>
		</div>
	);
}

function ProgressRing(props: VoidProps<{ progress: number }>) {
	const radius = 8;
	const circumference = 2 * Math.PI * radius;

	return (
		<div class="upload-progress">
			<svg viewBox="0 0 20 20" aria-hidden="true">
				<circle class="track" cx="10" cy="10" r={radius} />
				<circle
					class="fill"
					cx="10"
					cy="10"
					r={radius}
					stroke-dasharray={`${circumference}`}
					stroke-dashoffset={`${circumference * (1 - props.progress)}`}
				/>
			</svg>
			<span>{Math.round(props.progress * 100)}%</span>
		</div>
	);
}
