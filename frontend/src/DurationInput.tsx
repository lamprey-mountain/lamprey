import { createEffect, createSignal, type JSX, onMount } from "solid-js";
import { createDropdown, type DropdownItem } from "./Dropdown";

const UNITS: Record<string, number> = {
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

const TIME_UNITS = [
	{ label: "w", value: 604800 },
	{ label: "d", value: 86400 },
	{ label: "h", value: 3600 },
	{ label: "m", value: 60 },
	{ label: "s", value: 1 },
];

export function parseDuration(input: string): number | null {
	const regex = /(\d*\.?\d+)\s*([a-z]+)/gi;
	let totalSeconds = 0;
	let match;
	let hasMatch = false;

	while ((match = regex.exec(input)) !== null) {
		const value = parseFloat(match[1]);
		const unit = match[2].toLowerCase();
		if (UNITS[unit]) {
			totalSeconds += value * UNITS[unit];
			hasMatch = true;
		}
	}

	if (!hasMatch) {
		const plainNumber = parseFloat(input);
		return isNaN(plainNumber) ? null : plainNumber;
	}

	return totalSeconds;
}

export function formatDuration(seconds: number): string {
	if (seconds === 0) return "0s";
	let remaining = seconds;
	const parts = [];

	for (let i = 0; i < TIME_UNITS.length; i++) {
		const unit = TIME_UNITS[i];
		const isLast = i === TIME_UNITS.length - 1;
		const count = isLast ? remaining : Math.floor(remaining / unit.value);

		if (count > 0) {
			const displayCount = isLast ? Math.round(count * 1000) / 1000 : count;
			if (displayCount > 0) {
				parts.push(`${displayCount}${unit.label}`);
			}
			remaining -= count * unit.value;
		}
	}

	return parts.join(" ") || `${seconds}s`;
}

export type DurationPreset = {
	label: string;
	seconds: number | "forever";
};

const DEFAULT_PRESETS: DurationPreset[] = [
	{ label: "60 seconds", seconds: 60 },
	{ label: "5 minutes", seconds: 300 },
	{ label: "10 minutes", seconds: 600 },
	{ label: "1 hour", seconds: 3600 },
	{ label: "1 day", seconds: 86400 },
	{ label: "1 week", seconds: 604800 },
];

type DurationInputProps = {
	value?: number | "forever" | null;
	onInput: (durationInSeconds: number | "forever" | null) => void;
	presets?: DurationPreset[];
	showForever?: boolean;
	mount?: Element | DocumentFragment | null;
	placeholder?: string;
};

export const DurationInput = (props: DurationInputProps) => {
	const [text, setText] = createSignal("");
	const [customMode, setCustomMode] = createSignal(false);
	const presets = () => props.presets ?? DEFAULT_PRESETS;

	const options = (): Array<
		DropdownItem<number | "forever" | "custom" | null>
	> => {
		const currentText = text().trim();
		const numeric = parseFloat(currentText);
		const parsed = parseDuration(currentText);

		const opts: Array<DropdownItem<number | "forever" | "custom" | null>> = [];

		if (parsed !== null && !presets().some((p) => p.seconds === parsed)) {
			opts.push({ item: parsed as any, label: currentText });
		}

		opts.push(
			...presets().map(
				(p) => ({
					item: p.seconds as any,
					label: p.label,
				}),
			),
		);

		if (!isNaN(numeric) && numeric > 0 && !/[a-z]/i.test(currentText)) {
			opts.push(
				{ item: numeric, label: `${numeric} seconds` },
				{ item: numeric * 60, label: `${numeric} minutes` },
				{ item: numeric * 3600, label: `${numeric} hours` },
				{ item: numeric * 86400, label: `${numeric} days` },
			);
		}

		if (props.showForever) {
			opts.push({ item: "forever", label: "forever" });
		}
		opts.push({ item: "custom", label: "custom..." });
		return opts;
	};

	const commit = (val: string) => {
		if (val.trim() === "") {
			props.onInput(null);
			return;
		}

		if (val.toLowerCase() === "forever" && props.showForever) {
			props.onInput("forever");
			return;
		}

		const seconds = parseDuration(val);
		if (seconds !== null) {
			props.onInput(seconds);
			const isPreset = presets().some((p) => p.seconds === seconds);
			setCustomMode(!isPreset);
			const t = formatDuration(seconds);
			setText(t);
			dropdown.setValue(t);
		} else {
			const t = props.value === "forever"
				? "forever"
				: props.value === null
				? ""
				: formatDuration(props.value as number);
			setText(t);
			dropdown.setValue(t);
		}
	};

	const dropdown = createDropdown<number | "forever" | "custom" | null>({
		get selected() {
			return (props.value ?? undefined) as any;
		},
		ignoreMissingLabel: true,
		onSelect: (item) => {
			if (item === "custom") {
				setCustomMode(true);
				setText("");
				dropdown.setValue("");
				queueMicrotask(() => {
					dropdown.open();
					dropdown.focus();
				});
			} else {
				setCustomMode(false);
				const t = item === "forever"
					? "forever"
					: item === null
					? ""
					: formatDuration(item as number);
				setText(t);
				dropdown.setValue(t);
				props.onInput(item as any);
			}
		},
		onInput: setText,
		onKeyDown: (e) => {
			if (e.key === "Enter") commit(text());
		},
		onBlur: () => commit(text()),
		options: options as any,
		get mount() {
			return (
				props.mount ?? document.getElementById("overlay") ?? document.body
			);
		},
		get placeholder() {
			return props.placeholder;
		},
	});

	createEffect((prev) => {
		const v = props.value;
		if (v === prev) return v;

		if (v === undefined || v === null) {
			if (!customMode()) {
				setText("");
				dropdown.setValue("");
			}
			dropdown.setSelected(null as any);
			return v;
		}

		const isPreset = presets().some((p) => p.seconds === v);
		setCustomMode(!isPreset);

		const t = v === "forever" ? "forever" : formatDuration(v);
		if (v === "forever") {
			if (text() !== "forever") {
				setText("forever");
				dropdown.setValue("forever");
			}
		} else if (parseDuration(text()) !== v) {
			setText(t);
			dropdown.setValue(t);
		}

		dropdown.setSelected(v as any);
		return v;
	});

	return (
		<div class="duration-input">
			<dropdown.View style="width: 100%;" />
		</div>
	);
};
