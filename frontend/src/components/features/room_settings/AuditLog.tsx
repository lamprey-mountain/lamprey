import type { Room } from "sdk";
import { createEffect, createSignal, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.auditLog.useList(() => props.room.id);
	const [members, setMembers] = createSignal<
		Array<{ item: string; label: string }>
	>([]);

	createEffect(() => {
		const roomMembers = api.room_members.cache;
		const membersInRoom = Array.from(roomMembers.values()).filter(
			(m) => m.room_id === props.room.id,
		);
		if (membersInRoom) {
			const userList = membersInRoom.map((member) => {
				const user = api.users.cache.get(member.user_id);
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
			{/* TODO: filter audit log by event type, actor, time range */}
			<div style="display:flex;gap:4px">
				<div>
					<h3 class="dim">user</h3>
					<Dropdown selected="" options={members()} />
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
