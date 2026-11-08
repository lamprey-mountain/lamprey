import { ReactiveSet } from "@solid-primitives/set";
import { type AuditLogEntry, getTimestampFromUUID } from "sdk";
import { createSignal, For, Show } from "solid-js";
import { Time } from "@/atoms/Time.tsx";
import {
	formatAuditLogEntry,
	formatChanges,
	mergeAuditLogEntries,
} from "@/lib/audit-log-util";

export function AuditLogList(props: {
	room_id: string;
	entries: AuditLogEntry[];
}) {
	const [isExpanded, setIsExpanded] = createSignal(false);
	const expanded = new ReactiveSet<string>();
	const merged = () => mergeAuditLogEntries(props.entries);

	const toggle = (id: string) => {
		if (expanded.has(id)) {
			expanded.delete(id);
		} else {
			expanded.add(id);
		}
	};

	const toggleAll = () => {
		if (isExpanded()) {
			expanded.clear();
		} else {
			for (const mergedEntry of merged()) {
				expanded.add(mergedEntry.entries[0].id);
			}
		}
		setIsExpanded((b) => !b);
	};

	return (
		<>
			<div style="display:flex;gap:4px">
				<div>
					<div class="dim">&nbsp;</div>
					<button type="button" class="button" onClick={toggleAll}>
						{isExpanded() ? "collapse" : "expand"} all
					</button>
				</div>
			</div>
			<ul class="room-settings-audit-log">
				<For each={merged()}>
					{(mergedEntry) => {
						const firstEntry = mergedEntry.entries[0];
						const ts = () => getTimestampFromUUID(firstEntry.id);
						const entryDescription = () =>
							formatAuditLogEntry(props.room_id, mergedEntry);

						return (
							<li data-id={firstEntry.id}>
								{/* biome-ignore lint/a11y/useSemanticElements: styled as an expandable row */}
								<div
									class="info"
									role="button"
									tabIndex={0}
									onClick={() => toggle(firstEntry.id)}
									onKeyDown={(e) => {
										if (e.key === "Enter" || e.key === " ") {
											e.preventDefault();
											toggle(firstEntry.id);
										}
									}}
								>
									<div style="display:flex;gap:4px">
										<h3>{entryDescription()}</h3>
									</div>
									<Time date={ts()} />
								</div>
								<Show
									when={
										(formatChanges(props.room_id, mergedEntry).length !== 0 ||
											mergedEntry.reason) &&
										expanded.has(firstEntry.id)
									}
								>
									<ul class="metadata">
										<Show when={mergedEntry.reason}>
											<li>reason: {mergedEntry.reason}</li>
										</Show>
										{formatChanges(props.room_id, mergedEntry)}
									</ul>
								</Show>
							</li>
						);
					}}
				</For>
			</ul>
		</>
	);
}
