import { createEffect, createSignal } from "solid-js";

const units: Record<string, number> = {
	s: 1,
	sec: 1,
	second: 1,
	seconds: 1,
	m: 60,
	min: 60,
	minute: 60,
	minutes: 60,
	h: 3600,
	hr: 3600,
	hour: 3600,
	hours: 3600,
	d: 86400,
	day: 86400,
	days: 86400,
	w: 604800,
	week: 604800,
	weeks: 604800,
};

export function parseDuration(input: string): number | null {
	const regex = /(\d+(?:\.\d+)?)\s*([a-z]+)/gi;
	let totalSeconds = 0;
	let match;
	let hasMatch = false;

	while ((match = regex.exec(input)) !== null) {
		const value = parseFloat(match[1]);
		const unit = match[2].toLowerCase();
		if (units[unit]) {
			totalSeconds += value * units[unit];
			hasMatch = true;
		}
	}

	if (!hasMatch) {
		const plainNumber = parseFloat(input);
		if (!isNaN(plainNumber)) return plainNumber;
		return null;
	}

	return totalSeconds;
}

export function formatDuration(seconds: number): string {
	if (seconds === 0) return "0s";
	let remaining = seconds;
	const parts = [];

	const timeUnits = [
		{ label: "w", value: 604800 },
		{ label: "d", value: 86400 },
		{ label: "h", value: 3600 },
		{ label: "m", value: 60 },
		{ label: "s", value: 1 },
	];

	for (const unit of timeUnits) {
		const count = Math.floor(remaining / unit.value);
		if (count > 0) {
			parts.push(`${count}${unit.label}`);
			remaining %= unit.value;
		}
	}

	return parts.join(" ") || `${seconds}s`;
}

type DurationInputProps = {
	value?: number | string;
	onInput: (durationInSeconds: number) => void;
};

export const DurationInput = (props: DurationInputProps) => {
	const [text, setText] = createSignal("");

	createEffect(() => {
		const v = props.value;
		if (v === undefined || v === null) {
			setText("");
			return;
		}
		const currentSeconds = parseDuration(text());
		if (typeof v === "number") {
			if (v !== currentSeconds) {
				setText(formatDuration(v));
			}
		} else {
			if (v !== text()) {
				setText(v);
			}
		}
	});

	return (
		<div class="duration-input">
			<input
				type="text"
				value={text()}
				onInput={(e) => {
					const val = e.currentTarget.value;
					setText(val);
					const seconds = parseDuration(val);
					if (seconds !== null) {
						props.onInput(seconds);
					}
				}}
				placeholder="eg. 1h 30m, 5s, 10 minutes"
			/>
		</div>
	);
};
