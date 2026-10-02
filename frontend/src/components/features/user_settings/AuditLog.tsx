import { ReactiveSet } from "@solid-primitives/set";
import { getTimestampFromUUID, type User } from "sdk";
import {
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { Time } from "@/atoms/Time.tsx";
import {
	formatAuditLogEntry,
	formatChanges,
	mergeAuditLogEntries,
} from "@/lib/audit-log-util";

export function AuditLog(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const [isExpanded, setIsExpanded] = createSignal(false);
	const expanded = new ReactiveSet();

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

	const toggleAll = () => {
		if (isExpanded()) {
			collapseAll();
		} else {
			expandAll();
		}
		setIsExpanded((b) => !b);
	};

	const expandAll = () => {
		const l = log();
		if (!l) return;
		for (const mergedEntry of mergeAuditLogEntries(l.audit_log_entries)) {
			expanded.add(mergedEntry.entries[0].id);
		}
	};

	const collapseAll = () => {
		expanded.clear();
	};

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
				<div>
					<div class="dim">&nbsp;</div>
					<button type="button" class="button" onClick={toggleAll}>
						{isExpanded() ? "collapse" : "expand"} all
					</button>
				</div>
			</div>
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For
						each={(() => {
							const l = log();
							if (!l) return [];
							return mergeAuditLogEntries(l.audit_log_entries);
						})()}
					>
						{(mergedEntry) => {
							const firstEntry = mergedEntry.entries[0];
							const ts = () => getTimestampFromUUID(firstEntry.id);
							const entryDescription = () =>
								formatAuditLogEntry(props.user.id, mergedEntry);

							return (
								<li data-id={firstEntry.id}>
									<div
										class="info"
										onClick={() =>
											expanded.has(firstEntry.id)
												? expanded.delete(firstEntry.id)
												: expanded.add(firstEntry.id)
										}
									>
										<div style="display:flex;gap:4px">
											<h3>{entryDescription()}</h3>
										</div>
										<Time date={ts()} />
									</div>
									<Show
										when={
											(formatChanges(props.user.id, mergedEntry).length !== 0 ||
												mergedEntry.reason) &&
											expanded.has(firstEntry.id)
										}
									>
										<ul class="metadata">
											<Show when={mergedEntry.reason}>
												<li>reason: {mergedEntry.reason}</li>
											</Show>
											{formatChanges(props.user.id, mergedEntry)}
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
