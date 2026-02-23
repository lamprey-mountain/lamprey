import { createEffect, createSignal, onMount, Show } from "solid-js";
import { createDropdown, Dropdown, type DropdownItem } from "./Dropdown";

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

export type DurationPreset = {
	label: string;
	seconds: number | "forever";
};

const defaultPresets: DurationPreset[] = [
	{ label: "60 seconds", seconds: 60 },
	{ label: "5 minutes", seconds: 300 },
	{ label: "10 minutes", seconds: 600 },
	{ label: "1 hour", seconds: 3600 },
	{ label: "1 day", seconds: 86400 },
	{ label: "1 week", seconds: 604800 },
];

type DurationInputProps = {
	value?: number | "forever" | null;
	onInput: (durationInSeconds: number | "forever") => void;
	presets?: DurationPreset[];
	showForever?: boolean;
	mount?: Element | DocumentFragment | null;
};

export const DurationInput = (props: DurationInputProps) => {
	const [text, setText] = createSignal("");
	const [isForever, setIsForever] = createSignal(false);
	const presets = () => props.presets ?? defaultPresets;
	const [customMode, setCustomMode] = createSignal(false);
	const [mountEl, setMountEl] = createSignal<Element | DocumentFragment | null>(
		null,
	);

	onMount(() => {
		// Try to find the overlay element or use the provided mount
		const overlay = document.getElementById("overlay");
		setMountEl(props.mount ?? overlay ?? document.body);
	});

	const options = (): Array<DropdownItem<number | "forever">> => {
		const opts = presets().map((p) => ({
			item: p.seconds,
			label: p.label,
		}));
		if (props.showForever) {
			opts.push({ item: "forever", label: "forever" });
		}
		opts.push({ item: "custom" as any, label: "custom..." });
		return opts;
	};

	const dropdown = createDropdown<number | "forever">({
		selected: isForever() ? "forever" : (props.value ?? undefined),
		onSelect: (item) => {
			if (item === "custom") {
				setCustomMode(true);
			} else if (item !== null) {
				setCustomMode(false);
				setIsForever(item === "forever");
				setText(item === "forever" ? "" : formatDuration(item as number));
				props.onInput(item);
			}
		},
		options,
		mount: mountEl(),
	});

	createEffect(() => {
		const v = props.value;
		if (v === undefined || v === null || customMode()) {
			if (!customMode()) setText("");
			setIsForever(false);
			return;
		}
		if (v === "forever") {
			setIsForever(true);
			setText("");
			return;
		}
		setIsForever(false);
		if (typeof v === "number" && !customMode()) {
			setText(formatDuration(v));
		}
	});

	return (
		<div class="duration-input">
			<Show
				when={!customMode()}
				fallback={
					<input
						type="text"
						value={text()}
						onInput={(e) => {
							const val = e.currentTarget.value;
							setText(val);
							const seconds = parseDuration(val);
							if (seconds !== null) {
								setIsForever(false);
								props.onInput(seconds);
							}
						}}
						onBlur={() => setCustomMode(false)}
						placeholder="enter duration: eg. 1h 30m, 5s, 10 minutes"
						style="width: 100%;"
					/>
				}
			>
				<dropdown.View />
			</Show>
		</div>
	);
};
