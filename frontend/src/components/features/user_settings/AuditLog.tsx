import type { User } from "sdk";
import { createSignal, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";
import { AuditLogSkeleton } from "@/components/features/audit_log/AuditLogSkeleton";
import { AUDIT_LOG_TYPE_OPTIONS } from "@/lib/audit-log-util";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const [action, setAction] = createSignal("");
	const log = api.auditLog.useUserList(
		() => props.user.id,
		() => ({
			type: action() || undefined,
		}),
	);

	return (
		<>
			<h2>audit log</h2>
			<div style="display:flex;gap:4px">
				{/* TODO: filter audit log by actor (application?) and time range */}
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
