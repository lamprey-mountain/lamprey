import { ReactiveSet } from "@solid-primitives/set";
import {
	type AuditLogEntry,
	getTimestampFromUUID,
	type Room,
	SERVER_ROOM_ID,
} from "sdk";
import { For, Show, type VoidProps } from "solid-js";
import { useApi2, useAuditLog2 } from "@/api";
import { Dropdown } from "../../../atoms/Dropdown.tsx";
import { Time } from "../../../atoms/Time.tsx";
import {
	formatAuditLogEntry,
	formatChanges,
	type MergedAuditLogEntry,
	mergeAuditLogEntries,
} from "../../../audit-log-util.tsx";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api2 = useApi2();
	const auditLog2 = useAuditLog2();
	const log = auditLog2.useList(() => SERVER_ROOM_ID);
	const collapsed = new ReactiveSet();

	return (
		<>
			<h2>audit log</h2>
			<Show when={false}>
				{/* TODO: expand/collapse audit log entries */}
				<button>expand all</button>
				<button>collapse all</button>
			</Show>
			<Show when={false}>
				{/* TODO: filter audit log by event type, actor, time range */}
				<Dropdown
					options={[
						{ item: "MessageDelete", label: "message delete" },
						{ item: "MessageVersionDelete", label: "message version delete" },
						{ item: "MessageDeleteBulk", label: "message delete bulk" },
						{ item: "ReactionPurge", label: "reaction purge" },
					]}
				/>
				<Dropdown
					options={[
						{ item: "foo", label: "foo" },
						{ item: "bar", label: "bar" },
						{ item: "baz", label: "baz" },
					]}
				/>
			</Show>
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For
						each={mergeAuditLogEntries(
							log()!
								.state.ids.map((id) => auditLog2.cache.get(id))
								.filter((e): e is AuditLogEntry => e !== undefined),
						)}
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
											collapsed.has(firstEntry.id)
												? collapsed.delete(firstEntry.id)
												: collapsed.add(firstEntry.id)
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
											!collapsed.has(firstEntry.id)
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
