import type { VoidProps } from "solid-js";
import { tick } from "@/hooks/tick";
import { tooltip } from "./Tooltip";

export function timeAgo(date: Date): string {
	const diff = Date.now() - +date;
	const fmt = new Intl.RelativeTimeFormat("en", {
		style: "long",
		numeric: "auto",
	});

	if (diff < 0) {
		if (diff > -1000 * 60) {
			return "now";
		}
		if (diff > -1000 * 60 * 60) {
			return fmt.format(-Math.round(diff / (1000 * 60)), "minute");
		}
		if (diff > -1000 * 60 * 60 * 24) {
			return fmt.format(-Math.round(diff / (1000 * 60 * 60)), "hour");
		}
		if (diff > -1000 * 60 * 60 * 24 * 3000) {
			return fmt.format(-Math.round(diff / (1000 * 60 * 60 * 24)), "day");
		}

		return "far later";
	}

	if (diff < 1000 * 60) return "now"; // FIXME: i18n
	if (diff < 1000 * 60 * 60) {
		return fmt.format(-Math.round(diff / (1000 * 60)), "minute");
	}
	if (diff < 1000 * 60 * 60 * 24) {
		return fmt.format(-Math.round(diff / (1000 * 60 * 60)), "hour");
	}
	if (diff < 1000 * 60 * 60 * 24 * 3000) {
		return fmt.format(-Math.round(diff / (1000 * 60 * 60 * 24)), "day");
	}
	// if (diff < 1000 * 60 * 60 * 24 * 365) return fmt.format(Math.round(diff / (1000 * 60 * 60 * 24)), "month");
	return "long ago"; // fixme: i18n
}

export function formatTime(date: Date, format: TimeFormat): string {
	// TODO: proper i18n support

	// TODO: read from user preferences
	const TWENTYFOUR_HOUR = true;

	switch (format) {
		case "relative":
			return timeAgo(date);
		case "time":
			return `${TWENTYFOUR_HOUR ? date.getHours() : date.getHours() % 12 || 12}:${date.getMinutes().toString().padStart(2, "0")}`;
		case "full":
			return new Intl.DateTimeFormat("en", {
				dateStyle: "medium",
				timeStyle: "medium",
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
	const date = () => ("date" in props ? props.date : new Date(props.ts));

	const wrap = (
		<time datetime={date().toISOString()} class={`time ${props.class ?? ""}`}>
			{(tick(), formatTime(date(), props.format ?? "relative"))}
		</time>
	) as HTMLElement;

	return tooltip(
		{
			animGroup: props.animGroup,
			placement: "left-start",
		},
		() => date().toDateString(),
		wrap,
	);
}
