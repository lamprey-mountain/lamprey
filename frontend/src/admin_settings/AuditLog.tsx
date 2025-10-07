import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type Room, SERVER_ROOM_ID } from "sdk";
import { formatChanges } from "../audit-log-util.tsx";
import { Time } from "../Time.tsx";
import { ReactiveSet } from "@solid-primitives/set";
import { Dropdown } from "../Dropdown.tsx";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.audit_logs.fetch(() => SERVER_ROOM_ID);
	const collapsed = new ReactiveSet();

	return (
		<>
			<h2>audit log</h2>
			<Show when={false}>
				{/* TODO: expand/collapse audit log entries */}
				<button>expand all</button>
				<button>collapse all</button>
			</Show>
			<Show when={false}>
				{/* TODO: filter audit log by event type, actor, time range */}
				<Dropdown
					options={[
						{ item: "MessageDelete", label: "message delete" },
						{ item: "MessageVersionDelete", label: "message version delete" },
						{ item: "MessageDeleteBulk", label: "message delete bulk" },
						{ item: "ReactionPurge", label: "reaction purge" },
					]}
				/>
				<Dropdown
					options={[
						{ item: "foo", label: "foo" },
						{ item: "bar", label: "bar" },
						{ item: "baz", label: "baz" },
					]}
				/>
			</Show>
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For each={log()!.items}>
						{(entry) => {
							const user = api.users.fetch(() => entry.user_id);
							const name = user()?.name;
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
											<span>{name}</span>
										</div>
										<Time date={ts()} />
									</div>
									<Show
										when={(formatChanges(SERVER_ROOM_ID, entry).length !== 0 ||
											entry.reason) && !collapsed.has(entry.id)}
									>
										<ul class="metadata">
											<Show when={entry.reason}>
												<li>reason: {entry.reason}</li>
											</Show>
											{formatChanges(SERVER_ROOM_ID, entry)}
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
