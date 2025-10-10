import type { VoidProps } from "solid-js";
import { tooltip } from "./Tooltip";

export function timeAgo(date: Date): string {
	const diff = Date.now() - (+date);
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

type TimeProps = {
	animGroup?: string;
} & ({ ts: number } | { date: Date });

export function Time(props: VoidProps<TimeProps>) {
	const date = () => "date" in props ? props.date : new Date(props.ts);

	return (
		<>
			{tooltip(
				{
					animGroup: props.animGroup,
					placement: "left-start",
				},
				date().toDateString(),
				<time datetime={date().toISOString()}>{timeAgo(date())}
				</time> as HTMLElement,
			)}
		</>
	);
}
