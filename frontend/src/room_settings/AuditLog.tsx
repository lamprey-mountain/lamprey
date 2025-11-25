import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type Room } from "sdk";
import { formatChanges } from "../audit-log-util.tsx";
import { Time } from "../Time.tsx";
import { ReactiveSet } from "@solid-primitives/set";
import { Dropdown } from "../Dropdown.tsx";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.audit_logs.fetch(() => props.room.id);
	const collapsed = new ReactiveSet();

	return (
		<>
			<h2>audit log</h2>
			<Show when={false}>
				{/* TODO: expand/collapse audit log entries */}
				<button>expand all</button>
				<button>collapse all</button>
			</Show>
			{/* TODO: filter audit log by event type, actor, time range */}
			<div style="display:flex;gap:4px">
				<div>
					<h3 class="dim">user</h3>
					<Dropdown
						options={[
							{ item: "foo", label: "foo" },
							{ item: "bar", label: "bar" },
							{ item: "baz", label: "baz" },
						]}
					/>
				</div>
				<div>
					<h3 class="dim">action</h3>
					<Dropdown
						options={[
							{ item: "", label: "all actions" },
							{ item: "MessageDelete", label: "message delete" },
							{ item: "MessageVersionDelete", label: "message version delete" },
							{ item: "MessageDeleteBulk", label: "message delete bulk" },
							{ item: "ReactionPurge", label: "reaction purge" },
						]}
					/>
				</div>
			</div>
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For each={log()!.items}>
						{(entry) => {
							const member = api.room_members.fetch(() => props.room.id, () =>
								entry.user_id);
							const user = api.users.fetch(() =>
								entry.user_id
							);
							const m = member.error ? { membership: null } : member();
							const name =
								(m?.membership === "Join" ? m.override_name : null) ||
								user()?.name;
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
										when={(formatChanges(props.room.id, entry).length !== 0 ||
											entry.reason) && !collapsed.has(entry.id)}
									>
										<ul class="metadata">
											<Show when={entry.reason}>
												<li>reason: {entry.reason}</li>
											</Show>
											{formatChanges(props.room.id, entry)}
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
