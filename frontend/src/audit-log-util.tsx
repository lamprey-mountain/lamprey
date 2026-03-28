import { type AuditLogChange, type AuditLogEntry } from "sdk";
import { ChangeObject, diffArrays } from "diff";
import { JSX, untrack } from "solid-js";
import { useApi2, useChannels2, useRoomMembers2, useRooms2 } from "@/api";
import { useCtx } from "./context";

const MERGE_WINDOW_MS = 5 * 60 * 1000; // 5 minutes

export interface MergedAuditLogEntry {
	entries: AuditLogEntry[];
	user_id: string;
	type: string;
	reason?: string | null;
	metadata: Record<string, unknown>;
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

				// Check if entry has metadata with changes
				const entryWithMetadata = entry as {
					metadata?: { changes?: import("sdk").AuditLogChange[] };
				};
				if (entryWithMetadata.metadata?.changes) {
					if (!lastMerged.changes) {
						lastMerged.changes = [];
					}
					lastMerged.changes.push(...entryWithMetadata.metadata.changes);
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
			metadata: (entry as { metadata?: Record<string, unknown> }).metadata ??
				{},
			changes:
				(entry as { metadata?: { changes?: import("sdk").AuditLogChange[] } })
					.metadata?.changes,
		});
	}

	return merged;
}

function getTimestampFromUUID(uuid: string): Date {
	return new Date(parseInt(uuid.substring(0, 8), 16) * 1000);
}

const resolveName = (
	api2: ReturnType<typeof useApi2>,
	channels2: ReturnType<typeof useChannels2>,
	room_id: string,
	id: string | undefined,
	type: "user" | "channel" | "role" | "webhook" | "room",
	metadataName?: string,
) => {
	if (!id) return metadataName ?? "unknown";

	switch (type) {
		case "user": {
			const roomMembers2 = useRoomMembers2();
			const member = roomMembers2.cache.get(`${room_id}:${id}`);
			if (member?.override_name) return member.override_name;
			const user = api2.users.cache.get(id);
			if (user) return user.name;
			return metadataName ?? id;
		}
		case "channel": {
			const chan = channels2.cache.get(id);
			return chan?.name ?? metadataName ?? id;
		}
		case "role": {
			const role = api2.roles.cache.get(id);
			return role?.name ?? metadataName ?? id;
		}
		case "webhook": {
			const webhook = api2.webhooks.cache.get(id);
			return webhook?.name ?? metadataName ?? id;
		}
		case "room": {
			const room = api2.rooms.cache.get(id);
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
	const api2 = useApi2();
	const channels2 = useChannels2();

	const firstEntry = "entries" in ent ? ent.entries[0] : ent;

	const actor = resolveName(
		api2,
		channels2,
		room_id,
		firstEntry.user_id,
		"user",
	);

	// Helper to safely access metadata (some entry types don't have it)
	const metadata = "metadata" in ent
		? ent.metadata as Record<string, unknown> | undefined
		: undefined;
	const getMetadata = (key: string): unknown => metadata?.[key];

	const params: Record<string, JSX.Element> = {
		actor,
		channel_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("channel_id") as string | undefined,
			"channel",
			getMetadata("channel_name") as string | undefined,
		),
		role_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("role_id") as string | undefined,
			"role",
			getMetadata("role_name") as string | undefined,
		),
		webhook_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("webhook_id") as string | undefined,
			"webhook",
			getMetadata("webhook_name") as string | undefined,
		),
		room_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("room_id") as string | undefined,
			"room",
			getMetadata("room_name") as string | undefined,
		),
		thread_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("thread_id") as string | undefined,
			"channel",
			getMetadata("thread_name") as string | undefined,
		),
		target: resolveName(
			api2,
			channels2,
			room_id,
			(getMetadata("user_id") || getMetadata("overwrite_id")) as
				| string
				| undefined,
			(getMetadata("type") as string) === "Role" ? "role" : "user",
			getMetadata("target_name") as string | undefined,
		),
		bot_name: resolveName(
			api2,
			channels2,
			room_id,
			getMetadata("bot_id") as string | undefined,
			"user",
			getMetadata("bot_name") as string | undefined,
		),
		invite_code: (getMetadata("code") as string) ?? "unknown",
		count: (getMetadata("message_ids") as string[])?.length ?? 0,
	};

	const translated = t(`audit_log.${ent.type}` as any, params);
	if (!translated) return `${actor} - ${ent.type}`;

	return interpolate(translated, params);
}

export function formatChanges(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const api2 = useApi2();
	const channels2 = useChannels2();
	const { t } = useCtx();

	const entWithMetadata = ent as { metadata?: Record<string, unknown> };
	const channelName = resolveName(
		api2,
		channels2,
		room_id,
		entWithMetadata.metadata?.channel_id as string | undefined,
		"channel",
	);

	switch (ent.type) {
		case "MessageDelete":
		case "MessageVersionDelete":
		case "ReactionPurge":
		case "PermissionOverwriteDelete": {
			formatted.push(
				<li>{t("audit_log.changes.in_channel", { name: channelName })}</li>,
			);
			break;
		}
		case "MessageDeleteBulk": {
			formatted.push(
				<li>{t("audit_log.changes.in_channel", { name: channelName })}</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.messages_deleted",
						(entWithMetadata.metadata?.message_ids as string[])?.length ?? 0,
					)}
				</li>,
			);
			break;
		}
		case "InviteDelete": {
			formatted.push(
				<li>
					{t("audit_log.changes.invite_deleted", {
						invite_code: entWithMetadata.metadata?.code as string | undefined,
					})}
				</li>,
			);
			break;
		}
		case "PermissionOverwriteSet": {
			const overwriteType = ent.metadata?.type as string ?? "unknown";
			const overwriteName = resolveName(
				api2,
				channels2,
				room_id,
				ent.metadata?.overwrite_id as string | undefined,
				(ent.metadata?.type as string) === "Role" ? "role" : "user",
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.permission_overwrite_for",
						{ type: overwriteType, target: overwriteName },
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
						{
							role_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.role_id as string | undefined,
								"role",
							),
						},
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
						{
							role_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.role_id as string | undefined,
								"role",
							),
						},
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
						{
							bot_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.bot_id as string | undefined,
								"user",
							),
						},
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
						{
							user_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.user_id as string | undefined,
								"user",
							),
						},
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
						{
							user_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.user_id as string | undefined,
								"user",
							),
						},
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
						{
							user_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.user_id as string | undefined,
								"user",
							),
						},
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
						{
							user_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.user_id as string | undefined,
								"user",
							),
						},
					)}
				</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.to_thread",
						{
							channel_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.thread_id as string | undefined,
								"channel",
							),
						},
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
						{
							user_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.user_id as string | undefined,
								"user",
							),
						},
					)}
				</li>,
			);
			formatted.push(
				<li>
					{t(
						"audit_log.changes.to_thread",
						{
							channel_name: resolveName(
								api2,
								channels2,
								room_id,
								ent.metadata?.thread_id as string | undefined,
								"channel",
							),
						},
					)}
				</li>,
			);
			break;
		}
	}

	const changes = "changes" in ent && ent.changes
		? ent.changes
		: "metadata" in ent && ent.metadata && "changes" in ent.metadata
		? (ent.metadata.changes as AuditLogChange[])
		: undefined;

	if (changes) {
		for (const c of changes) {
			if (ent.type === "RoleUpdate" && c.key === "allow") {
				formatted.push(
					...renderPermissionDiff(
						api2,
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
						api2,
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
						api2,
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
						api2,
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
								{ role_name: resolveName(api2, channels2, room_id, r, "role") },
							)}
						</li>,
					);
				}
				for (const r of removed) {
					formatted.push(
						<li>
							{t(
								"audit_log.changes.role_removed",
								{ role_name: resolveName(api2, channels2, room_id, r, "role") },
							)}
						</li>,
					);
				}
			} else if (c.new) {
				formatted.push(
					<li>
						{t(
							"audit_log.changes.set_field",
							{ field: c.key, value: JSON.stringify(c.new) ?? "[null]" },
						)}
					</li>,
				);
			} else {
				formatted.push(
					<li>
						{t("audit_log.changes.removed_field", { field: c.key })}
					</li>,
				);
			}
		}
	}

	return formatted;
}

function renderPermissionDiff(
	api2: ReturnType<typeof useApi2>,
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
				{t(`audit_log.changes.${addedLabel}`, { permission: p })}
			</li>,
		);
	}
	for (const p of removed) {
		formatted.push(
			<li>
				{t(`audit_log.changes.${removedLabel}`, { permission: p })}
			</li>,
		);
	}
	return formatted;
}
