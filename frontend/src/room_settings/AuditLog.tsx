import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import {
	type AuditLogChange,
	type AuditLogEntry,
	getTimestampFromUUID,
	type Room,
} from "sdk";

export function AuditLog(props: VoidProps<{ room: Room }>) {
	const api = useApi();

	const log = api.audit_logs.fetch(() => props.room.id);

	const dateFmt = new Intl.DateTimeFormat("en-US", {
		dateStyle: "short",
		timeStyle: "short",
		hour12: false,
	});

	return (
		<>
			<h2>audit log</h2>
			<br />
			<Show when={log()}>
				<ul class="room-settings-audit-log">
					<For each={log()!.items}>
						{(entry) => {
							const member = api.room_members.fetch(() => props.room.id, () =>
								entry.user_id);
							const user = api.users.fetch(() =>
								entry.user_id
							);
							const m = member.error ? { membership: null } : member();
							const name =
								(m?.membership === "Join" ? m.override_name : null) ||
								user()?.name;
							const ts = () => getTimestampFromUUID(entry.id);
							return (
								<li data-id={entry.id}>
									<div class="info">
										<h3>{entry.type}</h3>
									</div>
									<ul>
										<li>
											<em class="light">caused by:</em> {name}
										</li>
										<li>
											<em class="light">caused at:</em>{" "}
											<time datetime={ts().toISOString()}>
												{dateFmt.format(ts())}
											</time>
										</li>
										<Show when={entry.reason}>
											<li>
												<em class="light">reason:</em> {entry.reason}
											</li>
										</Show>
									</ul>
									<br />
									<h3>changes</h3>
									<ul>
										<For each={getNotableChanges(entry as any)}>
											{(c) => {
												return (
													<li>
														<em class="light">{c.key}:</em>{" "}
														{JSON.stringify(c.old) ?? "[null]"}{" "}
														<em class="light">-&gt;</em>{" "}
														{JSON.stringify(c.new) ?? "[null]"}
													</li>
												);
											}}
										</For>
									</ul>
									<br />
									<details>
										<summary>json</summary>
										<pre>{JSON.stringify(entry, null, 4)}</pre>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</>
	);
}

function getNotableChanges(ent: AuditLogEntry): AuditLogChange[] {
	if ("changes" in ent) {
		return (ent as any).changes;
	}
	switch (ent.type) {
		case "MessageDelete":
			return [
				{
					key: "message_id",
					old: ent.message_id,
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "MessageVersionDelete":
			return [
				{
					key: "version_id",
					old: ent.version_id,
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "MessageDeleteBulk":
			return [
				{
					key: "message_ids",
					old: ent.message_ids.join(", "),
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "RoleDelete":
			return [
				{
					key: "role_id",
					old: ent.role_id,
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "InviteDelete":
			return [
				{ key: "code", old: ent.code, new: "(deleted)" } as AuditLogChange,
			];
		case "ReactionPurge":
			return [
				{
					key: "message_id",
					old: ent.message_id,
					new: "(reactions purged)",
				} as AuditLogChange,
			];
		case "EmojiDelete":
			return [
				{
					key: "emoji_id",
					old: ent.emoji_id,
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "ThreadOverwriteSet":
			return [
				{ key: "target", old: null, new: `${ent.ty} ${ent.overwrite_id}` },
				{ key: "allow", old: null, new: ent.allow.join(", ") },
				{ key: "deny", old: null, new: ent.deny.join(", ") },
			] as AuditLogChange[];
		case "ThreadOverwriteDelete":
			return [
				{
					key: "overwrite_id",
					old: ent.overwrite_id,
					new: "(deleted)",
				} as AuditLogChange,
			];
		case "MemberKick":
			return [
				{ key: "user_id", old: ent.user_id, new: "(kicked)" } as AuditLogChange,
			];
		case "MemberBan":
			return [
				{ key: "user_id", old: ent.user_id, new: "(banned)" } as AuditLogChange,
			];
		case "MemberUnban":
			return [
				{
					key: "user_id",
					old: ent.user_id,
					new: "(unbanned)",
				} as AuditLogChange,
			];
		case "RoleApply":
			return [
				{
					key: "user_id",
					old: ent.user_id,
					new: `+role ${ent.role_id}`,
				} as AuditLogChange,
			];
		case "RoleUnapply":
			return [
				{
					key: "user_id",
					old: ent.user_id,
					new: `-role ${ent.role_id}`,
				} as AuditLogChange,
			];
		case "BotAdd":
			return [
				{ key: "bot_id", old: null, new: ent.bot_id } as AuditLogChange,
			];
		default:
			return [];
	}
}
