import { createResource, For, Show, type VoidProps } from "solid-js";
import type { RoomT } from "../types.ts";
import { useApi } from "../api.tsx";
import { formatBytes } from "../media/util.tsx";

export function Metrics(props: VoidProps<{ room: RoomT }>) {
	const api = useApi();

	const [analyticsOverview] = createResource(
		() => props.room.id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/room/{room_id}/analytics/overview",
				{ params: { path: { room_id }, query: { aggregate: "Daily" } } },
			);
			return data;
		},
	);

	const [analyticsMembersCount] = createResource(
		() => props.room.id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/room/{room_id}/analytics/members-count",
				{ params: { path: { room_id }, query: { aggregate: "Daily" } } },
			);
			return data;
		},
	);

	const [analyticsMembersJoin] = createResource(
		() => props.room.id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/room/{room_id}/analytics/members-join",
				{ params: { path: { room_id }, query: { aggregate: "Daily" } } },
			);
			return data;
		},
	);

	const [analyticsMembersLeave] = createResource(
		() => props.room.id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/room/{room_id}/analytics/members-leave",
				{ params: { path: { room_id }, query: { aggregate: "Daily" } } },
			);
			return data;
		},
	);

	return (
		<>
			<h2>Analytics</h2>
			<h3>Overview</h3>
			<Chart
				data={analyticsOverview()}
				field="message_count"
				name="Message Count"
			/>
			<Chart
				data={analyticsOverview()}
				field="media_count"
				name="Media Count"
			/>
			<Chart
				data={analyticsOverview()}
				field="media_size"
				name="Media Size"
				formatter={formatBytes}
			/>
			<h3>Members</h3>
			<Chart
				data={analyticsMembersCount()}
				field="count"
				name="Member Count"
			/>
			<Chart
				data={analyticsMembersJoin()}
				field="count"
				name="Members Joined"
			/>
			<Chart
				data={analyticsMembersLeave()}
				field="count"
				name="Members Left"
			/>
		</>
	);
}

type ChartProps<T> = {
	data?: Array<{ bucket: string } & T>;
	field: keyof T;
	name: string;
	formatter?: (value: number) => string;
};

function Chart<T extends { bucket: string }>(props: ChartProps<T>) {
	const data = () => props.data ?? [];
	const points = () => data().map((d) => d[props.field] as unknown as number);
	const maxHeight = () => Math.max(...points(), 1);

	const scaleX = () => 600 / (points().length > 1 ? points().length - 1 : 1);
	const scaleY = () => 100 / maxHeight();

	const pathStroke = () => {
		if (points().length === 0) return "";
		return [
			`M 0 ${-points()[0] * scaleY()}`,
			...points()
				.slice(1)
				.map((d, i) => `L ${(i + 1) * scaleX()} ${-d * scaleY()}`),
		].join(" ");
	};
	const pathFill = () => {
		if (points().length === 0) return "";
		return [
			`M 0 0`,
			`L 0 ${-points()[0] * scaleY()}`,
			...points()
				.slice(1)
				.map((d, i) => `L ${(i + 1) * scaleX()} ${-d * scaleY()}`),
			`L ${scaleX() * (points().length - 1)} 0`,
		].join(" ");
	};

	return (
		<div class="chart-container">
			<h4>{props.name}</h4>
			<div class="chart">
				<Show when={props.data} fallback="loading...">
					{(data) => (
						<svg viewBox="0 -105 600 120" style="width: 100%">
							<defs>
								<linearGradient id="chart-gradient" x1="0" x2="0" y1="0" y2="1">
									<stop offset="0%" stop-color="#08f6" />
									<stop offset="100%" stop-color="#08f1" />
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
											stroke="#444"
											stroke-width="1"
										/>
										<text x="0" y={y + 12} fill="#aaa" font-size="10">
											{props.formatter
												? props.formatter(maxHeight() * (-y / 100))
												: (maxHeight() * (-y / 100)).toFixed(0)}
										</text>
									</>
								)}
							</For>
							{/* X axis labels */}
							<For
								each={Array.from(
									{ length: Math.min(10, data().length) },
									(_, i) => i,
								)}
							>
								{(i) => {
									const index = Math.floor(
										i * (data().length / Math.min(10, data().length)),
									);
									const d = data()[index];
									if (!d) return null;
									return (
										<text
											x={(index * 600) / (data().length - 1 || 1)}
											y="12"
											fill="#aaa"
											font-size="10"
											text-anchor="middle"
										>
											{new Date(d.bucket).toLocaleDateString()}
										</text>
									);
								}}
							</For>

							<path
								d={pathStroke()}
								fill="none"
								stroke="#08f"
								stroke-width="2"
							/>
							<path d={pathFill()} fill="url(#chart-gradient)" />
						</svg>
					)}
				</Show>
			</div>
		</div>
	);
}
