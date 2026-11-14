import type { User } from "sdk";
import { Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";
import { AuditLogSkeleton } from "@/components/features/audit_log/AuditLogSkeleton";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const log = api.auditLog.useUserList(() => props.user.id);

	return (
		<>
			<h2>audit log</h2>
			<div style="display:flex;gap:4px">
				{/* TODO: filter audit log by event type, actor (application?), time range */}
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
			<Show when={log()} fallback={<AuditLogSkeleton />}>
				{(l) => (
					<AuditLogList
						room_id={props.user.id}
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
