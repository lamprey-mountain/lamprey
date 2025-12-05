import { createResource, Show, type VoidProps } from "solid-js";
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
				{ params: { path: { room_id } } },
			);
			return data;
		},
	);

	const [analyticsMembersCount] = createResource(
		() => props.room.id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/room/{room_id}/analytics/members-count",
				{ params: { path: { room_id } } },
			);
			return data;
		},
	);

	// const [analyticsMembersCount] =
	// const [analyticsMembersLeave] =
	// const [analyticsMembersInvites] =

	return (
		<>
			<h2>analytics</h2>
			<p>todo</p>
			<h3>overview</h3>
			<Chart
				data={analyticsOverview()}
				fields={{
					media_count: {
						name: "Media count",
						description: "The total number of uploaded files",
					},
					media_size: {
						name: "Media size",
						description:
							"The combined total size of all uploaded files in this room",
					},
					message_count: { name: "Message count" },
				}}
			/>
			<h3>member count</h3>
			<Chart
				data={analyticsMembersCount()}
				fields={{
					count: { name: "Member count" },
				}}
			/>
		</>
	);
}

type ChartProps<T> = {
	// FIXME: typescript doesnt like this?
	data?: Array<{ bucket: string } & T>;
	fields: Record<keyof T, { name: string; description?: string }>;
};

function Chart<T>(props: ChartProps<T>) {
	// render data in a nice graph
	// dynamic y axis based on max

	return (
		<div class="chart">
			<Show when={props.data} fallback="loading...">{(data) => (
				<svg>
					<text>todo</text>
				</svg>
			)}</Show>
		</div>
	);
}
