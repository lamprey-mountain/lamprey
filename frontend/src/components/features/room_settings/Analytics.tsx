import {
	createMemo,
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import type { RoomT } from "../../../types.ts";
import { useApi2 } from "@/api";
import { useCtx } from "../../../context.ts";
import { formatBytes } from "../../../media/util.tsx";
import { DateRangePicker } from "../../../atoms/Daterangepicker.tsx";
import { Dropdown } from "../../../atoms/Dropdown.tsx";
import type { Aggregation } from "@/api/services/RoomAnalyticsService.ts";

export function Metrics(props: VoidProps<{ room: RoomT }>) {
	const api2 = useApi2();

	const [aggregation, setAggregation] = createSignal<Aggregation>("Daily");
	const [dateRange, setDateRange] = createSignal({
		start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split(
			"T",
		)[0],
		end: new Date().toISOString().split("T")[0],
	});

	const [hoveredTime, setHoveredTime] = createSignal<string | null>(null);
	const [selectionStartTime, setSelectionStartTime] = createSignal<
		string | null
	>(null);

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

	const [analyticsOverview] = createResource(
		refreshData,
		(args) => api2.room_analytics.getOverview(args.room_id, args),
	);

	const [analyticsMembersCount] = createResource(
		refreshData,
		(args) => api2.room_analytics.getMembersCount(args.room_id, args),
	);

	const [analyticsMembersJoin] = createResource(
		refreshData,
		(args) => api2.room_analytics.getMembersJoin(args.room_id, args),
	);

	const [analyticsMembersLeave] = createResource(
		refreshData,
		(args) => api2.room_analytics.getMembersLeave(args.room_id, args),
	);

	const onZoom = (startBucket: string, endBucket: string) => {
		const start = startBucket;
		const end = endBucket;
		if (start === end) return;
		setDateRange({
			start: start < end ? start : end,
			end: start < end ? end : start,
		});
	};

	const resetZoom = () => {
		setDateRange({
			start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split(
				"T",
			)[0],
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

				<button onClick={resetZoom}>Reset View</button>

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
							value !== null && setAggregation(value as Aggregation)}
					/>
				</div>
			</div>

			<h3>Overview</h3>
			<Chart
				data={analyticsOverview()}
				field="message_count"
				name="Message Count"
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
			<Chart
				data={analyticsOverview()}
				field="media_count"
				name="Media Count"
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
			<Chart
				data={analyticsOverview()}
				field="media_size"
				name="Media Size"
				formatter={formatBytes}
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
			<h3>Members</h3>
			<Chart
				data={analyticsMembersCount()}
				field="count"
				name="Member Count"
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
			<Chart
				data={analyticsMembersJoin()}
				field="count"
				name="Members Joined"
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
			<Chart
				data={analyticsMembersLeave()}
				field="count"
				name="Members Left"
				hoveredTime={hoveredTime()}
				setHoveredTime={setHoveredTime}
				selectionStartTime={selectionStartTime()}
				setSelectionStartTime={setSelectionStartTime}
				onZoom={onZoom}
			/>
		</>
	);
}

type ChartProps<T> = {
	data?: Array<{ bucket: string } & T>;
	field: keyof T;
	name: string;
	formatter?: (value: number) => string;
	hoveredTime: string | null;
	setHoveredTime: (time: string | null) => void;
	selectionStartTime: string | null;
	setSelectionStartTime: (time: string | null) => void;
	onZoom: (startBucket: string, endBucket: string) => void;
};

function Chart<T extends { bucket: string }>(props: ChartProps<T>) {
	const ctx = useCtx();
	const data = createMemo(() => props.data ?? []);
	const points = createMemo(() =>
		data().map((d) => d[props.field] as unknown as number)
	);
	const maxHeight = createMemo(() => Math.max(...points(), 1));

	const scaleX = createMemo(() =>
		600 / (points().length > 1 ? points().length - 1 : 1)
	);
	const scaleY = createMemo(() => 100 / maxHeight());

	const pathStroke = createMemo(() => {
		const pts = points();
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			`M 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((d, i) => `L ${(i + 1) * sx} ${-d * sy}`),
		].join(" ");
	});

	const pathFill = createMemo(() => {
		const pts = points();
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			`M 0 0`,
			`L 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((d, i) => `L ${(i + 1) * sx} ${-d * sy}`),
			`L ${sx * (pts.length - 1)} 0`,
		].join(" ");
	});

	const ticks = createMemo(() => {
		const d = data();
		if (d.length === 0) return [];

		const count = Math.min(6, d.length);
		const result = [];
		const sx = scaleX();

		const getTime = (i: number) => new Date(d[i].bucket).getTime();
		const startTime = getTime(0);
		const endTime = getTime(d.length - 1);
		const duration = endTime - startTime;

		let formatter: (date: Date) => string;
		if (duration < 48 * 60 * 60 * 1000) {
			formatter = (date) =>
				date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
		} else if (duration < 30 * 24 * 60 * 60 * 1000) {
			formatter = (date) =>
				date.toLocaleDateString([], { month: "numeric", day: "numeric" });
		} else {
			formatter = (date) => date.toLocaleDateString();
		}

		// Pick distributed points
		for (let i = 0; i < count; i++) {
			let index;
			if (i === 0) index = 0;
			else if (i === count - 1) index = d.length - 1;
			else index = Math.floor((i * (d.length - 1)) / (count - 1));

			result.push({
				x: index * sx,
				label: formatter(new Date(d[index].bucket)),
				anchor: i === 0 ? "start" : i === count - 1 ? "end" : "middle",
			});
		}
		return result;
	});

	let svgRef: SVGSVGElement | undefined;

	const getIndexFromMouseEvent = (e: MouseEvent) => {
		if (!svgRef || data().length === 0) return null;
		const rect = svgRef.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const ratio = x / rect.width;
		const index = Math.round(ratio * (data().length - 1));
		return Math.max(0, Math.min(data().length - 1, index));
	};

	const onMouseMove = (e: MouseEvent) => {
		const index = getIndexFromMouseEvent(e);
		if (index !== null) {
			const point = data()[index];
			props.setHoveredTime(point.bucket);

			const val = props.formatter
				? props.formatter(point[props.field] as unknown as number)
				: (point[props.field] as unknown as number).toFixed(0);

			ctx.setCursorStats({
				x: e.clientX,
				y: e.clientY,
				label: `${new Date(point.bucket).toLocaleString()}: ${val}`,
			});
		}
	};

	const onMouseLeave = () => {
		props.setHoveredTime(null);
		ctx.setCursorStats(null);
	};

	const onMouseDown = (e: MouseEvent) => {
		const index = getIndexFromMouseEvent(e);
		if (index !== null) {
			props.setSelectionStartTime(data()[index].bucket);
		}
	};

	const onMouseUp = (e: MouseEvent) => {
		if (props.selectionStartTime !== null) {
			const index = getIndexFromMouseEvent(e);
			if (index !== null) {
				const endBucket = data()[index].bucket;
				if (props.selectionStartTime !== endBucket) {
					props.onZoom(props.selectionStartTime, endBucket);
				}
			}
			props.setSelectionStartTime(null);
		}
	};

	const hoveredIndex = createMemo(() => {
		if (!props.hoveredTime) return null;
		const idx = data().findIndex((d) => d.bucket === props.hoveredTime);
		return idx !== -1 ? idx : null;
	});

	const selectionStartIndex = createMemo(() => {
		if (!props.selectionStartTime) return null;
		const idx = data().findIndex((d) => d.bucket === props.selectionStartTime);
		return idx !== -1 ? idx : null;
	});

	return (
		<div class="chart-container">
			<div class="chart-header">
				<h4>{props.name}</h4>
			</div>
			<div class="chart">
				<Show when={props.data} fallback="loading...">
					{(d) => (
						<Show when={d().length > 0} fallback="No data available">
							<svg
								ref={svgRef}
								viewBox="0 -105 600 120"
								style="width: 100%; overflow: visible;"
								onMouseMove={onMouseMove}
								onMouseLeave={onMouseLeave}
								onMouseDown={onMouseDown}
								onMouseUp={onMouseUp}
							>
								<defs>
									<linearGradient
										id={`chart-gradient-${props.name.replace(/\s+/g, "-")}`}
										x1="0"
										x2="0"
										y1="0"
										y2="1"
									>
										<stop
											offset="0%"
											stop-color="oklch(var(--color-link-500) / 0.4)"
										/>
										<stop
											offset="100%"
											stop-color="oklch(var(--color-link-500) / 0.05)"
										/>
									</linearGradient>
								</defs>
								{/* Y axis labels */}
								<For each={[-25, -50, -75, -100]}>
									{(y) => (
										<>
											<line
												x1="0"
												x2="600"
												y1={y}
												y2={y}
												stroke="oklch(var(--color-sep-400))"
												stroke-width="1"
												stroke-dasharray="4 4"
											/>
											<text
												x="0"
												y={y - 2}
												fill="oklch(var(--color-fg5))"
												font-size="10"
											>
												{props.formatter
													? props.formatter(maxHeight() * (-y / 100))
													: (maxHeight() * (-y / 100)).toFixed(0)}
											</text>
										</>
									)}
								</For>

								<path
									d={pathStroke()}
									fill="none"
									stroke="oklch(var(--color-link-500))"
									stroke-width="2"
								/>
								<path
									d={pathFill()}
									fill={`url(#chart-gradient-${
										props.name.replace(/\s+/g, "-")
									})`}
								/>

								{/* Selection overlay */}
								<Show
									when={selectionStartIndex() !== null &&
										hoveredIndex() !== null}
								>
									<rect
										x={Math.min(selectionStartIndex()!, hoveredIndex()!) *
											scaleX()}
										y="-100"
										width={Math.abs(
											selectionStartIndex()! - hoveredIndex()!,
										) * scaleX()}
										height="100"
										fill="oklch(var(--color-link-500) / 0.3)"
									/>
								</Show>

								{/* Hover line */}
								<Show when={hoveredIndex() !== null}>
									<line
										x1={hoveredIndex()! * scaleX()}
										x2={hoveredIndex()! * scaleX()}
										y1="0"
										y2="-100"
										stroke="oklch(var(--color-link-500) / 0.6)"
										stroke-width="1"
									/>
									<circle
										cx={hoveredIndex()! * scaleX()}
										cy={-points()[hoveredIndex()!] * scaleY()}
										r="4"
										fill="oklch(var(--color-link-500))"
									/>
								</Show>

								{/* X axis labels */}
								<For each={ticks()}>
									{(tick) => (
										<text
											x={tick.x}
											y="15"
											fill="oklch(var(--color-fg5))"
											font-size="10"
											text-anchor={tick.anchor as "start" | "end" | "middle"}
										>
											{tick.label}
										</text>
									)}
								</For>
							</svg>
						</Show>
					)}
				</Show>
			</div>
		</div>
	);
}
