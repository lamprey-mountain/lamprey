import { ReactiveSet } from "@solid-primitives/set";
import { getTimestampFromUUID, type Room, SERVER_ROOM_ID } from "sdk";
import { createMemo, createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { Time } from "@/atoms/Time.tsx";
import { AuditLogSkeleton } from "@/components/features/audit_log/AuditLogSkeleton";
import {
	AUDIT_LOG_TYPE_OPTIONS,
	formatAuditLogEntry,
	formatChanges,
	mergeAuditLogEntries,
} from "@/lib/audit-log-util";

export function AuditLog(_props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const [userId, setUserId] = createSignal("");
	const [action, setAction] = createSignal("");
	const log = api.auditLog.useList(
		() => SERVER_ROOM_ID,
		undefined,
		() => ({ user_id: userId() || undefined, type: action() || undefined }),
	);
	const users = createMemo(() => [
		{ item: "", label: "all users" },
		...Array.from(api.users.cache.values())
			.map((user) => ({ item: user.id, label: user.name }))
			.sort((a, b) => a.label.localeCompare(b.label)),
	]);
	const [isExpanded, setIsExpanded] = createSignal(false);
	const expanded = new ReactiveSet();
	const entries = createMemo(
		() =>
			log()?.state.ids.flatMap((id) => {
				const entry = api.auditLog.cache.get(id);
				return entry ? [entry] : [];
			}) ?? [],
	);
	const merged = createMemo(() => mergeAuditLogEntries(entries()));

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
		for (const id of l.state.ids) {
			expanded.add(id);
		}
	};

	const collapseAll = () => {
		expanded.clear();
	};

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
						options={users()}
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
				<div>
					<div class="dim">&nbsp;</div>
					<button type="button" class="button" onClick={toggleAll}>
						{isExpanded() ? "collapse" : "expand"} all
					</button>
				</div>
			</div>
			<Show when={log()} fallback={<AuditLogSkeleton controls={false} />}>
				<ul class="room-settings-audit-log">
					<For each={merged()}>
						{(mergedEntry) => {
							const firstEntry = mergedEntry.entries[0];
							const ts = () => getTimestampFromUUID(firstEntry.id);
							const entryDescription = () =>
								formatAuditLogEntry(SERVER_ROOM_ID, mergedEntry);
							const changes = createMemo(() =>
								formatChanges(SERVER_ROOM_ID, mergedEntry),
							);

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
											(changes().length !== 0 || mergedEntry.reason) &&
											expanded.has(firstEntry.id)
										}
									>
										<ul class="metadata">
											<Show when={mergedEntry.reason}>
												<li>reason: {mergedEntry.reason}</li>
											</Show>
											{changes()}
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
