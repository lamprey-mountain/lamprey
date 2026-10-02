import { ReactiveSet } from "@solid-primitives/set";
import { getTimestampFromUUID, type Room } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { Dropdown } from "@/atoms/Dropdown.tsx";
import { Time } from "@/atoms/Time.tsx";
import {
	formatAuditLogEntry,
	formatChanges,
	mergeAuditLogEntries,
} from "@/lib/audit-log-util";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();
	const log = api.auditLog.useList(() => props.room.id);
	const [members, setMembers] = createSignal<
		Array<{ item: string; label: string }>
	>([]);
	const [isExpanded, setIsExpanded] = createSignal(false);
	const expnded = new ReactiveSet();

	createEffect(() => {
		const roomMembers = api.room_members.cache;
		const membersInRoom = Array.from(roomMembers.values()).filter(
			(m) => m.room_id === props.room.id,
		);
		if (membersInRoom) {
			const userList = membersInRoom.map((member) => {
				const user = api.users.cache.get(member.user_id);
				return {
					item: member.user_id,
					label: member.override_name || user?.name || member.user_id,
				};
			});
			userList.unshift({ item: "", label: "all users" });
			setMembers(userList);
		}
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
		for (const id of l.state.ids) {
			expnded.add(id);
		}
	};

	const collapseAll = () => {
		expnded.clear();
	};

	return (
		<>
			<h2>audit log</h2>
			{/* TODO: filter audit log by event type, actor, time range */}
			<div style="display:flex;gap:4px">
				<div>
					<h3 class="dim">user</h3>
					<Dropdown selected="" options={members()} />
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
				{(l) => (
					<ul class="room-settings-audit-log">
						<For
							each={mergeAuditLogEntries(
								l().state.ids.map((id) => api.auditLog.cache.get(id)!),
							)}
						>
							{(mergedEntry) => {
								const firstEntry = mergedEntry.entries[0];
								const ts = () => getTimestampFromUUID(firstEntry.id);
								const entryDescription = () =>
									formatAuditLogEntry(props.room.id, mergedEntry);

								return (
									<li data-id={firstEntry.id}>
										<div
											class="info"
											onClick={() =>
												expnded.has(firstEntry.id)
													? expnded.delete(firstEntry.id)
													: expnded.add(firstEntry.id)
											}
										>
											<div style="display:flex;gap:4px">
												<h3>{entryDescription()}</h3>
											</div>
											<Time date={ts()} />
										</div>
										<Show
											when={
												(formatChanges(props.room.id, mergedEntry).length !==
													0 ||
													mergedEntry.reason) &&
												expnded.has(firstEntry.id)
											}
										>
											<ul class="metadata">
												<Show when={mergedEntry.reason}>
													<li>reason: {mergedEntry.reason}</li>
												</Show>
												{formatChanges(props.room.id, mergedEntry)}
											</ul>
										</Show>
									</li>
								);
							}}
						</For>
					</ul>
				)}
			</Show>
		</>
	);
}
