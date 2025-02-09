import { For, Show, VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { RoomT } from "../types.ts";

export function AuditLog(props: VoidProps<{ room: RoomT }>) {
	const api = useApi();

	const log = api.audit_logs.fetch(() => props.room.id);

	return (
		<>
			<h2>audit log</h2>
			<br />
			<Show when={log()}>
				<ul>
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
							return (
								<li data-id={entry.id}>
									<details>
										<summary>{entry.payload.type} - {name}</summary>
										<pre>{JSON.stringify(entry.payload, null, 4)}</pre>
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
