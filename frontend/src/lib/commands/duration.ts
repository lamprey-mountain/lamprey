export type DurationSuggestion = {
	value: string;
	label: string;
	description: string;
};

const UNITS = [
	{ unit: "s", name: "second", seconds: 1 },
	{ unit: "m", name: "minute", seconds: 60 },
	{ unit: "h", name: "hour", seconds: 3600 },
	{ unit: "d", name: "day", seconds: 86400 },
	{ unit: "w", name: "week", seconds: 604800 },
] as const;

const PRESETS = [
	{ value: "60s", label: "1 minute" },
	{ value: "300s", label: "5 minutes" },
	{ value: "900s", label: "15 minutes" },
	{ value: "3600s", label: "1 hour" },
	{ value: "21600s", label: "6 hours" },
	{ value: "86400s", label: "1 day" },
];

const COMPOUND = /^(?:\d+(?:\.\d+)?[smhdw])+(?:\d+(?:\.\d+)?)?$/i;

/** parse `5`, `5m`, `1h30m`, `1m30` or `1d 2h 3m` into seconds */
export function parseDuration(raw: string): number | undefined {
	const trimmed = raw.trim().replace(/\s+/g, "");
	if (!trimmed) return undefined;

	if (/^\d+(?:\.\d+)?$/.test(trimmed)) return Math.round(Number(trimmed));

	const pattern = /(\d+(?:\.\d+)?)([smhdw])/gi;
	let total = 0;
	let consumed = 0;
	let match: RegExpExecArray | null = pattern.exec(trimmed);

	while (match) {
		if (match.index !== consumed) return undefined;
		const unit = UNITS.find((u) => u.unit === match?.[2].toLowerCase());
		if (!unit) return undefined;
		total += Number(match[1]) * unit.seconds;
		consumed = match.index + match[0].length;
		match = pattern.exec(trimmed);
	}

	// trailing bare number counts as seconds, e.g. `1m30`
	if (consumed < trimmed.length) {
		const tail = trimmed.slice(consumed);
		if (!/^\d+(?:\.\d+)?$/.test(tail)) return undefined;
		total += Number(tail);
	}

	return Math.round(total);
}

export function durationSuggestions(query: string): DurationSuggestion[] {
	const trimmed = query.trim();

	if (!trimmed) {
		return PRESETS.map((preset) => ({
			...preset,
			description: `${parseDuration(preset.value) ?? 0} seconds`,
		}));
	}

	// compound or already-complete durations, e.g. `1m30s`
	if (COMPOUND.test(trimmed) && /\d[smhdw]/i.test(trimmed)) {
		const seconds = parseDuration(trimmed);
		if (seconds === undefined) return [];
		return [
			{
				value: trimmed.toLowerCase(),
				label: formatDurationLabel(seconds),
				description: `${seconds} seconds`,
			},
		];
	}

	const match = /^(\d+)([smhdw]?)$/i.exec(trimmed);
	if (!match) return [];

	const amount = Number(match[1]);
	const requested = match[2]?.toLowerCase();

	return UNITS.filter((u) => !requested || u.unit === requested).map((u) => ({
		value: `${amount}${u.unit}`,
		label: `${amount} ${u.name}${amount === 1 ? "" : "s"}`,
		description: `${amount * u.seconds} seconds`,
	}));
}

function formatDurationLabel(seconds: number): string {
	let remaining = seconds;
	const parts: string[] = [];

	for (const unit of [...UNITS].reverse()) {
		if (remaining < unit.seconds) continue;
		const amount = Math.floor(remaining / unit.seconds);
		remaining -= amount * unit.seconds;
		parts.push(`${amount} ${unit.name}${amount === 1 ? "" : "s"}`);
	}

	return parts.join(" ") || "0 seconds";
}
