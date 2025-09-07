import { type AuditLogChange, type AuditLogEntry } from "sdk";
import { ChangeObject, diffArrays } from "diff";
import { JSX, untrack } from "solid-js";
import { useApi } from "./api";

// TODO: resolve names, ideally without reactivity infinite loops
export function formatChanges(
	room_id: string,
	ent: AuditLogEntry,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	// const api = useApi();

	switch (ent.type) {
		case "MessageDelete":
		case "MessageVersionDelete":
		case "MessageDeleteBulk":
		case "ReactionPurge":
		case "ThreadOverwriteDelete": {
			// const thread = api.threads.fetch(() => ent.thread_id);
			// formatted.push(
			// 	<li>in {thread()?.name ?? <em>unknown thread</em>}</li>,
			// );
			formatted.push(<li>in {ent.metadata.thread_id}</li>);
			break;
		}
	}

	switch (ent.type) {
		case "MessageDeleteBulk": {
			formatted.push(
				<li>{ent.metadata.message_ids.length} messages were deleted</li>,
			);
			break;
		}
		case "InviteDelete": {
			formatted.push(
				<li>
					invite <em class="light">{ent.metadata.code}</em> was deleted
				</li>,
			);
			break;
		}
		case "ThreadOverwriteSet": {
			// const entityName = () => {
			// 	if (ent.ty === "Role") {
			// 		const role = api.roles.fetch(() => room_id, () => ent.id);
			// 		return role()?.name ?? "unknown role";
			// 	} else {
			// 		const room_member = api.room_members.fetch(
			// 			() => room_id,
			// 			() => ent.id,
			// 		);
			// 		const user = api.users.fetch(() => ent.id);
			// 		return room_member()?.override_name ?? user()?.name ?? "unknown user";
			// 	}
			// };

			// formatted.push(<li>for {ent.ty} {entityName()}</li>);
			formatted.push(
				<li>for {ent.metadata.type} {ent.metadata.overwrite_id}</li>,
			);
			break;
		}
		case "RoleApply": {
			// const role = api.roles.fetch(() => room_id, () => ent.role_id);
			// formatted.push(<li>added role {role()?.name ?? "unknown role"}</li>);
			formatted.push(<li>added role {ent.metadata.role_id}</li>);
			break;
		}
		case "RoleUnapply": {
			// const role = api.roles.fetch(() => room_id, () => ent.role_id);
			// formatted.push(<li>removed role {role()?.name ?? "unknown role"}</li>);
			formatted.push(<li>removed role {ent.metadata.role_id}</li>);
			break;
		}
		case "BotAdd": {
			// const bot = api.users.fetch(() => ent.bot_id);
			// formatted.push(<li>bot {bot()?.name ?? "unknown bot"} was added</li>);
			formatted.push(<li>bot {ent.metadata.bot_id} was added</li>);
			break;
		}
	}

	if ("changes" in ent.metadata) {
		const changes = (ent as any).metadata.changes as AuditLogChange[];
		for (const c of changes) {
			if (ent.type === "RoleUpdate" && c.key === "permissions") {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"granted permission",
						"revoked permission",
					),
				);
			} else if (ent.type === "ThreadOverwriteSet" && c.key === "allow") {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"granted permission",
						"unset permission",
					),
				);
			} else if (ent.type === "ThreadOverwriteSet" && c.key === "deny") {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"revoked permission",
						"unset permission",
					),
				);
			} else if (ent.type === "ThreadUpdate" && c.key === "deleted_at") {
				formatted.push(
					<li>{c.new ? "removed the thread" : "restored the thread"}</li>,
				);
			} else if (ent.type === "ThreadUpdate" && c.key === "archived_at") {
				formatted.push(
					<li>{c.new ? "archived the thread" : "unarchived the thread"}</li>,
				);
			} else if (
				(ent.type === "ThreadUpdate" || ent.type === "ThreadCreate") &&
				c.key === "nsfw"
			) {
				formatted.push(
					<li>{c.new ? "marked as nsfw" : "unmarked as nsfw"}</li>,
				);
			} else if (c.new) {
				formatted.push(
					<li>
						{"set "}
						<em class="light">{c.key}</em>
						{" to "}
						{JSON.stringify(c.new) ?? "[null]"}
					</li>,
				);
			} else {
				formatted.push(
					<li>
						{"removed "}
						<em class="light">{c.key}</em>
					</li>,
				);
			}
		}
	}

	return formatted;
}

function renderPermissionDiff(
	oldValues: Array<string>,
	newValues: Array<string>,
	addedLabel: string,
	removedLabel: string,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const diff = diffArrays(oldValues, newValues);
	const added = diff.flatMap((i) => i.added ? i.value : []);
	const removed = diff.flatMap((i) => i.removed ? i.value : []);

	for (const p of added) {
		formatted.push(
			<li>
				{addedLabel} <em class="light">{p}</em>
			</li>,
		);
	}
	for (const p of removed) {
		formatted.push(
			<li>
				{removedLabel} <em class="light">{p}</em>
			</li>,
		);
	}
	return formatted;
}
