import { ReactiveSet } from "@solid-primitives/set";
import {
	type AuditLogEntry,
	getTimestampFromUUID,
	type Room,
	SERVER_ROOM_ID,
} from "sdk";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { Time } from "@/atoms/Time.tsx";
import {
	formatAuditLogEntry,
	formatChanges,
	mergeAuditLogEntries,
} from "@/lib/audit-log-util";

export function AuditLog(_props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.auditLog.useList(() => SERVER_ROOM_ID);
	const [isExpanded, setIsExpanded] = createSignal(false);
	const expanded = new ReactiveSet();

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
			{/* TODO: filter audit log by event type, actor, time range */}
			<div style="display:flex;gap:4px">
				<div>
					<h3 class="dim">user</h3>
					<Dropdown
						selected=""
						options={[
							{ item: "foo", label: "foo" },
							{ item: "bar", label: "bar" },
							{ item: "baz", label: "baz" },
						]}
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
							return mergeAuditLogEntries(
								l.state.ids
									.map((id) => api.auditLog.cache.get(id))
									.filter((e): e is AuditLogEntry => e !== undefined),
							);
						})()}
					>
						{(mergedEntry) => {
							const firstEntry = mergedEntry.entries[0];
							const ts = () => getTimestampFromUUID(firstEntry.id);
							const entryDescription = () =>
								formatAuditLogEntry(SERVER_ROOM_ID, mergedEntry);

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
											(formatChanges(SERVER_ROOM_ID, mergedEntry).length !==
												0 ||
												mergedEntry.reason) &&
											expanded.has(firstEntry.id)
										}
									>
										<ul class="metadata">
											<Show when={mergedEntry.reason}>
												<li>reason: {mergedEntry.reason}</li>
											</Show>
											{formatChanges(SERVER_ROOM_ID, mergedEntry)}
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
