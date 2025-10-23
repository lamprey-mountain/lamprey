import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type User } from "sdk";
import { formatChanges } from "../audit-log-util.tsx";
import { ReactiveSet } from "@solid-primitives/set";
import { Time } from "../Time.tsx";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const collapsed = new ReactiveSet();

	// FIXME: return newest records first
	const [log] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/audit-logs",
			{
				params: { path: { user_id: "@self" } },
			},
		);
		return data;
	});

	return (
		<>
			<h2>audit log</h2>
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For each={log()!.items}>
						{(entry) => {
							const ts = () => getTimestampFromUUID(entry.id);
							return (
								<li data-id={entry.id}>
									<div
										class="info"
										onClick={() =>
											collapsed.has(entry.id)
												? collapsed.delete(entry.id)
												: collapsed.add(entry.id)}
									>
										<div style="display:flex;gap:4px">
											<h3>{entry.type}</h3>
										</div>
										<Time date={ts()} />
									</div>
									<Show
										when={(formatChanges(props.user.id, entry).length !== 0 ||
											entry.reason) && !collapsed.has(entry.id)}
									>
										<ul class="metadata">
											<Show when={entry.reason}>
												<li>reason: {entry.reason}</li>
											</Show>
											{formatChanges(props.user.id, entry)}
										</ul>
									</Show>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</>
	);
}
