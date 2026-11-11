import { Show } from "solid-js";
import { useApi } from "@/api";
import { AuditLogList } from "@/components/features/audit_log/AuditLogList";
import type { ApplicationDraft } from "./context";

export const Log = (props: { draft: ApplicationDraft }) => {
	const api = useApi();
	const appId = () =>
		props.draft.state === "create" ? undefined : props.draft.data.id;
	const log = api.auditLog.useUserList(appId);

	return (
		<>
			<h3>log</h3>
			<Show
				when={appId()}
				fallback={<p class="dim">save the application to see its audit log</p>}
			>
				<Show when={log()}>
					{(l) => (
						<AuditLogList
							room_id={appId()!}
							entries={l().state.ids.flatMap((id) => {
								const entry = api.auditLog.cache.get(id);
								return entry ? [entry] : [];
							})}
						/>
					)}
				</Show>
			</Show>
		</>
	);
};
