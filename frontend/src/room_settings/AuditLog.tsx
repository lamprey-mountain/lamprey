import {
	createEffect,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { getTimestampFromUUID, type Room } from "sdk";
import {
	formatChanges,
	mergeAuditLogEntries,
	type MergedAuditLogEntry,
} from "../audit-log-util.tsx";
import { Time } from "../Time.tsx";
import { ReactiveSet } from "@solid-primitives/set";
import { Dropdown } from "../Dropdown.tsx";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.audit_logs.fetch(() => props.room.id);
	const [members, setMembers] = createSignal<any[]>([]);
	const collapsed = new ReactiveSet();

	createEffect(() => {
		const roomMembers = api.room_members.list(() => props.room.id);
		if (roomMembers()) {
			const memberList = roomMembers()!.items;
			const userList = memberList.map((member: any) => {
				const user = api.users.fetch(() => member.user_id)();
				return {
					item: member.user_id,
					label: member.override_name || user?.name || member.user_id,
				};
			});
			userList.unshift({ item: "", label: "all users" });
			setMembers(userList);
		}
	});

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
						selected=""
						options={members()}
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
					<For each={mergeAuditLogEntries(log()!.items)}>
						{(mergedEntry) => {
							const firstEntry = mergedEntry.entries[0];
							const member = api.room_members.fetch(() => props.room.id, () =>
								firstEntry.user_id);
							const user = api.users.fetch(() =>
								firstEntry.user_id
							);
							const m = member.error ? { membership: null } : member();
							const name =
								(m?.membership === "Join" ? m.override_name : null) ||
								user()?.name;
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
											<span>{name}</span>
										</div>
										<Time date={ts()} />
									</div>
									<Show
										when={(formatChanges(props.room.id, mergedEntry).length !==
												0 ||
											mergedEntry.reason) && !collapsed.has(firstEntry.id)}
									>
										<ul class="metadata">
											<Show when={mergedEntry.reason}>
												<li>reason: {mergedEntry.reason}</li>
											</Show>
											{formatChanges(props.room.id, mergedEntry)}
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
