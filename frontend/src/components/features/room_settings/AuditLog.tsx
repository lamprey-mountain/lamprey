import type { Room } from "sdk";
import { createMemo, createSignal, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";
import { AuditLogSkeleton } from "@/components/features/audit_log/AuditLogSkeleton";
import { AUDIT_LOG_TYPE_OPTIONS } from "@/lib/audit-log-util";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const [userId, setUserId] = createSignal("");
	const [action, setAction] = createSignal("");
	const log = api.auditLog.useList(
		() => props.room.id,
		undefined,
		() => ({ user_id: userId() || undefined, type: action() || undefined }),
	);

	const members = createMemo(() => {
		const membersInRoom = Array.from(api.room_members.cache.values()).filter(
			(m) => m.room_id === props.room.id,
		);
		const userList = membersInRoom.map((member) => {
			const user = api.users.cache.get(member.user_id);
			return {
				item: member.user_id,
				label: member.override_name || user?.name || member.user_id,
			};
		});
		userList.unshift({ item: "", label: "all users" });
		return userList;
	});

	return (
		<>
			<h2>audit log</h2>
			{/* TODO: filter audit log by time range */}
			<div style="display:flex;gap:4px">
				<div>
					<h3 class="dim">user</h3>
					<Dropdown
						selected={userId()}
						onSelect={(item) => setUserId(item ?? "")}
						options={members()}
					/>
				</div>
				<div>
					<h3 class="dim">action</h3>
					<Dropdown
						selected={action()}
						onSelect={(item) => setAction(item ?? "")}
						options={AUDIT_LOG_TYPE_OPTIONS}
					/>
				</div>
			</div>
			<Show when={log()} fallback={<AuditLogSkeleton />}>
				{(l) => (
					<AuditLogList
						room_id={props.room.id}
						entries={l().state.ids.flatMap((id) => {
							const entry = api.auditLog.cache.get(id);
							return entry ? [entry] : [];
						})}
					/>
				)}
			</Show>
		</>
	);
}
