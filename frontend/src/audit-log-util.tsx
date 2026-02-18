import { type AuditLogChange, type AuditLogEntry } from "sdk";
import { ChangeObject, diffArrays } from "diff";
import { JSX, untrack } from "solid-js";
import { useApi } from "./api";
import { useCtx } from "./context";

const MERGE_WINDOW_MS = 5 * 60 * 1000; // 5 minutes

export interface MergedAuditLogEntry {
	entries: AuditLogEntry[];
	user_id: string;
	type: string;
	reason?: string | null;
	metadata: any;
	changes?: AuditLogChange[];
}

/**
 * Merge audit log entries that are:
 * - Done by the same user
 * - Are of the same type (only *Update events)
 * - Are within 5 minutes of each other
 */
export function mergeAuditLogEntries(
	entries: AuditLogEntry[],
): MergedAuditLogEntry[] {
	if (entries.length === 0) return [];

	const merged: MergedAuditLogEntry[] = [];

	for (const entry of entries) {
		const lastMerged = merged[merged.length - 1];

		const canMerge = entry.type.endsWith("Update");

		if (
			canMerge &&
			lastMerged &&
			lastMerged.user_id === entry.user_id &&
			lastMerged.type === entry.type
		) {
			const lastEntry = lastMerged.entries[lastMerged.entries.length - 1];
			const lastTs = getTimestampFromUUID(lastEntry.id).getTime();
			const currentTs = getTimestampFromUUID(entry.id).getTime();

			if (currentTs - lastTs <= MERGE_WINDOW_MS) {
				lastMerged.entries.push(entry);

				if (
					"metadata" in entry &&
					"changes" in (entry as any).metadata &&
					(entry as any).metadata.changes
				) {
					if (!lastMerged.changes) {
						lastMerged.changes = [];
					}
					lastMerged.changes.push(...(entry as any).metadata.changes);
				}

				if (entry.reason) {
					// NOTE: should i somehow combine reasons too?
					lastMerged.reason = entry.reason;
				}

				continue;
			}
		}

		merged.push({
			entries: [entry],
			user_id: entry.user_id,
			type: entry.type,
			reason: entry.reason,
			metadata: (entry as any).metadata,
			changes: "metadata" in entry && "changes" in (entry as any).metadata
				? (entry as any).metadata.changes
				: undefined,
		});
	}

	return merged;
}

function getTimestampFromUUID(uuid: string): Date {
	return new Date(parseInt(uuid.substring(0, 8), 16) * 1000);
}

const resolveName = (
	api: ReturnType<typeof useApi>,
	room_id: string,
	id: string | undefined,
	type: "user" | "channel" | "role" | "webhook" | "room",
	metadataName?: string,
) => {
	if (!id) return metadataName ?? "unknown";

	switch (type) {
		case "user": {
			const member = api.room_members.cache.get(room_id)?.get(id);
			if (member?.override_name) return member.override_name;
			const user = api.users.cache.get(id);
			if (user) return user.name;
			return metadataName ?? id;
		}
		case "channel": {
			const chan = api.channels.cache.get(id);
			return chan?.name ?? metadataName ?? id;
		}
		case "role": {
			const role = api.roles.cache.get(id);
			return role?.name ?? metadataName ?? id;
		}
		case "webhook": {
			const webhook = api.webhooks.cache.get(id);
			return webhook?.name ?? metadataName ?? id;
		}
		case "room": {
			const room = api.rooms.cache.get(id);
			return room?.name ?? metadataName ?? id;
		}
	}
	return id;
};

function interpolate(template: string, params: Record<string, any>): string {
	return template.replace(
		/{{(\w+)}}/g,
		(_, key) => params[key] ?? `{{${key}}}`,
	);
}

export function formatAuditLogEntry(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): string {
	const { t } = useCtx();
	const api = useApi();

	const firstEntry = "entries" in ent ? ent.entries[0] : ent;

	const actor = resolveName(api, room_id, firstEntry.user_id, "user");

	const params: any = {
		actor,
		channel_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.channel_id,
			"channel",
			(ent as any).metadata?.channel_name,
		),
		role_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.role_id,
			"role",
			(ent as any).metadata?.role_name,
		),
		webhook_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.webhook_id,
			"webhook",
			(ent as any).metadata?.webhook_name,
		),
		room_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.room_id,
			"room",
			(ent as any).metadata?.room_name,
		),
		thread_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.thread_id,
			"channel",
			(ent as any).metadata?.thread_name,
		),
		target: resolveName(
			api,
			room_id,
			(ent as any).metadata?.user_id || (ent as any).metadata?.overwrite_id,
			(ent as any).metadata?.type === "Role" ? "role" : "user",
			(ent as any).metadata?.target_name,
		),
		bot_name: resolveName(
			api,
			room_id,
			(ent as any).metadata?.bot_id,
			"user",
			(ent as any).metadata?.bot_name,
		),
		invite_code: (ent as any).metadata?.code ?? "unknown",
		count: (ent as any).metadata?.message_ids?.length ?? 0,
	};

	const translated = (t as any)(`audit_log.${ent.type}`, params) as
		| string
		| undefined;
	if (!translated) return `${actor} - ${ent.type}`;

	return interpolate(translated, params);
}

export function formatChanges(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const api = useApi();
	const { t } = useCtx();

	const channelName = resolveName(
		api,
		room_id,
		(ent as any).metadata?.channel_id,
		"channel",
	);

	switch (ent.type) {
		case "MessageDelete":
		case "MessageVersionDelete":
		case "ReactionPurge":
		case "PermissionOverwriteDelete": {
			formatted.push(
				<li>{t("audit_log.changes.in_channel", channelName)}</li>,
			);
			break;
		}
		case "MessageDeleteBulk": {
			formatted.push(
				<li>{t("audit_log.changes.in_channel", channelName)}</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.messages_deleted",
						(ent as any).metadata?.message_ids?.length ?? 0,
					)}
				</li>,
			);
			break;
		}
		case "InviteDelete": {
			formatted.push(
				<li>
					{t("audit_log.changes.invite_deleted", (ent as any).metadata?.code)}
				</li>,
			);
			break;
		}
		case "PermissionOverwriteSet": {
			const overwriteType = (ent as any).metadata?.type ?? "unknown";
			const overwriteName = resolveName(
				api,
				room_id,
				(ent as any).metadata?.overwrite_id,
				(ent as any).metadata?.type === "Role" ? "role" : "user",
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.permission_overwrite_for",
						overwriteType,
						overwriteName,
					)}
				</li>,
			);
			break;
		}
		case "RoleApply": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.role_added",
						resolveName(api, room_id, ent.metadata.role_id, "role"),
					)}
				</li>,
			);
			break;
		}
		case "RoleUnapply": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.role_removed",
						resolveName(api, room_id, ent.metadata.role_id, "role"),
					)}
				</li>,
			);
			break;
		}
		case "BotAdd": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.bot_added",
						resolveName(api, room_id, ent.metadata.bot_id, "user"),
					)}
				</li>,
			);
			break;
		}
		case "MemberKick": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.user_kicked",
						resolveName(api, room_id, ent.metadata.user_id, "user"),
					)}
				</li>,
			);
			break;
		}
		case "MemberBan": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.user_banned",
						resolveName(api, room_id, ent.metadata.user_id, "user"),
					)}
				</li>,
			);
			break;
		}
		case "MemberUnban": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.user_unbanned",
						resolveName(api, room_id, ent.metadata.user_id, "user"),
					)}
				</li>,
			);
			break;
		}
		case "ThreadMemberAdd": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.user_added_to_thread",
						resolveName(api, room_id, ent.metadata.user_id, "user"),
					)}
				</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.to_thread",
						resolveName(
							api,
							room_id,
							ent.metadata.thread_id,
							"channel",
						),
					)}
				</li>,
			);
			break;
		}
		case "ThreadMemberRemove": {
			formatted.push(
				<li>
					{t(
						"audit_log.changes.user_removed_from_thread",
						resolveName(api, room_id, ent.metadata.user_id, "user"),
					)}
				</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.to_thread",
						resolveName(
							api,
							room_id,
							ent.metadata.thread_id,
							"channel",
						),
					)}
				</li>,
			);
			break;
		}
	}

	const changes = "changes" in ent && ent.changes
		? ent.changes
		: "changes" in (ent as any).metadata
		? ((ent as any).metadata.changes as AuditLogChange[])
		: undefined;

	if (changes) {
		for (const c of changes) {
			if (ent.type === "RoleUpdate" && c.key === "allow") {
				formatted.push(
					...renderPermissionDiff(
						api,
						room_id,
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_granted",
						"permission_revoked",
					),
				);
			} else if (ent.type === "RoleUpdate" && c.key === "deny") {
				formatted.push(
					...renderPermissionDiff(
						api,
						room_id,
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_denied",
						"permission_unset",
					),
				);
			} else if (ent.type === "PermissionOverwriteSet" && c.key === "allow") {
				formatted.push(
					...renderPermissionDiff(
						api,
						room_id,
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_granted",
						"permission_unset",
					),
				);
			} else if (ent.type === "PermissionOverwriteSet" && c.key === "deny") {
				formatted.push(
					...renderPermissionDiff(
						api,
						room_id,
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_revoked",
						"permission_unset",
					),
				);
			} else if (ent.type === "ChannelUpdate" && c.key === "deleted_at") {
				formatted.push(
					<li>
						{c.new
							? t("audit_log.changes.channel_removed")
							: t("audit_log.changes.channel_restored")}
					</li>,
				);
			} else if (ent.type === "ChannelUpdate" && c.key === "archived_at") {
				formatted.push(
					<li>
						{c.new
							? t("audit_log.changes.channel_archived")
							: t("audit_log.changes.channel_unarchived")}
					</li>,
				);
			} else if (
				(ent.type === "ChannelUpdate" || ent.type === "ChannelCreate") &&
				c.key === "nsfw"
			) {
				formatted.push(
					<li>
						{c.new
							? t("audit_log.changes.channel_marked_nsfw")
							: t("audit_log.changes.channel_unmarked_nsfw")}
					</li>,
				);
			} else if (ent.type === "RoomUpdate" && c.key === "icon") {
				if (c.old && c.new) {
					formatted.push(<li>{t("audit_log.changes.icon_changed")}</li>);
				} else if (c.old) {
					formatted.push(<li>{t("audit_log.changes.icon_removed")}</li>);
				} else if (c.new) {
					formatted.push(<li>{t("audit_log.changes.icon_added")}</li>);
				}
			} else if (ent.type === "MemberUpdate" && c.key === "roles") {
				const diff = diffArrays(
					(c.old ?? []) as Array<string>,
					(c.new ?? []) as Array<string>,
				);
				const added = diff.flatMap((i) => (i.added ? i.value : []));
				const removed = diff.flatMap((i) => (i.removed ? i.value : []));
				for (const r of added) {
					formatted.push(
						<li>
							{t(
								"audit_log.changes.role_added",
								resolveName(api, room_id, r, "role"),
							)}
						</li>,
					);
				}
				for (const r of removed) {
					formatted.push(
						<li>
							{t(
								"audit_log.changes.role_removed",
								resolveName(api, room_id, r, "role"),
							)}
						</li>,
					);
				}
			} else if (c.new) {
				formatted.push(
					<li>
						{t(
							"audit_log.changes.set_field",
							c.key,
							JSON.stringify(c.new) ?? "[null]",
						)}
					</li>,
				);
			} else {
				formatted.push(
					<li>
						{t("audit_log.changes.removed_field", c.key)}
					</li>,
				);
			}
		}
	}

	return formatted;
}

function renderPermissionDiff(
	api: ReturnType<typeof useApi>,
	room_id: string,
	oldValues: Array<string>,
	newValues: Array<string>,
	addedLabel: keyof typeof import("./i18n/en.tsx").default.audit_log.changes,
	removedLabel: keyof typeof import("./i18n/en.tsx").default.audit_log.changes,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const { t } = useCtx();
	const diff = diffArrays([...oldValues].sort(), [...newValues].sort());
	const added = diff.flatMap((i) => i.added ? i.value : []);
	const removed = diff.flatMap((i) => i.removed ? i.value : []);

	for (const p of added) {
		formatted.push(
			<li>
				{t(`audit_log.changes.${addedLabel}`, p)}
			</li>,
		);
	}
	for (const p of removed) {
		formatted.push(
			<li>
				{t(`audit_log.changes.${removedLabel}`, p)}
			</li>,
		);
	}
	return formatted;
}
