import { type VoidProps } from "solid-js";
// import { createResource, Show, type VoidProps } from "solid-js";
import type { RoomT } from "../types.ts";
// import { useApi } from "../api.tsx";
// import { formatBytes } from "../media/util.tsx";

export function Metrics(props: VoidProps<{ room: RoomT }>) {
	// const api = useApi();

	// const [metrics] = createResource(() => props.room.id, async (room_id) => {
	// 	const { data } = await api.client.http.GET(
	// 		"/api/v1/room/{room_id}/metrics",
	// 		{ params: { path: { room_id } } },
	// 	);
	// 	return data;
	// });

	return (
		<>
			<h2>analytics</h2>
			<p>todo</p>
		</>
	);

	// <Show when={metrics()} fallback="loading...">
	// 	<ul style="list-style: disc inside">
	// 		<li>
	// 			room disk usage: {formatBytes(metrics()!.media_size)} across{" "}
	// 			{metrics()!.media_count} files
	// 		</li>
	// 		<li>members: {metrics()!.member_count}</li>
	// 		<li>total channels: {metrics()!.channel_count}</li>
	// 		<li>
	// 			active channels: {metrics()!.active_channel_count}{" "}
	// 			(not archived or removed)
	// 		</li>
	// 		<li>messages: {metrics()?.message_count}</li>
	// 	</ul>
	// 	<br />
	// 	<details>
	// 		<summary>raw data</summary>
	// 		<pre>{JSON.stringify(metrics(), null, 2)}</pre>
	// 	</details>
	// </Show>
}
