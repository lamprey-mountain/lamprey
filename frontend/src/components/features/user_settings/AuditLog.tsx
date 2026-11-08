import type { User } from "sdk";
import { createResource, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();

	// FIXME: return newest records first
	// TODO: move logic to audit logs service
	const [log] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/audit-logs",
			{
				params: { path: { user_id: "@self" } },
			},
		);
		return data;
	});

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
			<Show when={log()}>
				{(l) => (
					<AuditLogList
						room_id={props.user.id}
						entries={l().audit_log_entries}
					/>
				)}
			</Show>
		</>
	);
}
