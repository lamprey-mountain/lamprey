import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type User } from "sdk";
import {
	formatChanges,
	mergeAuditLogEntries,
	type MergedAuditLogEntry,
} from "../audit-log-util.tsx";
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
					<For each={mergeAuditLogEntries(log()!.items)}>
						{(mergedEntry) => {
							const firstEntry = mergedEntry.entries[0];
							const ts = () => getTimestampFromUUID(firstEntry.id);

							return (
								<li data-id={firstEntry.id}>
									<div
										class="info"
										onClick={() =>
											collapsed.has(firstEntry.id)
												? collapsed.delete(firstEntry.id)
												: collapsed.add(firstEntry.id)}
									>
										<div style="display:flex;gap:4px">
											<h3>{mergedEntry.type}</h3>
										</div>
										<Time date={ts()} />
									</div>
									<Show
										when={(formatChanges(props.user.id, mergedEntry).length !==
												0 ||
											mergedEntry.reason) && !collapsed.has(firstEntry.id)}
									>
										<ul class="metadata">
											<Show when={mergedEntry.reason}>
												<li>reason: {mergedEntry.reason}</li>
											</Show>
											{formatChanges(props.user.id, mergedEntry)}
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
