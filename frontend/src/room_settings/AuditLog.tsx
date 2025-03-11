import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { type AuditLogEntry, getTimestampFromUUID, type Room } from "sdk";

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
										<h3>{entry.payload.type}</h3>
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
									<h3>changes</h3>
									<ul>
										<For each={getNotableChanges(entry)}>
											{(c) => {
												return (
													<li>
														<em class="light">{c.key}:</em>{" "}
														{String(c.old ?? "[null]")}{" "}
														<em class="light">-&gt;</em>{" "}
														{String(c.new ?? "[null]")}
													</li>
												);
											}}
										</For>
									</ul>
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

function getNotableChanges(ent: AuditLogEntry) {
	const prev = ent.payload_prev as any;
	switch (ent.payload.type) {
		case "UpsertRoom":
			return pickDiff(
				prev?.room ?? {},
				ent.payload.room,
				["name", "description"],
			);
		case "UpsertThread":
			return pickDiff(
				prev?.thread ?? {},
				ent.payload.thread,
				["name", "description", "type", "visibility", "state", "room_id"],
			);
		case "UpsertRole":
			return pickDiff(
				prev?.role ?? {},
				ent.payload.role,
				[
					"name",
					"description",
					"is_default",
					"is_mentionable",
					"is_self_applicable",
					"permissions",
				],
			);
		case "UpsertRoomMember":
			return pickDiff(
				prev?.member ?? {},
				ent.payload.member,
				["membership", "override_name", "override_description", "roles"],
			);
		case "UpsertInvite":
			return pickDiff(
				prev?.invite ?? {},
				ent.payload.invite,
				["expires_at", "target"],
			);
		default:
			return diff(prev ?? {}, ent.payload);
	}
}

function pickDiff<T extends Record<string, unknown>, K extends keyof T>(
	a: T,
	b: T,
	keys: Array<K>,
) {
	return diff(
		pick(a, keys),
		pick(b, keys),
	);
}

function pick<T extends Record<string, unknown>>(
	obj: T,
	keys: Array<keyof T>,
): Exclude<T, typeof keys[number]> {
	const out = {} as any;
	for (const key of keys) {
		out[key] = obj[key];
	}
	return out;
}

function diff(
	a: Record<string, unknown>,
	b: Record<string, unknown>,
): Array<{ key: string; old: unknown; new: unknown }> {
	const changes = [];
	for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
		if (typeof a[key] === "object" || typeof b[key] === "object") {
			changes.push(
				...diff(
					a[key] as Record<string, unknown> ?? {},
					b[key] as Record<string, unknown> ?? {},
				)
					.map((c) => ({ ...c, key: `${key}.${c.key}` })),
			);
		} else if (a[key] !== b[key]) {
			changes.push({ key, old: a[key], new: b[key] });
		}
	}
	return changes;
}
