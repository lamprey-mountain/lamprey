import {
	createMemo,
	createResource,
	createSignal,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import type { Aggregation } from "@/api/services/RoomAnalyticsService.ts";
import { DateRangePicker } from "@/atoms/Daterangepicker.tsx";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { Chart, Charts } from "@/components/shared/Chart";
import { formatBytes } from "@/media/util.tsx";
import type { RoomT } from "@/types";

const HOUR = 60 * 60 * 1000;

export function RoomAnalytics(props: VoidProps<{ room: RoomT }>) {
	const api2 = useApi();

	const [aggregation, setAggregation] = createSignal<Aggregation>("Daily");
	const [dateRange, setDateRange] = createSignal({
		start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000)
			.toISOString()
			.split("T")[0],
		end: new Date().toISOString().split("T")[0],
	});

	const refreshData = () => {
		const start = dateRange().start;
		const end = dateRange().end;
		return {
			room_id: props.room.id,
			aggregate: aggregation(),
			start: start.includes("T") ? start : `${start}T00:00:00Z`,
			end: end.includes("T") ? end : `${end}T23:59:59Z`,
		};
	};

	const [analyticsOverview] = createResource(refreshData, (args) =>
		api2.room_analytics.getOverview(args.room_id, args),
	);

	const [analyticsMembersCount] = createResource(refreshData, (args) =>
		api2.room_analytics.getMembersCount(args.room_id, args),
	);

	const [analyticsMembersJoin] = createResource(refreshData, (args) =>
		api2.room_analytics.getMembersJoin(args.room_id, args),
	);

	const [analyticsMembersLeave] = createResource(refreshData, (args) =>
		api2.room_analytics.getMembersLeave(args.room_id, args),
	);

	// zoom indices come from whichever chart was dragged; all series share buckets
	const buckets = createMemo(() => {
		const datasets = [
			analyticsOverview(),
			analyticsMembersCount(),
			analyticsMembersJoin(),
			analyticsMembersLeave(),
		];
		const longest = datasets.reduce<Array<{ bucket: string }>>(
			(best, data) => (data && data.length > best.length ? data : best),
			[],
		);
		return longest.map((point) => point.bucket);
	});

	const onZoom = (startIndex: number, endIndex: number) => {
		const axis = buckets();
		const start = axis[Math.min(startIndex, endIndex)];
		const end = axis[Math.max(startIndex, endIndex)];
		if (start === undefined || end === undefined) return;
		setDateRange({ start, end });
	};

	const resetZoom = () => {
		setDateRange({
			start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000)
				.toISOString()
				.split("T")[0],
			end: new Date().toISOString().split("T")[0],
		});
	};

	return (
		<>
			<h2>Analytics</h2>

			<div class="analytics-controls">
				<DateRangePicker
					initialValue={dateRange()}
					onChange={(range) => setDateRange(range)}
				/>

				<button type="button" class="button" onClick={resetZoom}>
					Reset View
				</button>

				<div class="aggregation-selector">
					<label>Aggregation Interval:</label>
					<Dropdown
						selected={aggregation()}
						options={[
							{ item: "Hourly", label: "Hourly" },
							{ item: "Daily", label: "Daily" },
							{ item: "Weekly", label: "Weekly" },
							{ item: "Monthly", label: "Monthly" },
						]}
						onSelect={(value) =>
							value !== null && setAggregation(value as Aggregation)
						}
					/>
				</div>
			</div>

			<Charts onZoom={onZoom}>
				<h3>Overview</h3>
				<AnalyticsChart
					title="Message Count"
					data={analyticsOverview()}
					field="message_count"
					loading={analyticsOverview.loading}
				/>
				<AnalyticsChart
					title="Media Count"
					data={analyticsOverview()}
					field="media_count"
					loading={analyticsOverview.loading}
				/>
				<AnalyticsChart
					title="Media Size"
					data={analyticsOverview()}
					field="media_size"
					format={formatBytes}
					loading={analyticsOverview.loading}
				/>
				<h3>Members</h3>
				<AnalyticsChart
					title="Member Count"
					data={analyticsMembersCount()}
					field="count"
					loading={analyticsMembersCount.loading}
				/>
				<AnalyticsChart
					title="Members Joined"
					data={analyticsMembersJoin()}
					field="count"
					loading={analyticsMembersJoin.loading}
				/>
				<AnalyticsChart
					title="Members Left"
					data={analyticsMembersLeave()}
					field="count"
					loading={analyticsMembersLeave.loading}
				/>
			</Charts>
		</>
	);
}

type AnalyticsChartProps<T> = {
	title: string;
	data?: Array<{ bucket: string } & T>;
	field: keyof T;
	format?: (value: number) => string;
	loading?: boolean;
};

function AnalyticsChart<T extends { bucket: string }>(
	props: AnalyticsChartProps<T>,
) {
	const buckets = createMemo(
		() => props.data?.map((point) => point.bucket) ?? [],
	);
	const points = createMemo(
		() => props.data?.map((point) => Number(point[props.field])) ?? [],
	);
	const format = (value: number) => props.format?.(value) ?? value.toFixed(0);

	return (
		<Chart
			title={props.title}
			ariaLabel={props.title}
			points={points()}
			format={format}
			formatIndex={tickFormatter(buckets())}
			formatTooltip={(index) => ({
				label: new Date(buckets()[index]).toLocaleString(),
				value: format(points()[index]),
			})}
			emptyMessage={props.loading ? "Loading..." : "No data available"}
		/>
	);
}

const tickFormatter = (buckets: Array<string>) => {
	if (buckets.length === 0) return () => "";

	const start = new Date(buckets[0]).getTime();
	const end = new Date(buckets[buckets.length - 1]).getTime();
	const duration = end - start;

	if (duration < 48 * HOUR) {
		return (index: number) =>
			new Date(buckets[index]).toLocaleTimeString([], {
				hour: "2-digit",
				minute: "2-digit",
			});
	}
	if (duration < 30 * 24 * HOUR) {
		return (index: number) =>
			new Date(buckets[index]).toLocaleDateString([], {
				month: "numeric",
				day: "numeric",
			});
	}
	return (index: number) => new Date(buckets[index]).toLocaleDateString();
};
