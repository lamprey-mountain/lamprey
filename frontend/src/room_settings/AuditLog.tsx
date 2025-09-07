import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { AuditLogEntry, getTimestampFromUUID, type Room } from "sdk";
import { formatChanges } from "../audit-log-util.tsx";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();

	const log = api.audit_logs.fetch(() => props.room.id);

	const dateFmt = new Intl.DateTimeFormat("en-US", {
		dateStyle: "short",
		timeStyle: "short",
		hour12: false,
	});

	return (
		<>
			<h2>audit log</h2>
			<br />
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
									<div class="info">
										<h3>{entry.type}</h3>
									</div>
									<ul>
										<li>
											<em class="light">caused by:</em> {name}
										</li>
										<li>
											<em class="light">caused at:</em>{" "}
											<time datetime={ts().toISOString()}>
												{dateFmt.format(ts())}
											</time>
										</li>
										<Show when={entry.reason}>
											<li>
												<em class="light">reason:</em> {entry.reason}
											</li>
										</Show>
									</ul>
									<br />
									<h3>info</h3>
									{formatChanges(props.room.id, entry)}
									<br />
									<details>
										<summary>json</summary>
										<pre>{JSON.stringify(entry, null, 4)}</pre>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</>
	);
}
