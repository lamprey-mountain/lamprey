import type { VoidProps } from "solid-js";
import { usePreferences } from "@/api";
import { tick } from "@/hooks/tick";
import { createTooltip } from "./Tooltip";

export function timeAgo(date: Date): string {
	const diff = Date.now() - +date;
	const fmt = new Intl.RelativeTimeFormat("en", {
		style: "long",
		numeric: "auto",
	});

	const MS_MINUTE = 1000 * 60;
	const MS_HOUR = MS_MINUTE * 60;
	const MS_DAY = MS_HOUR * 24;
	const MS_WEEK = MS_DAY * 7;
	const MS_MONTH = MS_DAY * 30;

	if (diff < 0) {
		const absDiff = Math.abs(diff);
		if (absDiff < MS_MINUTE) {
			return "now";
		}
		if (absDiff < MS_HOUR) {
			return fmt.format(Math.round(absDiff / MS_MINUTE), "minute");
		}
		if (absDiff < MS_DAY) {
			return fmt.format(Math.round(absDiff / MS_HOUR), "hour");
		}
		if (absDiff < MS_WEEK) {
			return fmt.format(Math.round(absDiff / MS_DAY), "day");
		}
		if (absDiff < MS_MONTH) {
			return fmt.format(Math.round(absDiff / MS_WEEK), "week");
		}
		if (absDiff < MS_DAY * 365) {
			return fmt.format(Math.round(absDiff / MS_MONTH), "month");
		}

		return "far later";
	}

	if (diff < MS_MINUTE) return "now";
	if (diff < MS_HOUR) {
		return fmt.format(-Math.round(diff / MS_MINUTE), "minute");
	}
	if (diff < MS_DAY) {
		return fmt.format(-Math.round(diff / MS_HOUR), "hour");
	}
	if (diff < MS_WEEK) {
		return fmt.format(-Math.round(diff / MS_DAY), "day");
	}
	if (diff < MS_MONTH) {
		return fmt.format(-Math.round(diff / MS_WEEK), "week");
	}
	if (diff < MS_DAY * 365) {
		return fmt.format(-Math.round(diff / MS_MONTH), "month");
	}
	return "long ago"; // fixme: i18n
}

export function formatTime(
	date: Date,
	format: TimeFormat,
	timeFormatPref?: string,
): string {
	// TODO: proper i18n support

	const TWENTYFOUR_HOUR =
		timeFormatPref === "24h" ||
		(timeFormatPref !== "12h" &&
			new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions()
				.hourCycle === "h24");

	switch (format) {
		case "relative":
			return timeAgo(date);
		case "time":
			return `${TWENTYFOUR_HOUR ? date.getHours() : date.getHours() % 12 || 12}:${date.getMinutes().toString().padStart(2, "0")}`;
		case "full":
			return new Intl.DateTimeFormat(undefined, {
				dateStyle: "medium",
				timeStyle: "medium",
				hour12: !TWENTYFOUR_HOUR,
			}).format(date);
	}
}

type TimeFormat = "relative" | "time" | "full";

type TimeProps = {
	animGroup?: string;
	class?: string;
	format?: TimeFormat;
} & ({ ts: number } | { date: Date });

export function Time(props: VoidProps<TimeProps>) {
	const preferences = usePreferences();
	const prefs = preferences.useRead();

	const date = () => ("date" in props ? props.date : new Date(props.ts));

	const tooltip = createTooltip({
		animGroup: props.animGroup,
		placement: "left-start",
		tip: () => date().toDateString(),
	});

	return (
		<time
			datetime={date().toISOString()}
			class={`time ${props.class ?? ""}`}
			ref={tooltip.content}
		>
			{
				(tick(),
				formatTime(
					date(),
					props.format ?? "relative",
					prefs.frontend.time_format,
				))
			}
		</time>
	);
}
