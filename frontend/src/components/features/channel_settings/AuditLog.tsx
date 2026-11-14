import type { Channel } from "sdk";
import { Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";
import { AuditLogSkeleton } from "@/components/features/audit_log/AuditLogSkeleton";

export function AuditLog(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();
	const room_id = () => props.channel.room_id ?? undefined;
	const log = api.auditLog.useList(room_id, () => props.channel.id);

	return (
		<>
			<h2>audit log</h2>
			<Show
				when={room_id()}
				fallback={
					<p class="dim">audit log is not available for this channel</p>
				}
			>
				{(rid) => (
					<Show when={log()} fallback={<AuditLogSkeleton />}>
						{(l) => (
							<AuditLogList
								room_id={rid()}
								entries={l().state.ids.flatMap((id) => {
									const entry = api.auditLog.cache.get(id);
									return entry ? [entry] : [];
								})}
							/>
						)}
					</Show>
				)}
			</Show>
		</>
	);
}
