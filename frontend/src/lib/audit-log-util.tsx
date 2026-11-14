import { diffArrays } from "diff";
import {
	type api,
	type AuditLogChange,
	type AuditLogEntry,
	getTimestampFromUUID,
} from "sdk";
import type { JSX } from "solid-js";
import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { renderTemplate } from "@/lib/render-template.ts";

const MERGE_WINDOW_MS = 5 * 60 * 1000; // 5 minutes

type AuditLogVariant = api["AuditLogEntryType"];
type AuditLogType = AuditLogVariant["type"];
// the generated schema drops `metadata` for variants whose fields are all
// optional (e.g. SessionDeleteAll), so only consider variants that have it
type AuditLogVariantWithMetadata = Extract<
	AuditLogVariant,
	{ metadata: unknown }
>;
type AuditLogMetadata = AuditLogVariantWithMetadata["metadata"];

const AUDIT_LOG_TYPES = [
	"RoomCreate",
	"RoomUpdate",
	"ChannelCreate",
	"ChannelUpdate",
	"ChannelReorder",
	"MessageDelete",
	"MessageVersionDelete",
	"MessageDeleteBulk",
	"MessageRemove",
	"MessageRestore",
	"RoleCreate",
	"RoleUpdate",
	"RoleDelete",
	"RoleReorder",
	"InviteCreate",
	"InviteUpdate",
	"InviteDelete",
	"ReactionDeleteAll",
	"ReactionDeleteKey",
	"ReactionDeleteUser",
	"EmojiCreate",
	"EmojiUpdate",
	"EmojiDelete",
	"PermissionOverwriteCreate",
	"PermissionOverwriteUpdate",
	"PermissionOverwriteDelete",
	"MemberKick",
	"MemberBan",
	"MemberUnban",
	"MemberPrune",
	"MemberUpdate",
	"MemberDisconnect",
	"MemberDisconnectAll",
	"MemberMove",
	"RoleApply",
	"RoleUnapply",
	"BotAdd",
	"ThreadMemberAdd",
	"ThreadMemberRemove",
	"UserUpdate",
	"UserSuspend",
	"UserUnsuspend",
	"FriendRequest",
	"FriendAccept",
	"FriendDelete",
	"BlockCreate",
	"BlockDelete",
	"IgnoreAdd",
	"IgnoreRemove",
	"SessionLogin",
	"SessionUpdate",
	"SessionDelete",
	"SessionDeleteAll",
	"AuthUpdate",
	"AuthSudo",
	"ApplicationCreate",
	"ApplicationUpdate",
	"ApplicationDelete",
	"EmailCreate",
	"EmailUpdate",
	"EmailDelete",
	"ConnectionCreate",
	"ConnectionDelete",
	"UserRegistered",
	"UserDelete",
	"UserUndelete",
	"HarvestCreate",
	"AdminWhisper",
	"AdminBroadcast",
	"RoomDelete",
	"RoomUndelete",
	"RoomQuarantine",
	"RoomUnquarantine",
	"MessagePin",
	"MessageUnpin",
	"MessagePinReorder",
	"CalendarEventCreate",
	"CalendarEventUpdate",
	"CalendarEventDelete",
	"CalendarOverwriteCreate",
	"CalendarOverwriteUpdate",
	"CalendarOverwriteDelete",
	"CalendarRsvpDelete",
	"WebhookCreate",
	"WebhookUpdate",
	"WebhookDelete",
	"RatelimitUpdate",
	"RatelimitDelete",
	"RatelimitDeleteAll",
	"AutomodRuleCreate",
	"AutomodRuleUpdate",
	"AutomodRuleDelete",
	"Reindex",
	"ChannelReindex",
	"RoomReindex",
	"ReindexEverything",
	"ServerUpdate",
	"TagCreate",
	"TagUpdate",
	"TagDelete",
	"RedexCreate",
	"RedexDelete",
	"RedexVersionCreate",
	"RedexVersionDelete",
] as const satisfies readonly AuditLogType[];

// compile-time guards: the sdk types and the translations must stay in sync
type AssertNever<T extends never> = T;
type AuditLogMessages = typeof import("@/i18n/en.json")["audit_log"];
type _AllAuditLogTypesListed = AssertNever<
	Exclude<AuditLogType, (typeof AUDIT_LOG_TYPES)[number]>
>;
type _AllAuditLogTypesTranslated = AssertNever<
	Exclude<AuditLogType, keyof AuditLogMessages>
>;

const formatTypeLabel = (type: string) =>
	type.replace(/([a-z0-9])([A-Z])/g, "$1 $2").toLowerCase();

export const AUDIT_LOG_TYPE_OPTIONS = [
	{ item: "", label: "all actions" },
	...AUDIT_LOG_TYPES.map((type) => ({
		item: type,
		label: formatTypeLabel(type),
	})),
];

type MetadataOf<K extends AuditLogType> = Extract<
	AuditLogVariantWithMetadata,
	{ type: K }
>["metadata"];

// correlate `type` and `metadata` so narrowing on `type` narrows metadata
export type MergedAuditLogEntry = {
	entries: AuditLogEntry[];
	user_id: string;
	reason?: string | null;
	changes?: AuditLogChange[];
} & {
	[K in AuditLogType]: { type: K; metadata: MetadataOf<K> };
}[AuditLogType];

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

		// TODO: make each entry detail line for RedexVersionCreate say "created a new version"
		const canMerge =
			entry.type.endsWith("Update") || entry.type === "RedexVersionCreate";

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

				const metadata = metadataOf(entry);
				if ("changes" in metadata) {
					lastMerged.changes = [
						...(lastMerged.changes ?? []),
						...(metadata.changes ?? []),
					];
				}

				if (entry.reason) {
					// NOTE: should i somehow combine reasons too?
					lastMerged.reason = entry.reason;
				}

				continue;
			}
		}

		merged.push(createMergedEntry(entry));
	}

	return merged;
}

function metadataOf(
	ent: AuditLogEntry | MergedAuditLogEntry,
): AuditLogMetadata {
	return "metadata" in ent ? ent.metadata : ({} as AuditLogMetadata);
}

function createMergedEntry(entry: AuditLogEntry): MergedAuditLogEntry {
	const metadata = metadataOf(entry);
	const base = {
		entries: [entry],
		user_id: entry.user_id,
		reason: entry.reason,
		metadata,
		changes: "changes" in metadata ? metadata.changes : undefined,
	};
	// the mapped union above cannot be inferred from loose field values
	return { ...base, type: entry.type } as MergedAuditLogEntry;
}

const resolveName = (
	api2: ReturnType<typeof useApi>,
	room_id: string,
	id: string | undefined,
	type: "user" | "channel" | "role" | "webhook" | "room",
	metadataName?: string,
) => {
	if (!id) return metadataName ?? "unknown";

	switch (type) {
		case "user": {
			const roomMembers2 = api2.roomMembers;
			const member = roomMembers2.cache.get(`${room_id}:${id}`);
			if (member?.override_name) return member.override_name;
			const user = api2.users.cache.get(id);
			if (user) return user.name;
			return metadataName ?? id;
		}
		case "channel": {
			const chan = api2.channels.cache.get(id);
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

/** read a raw field out of the (union typed) audit log metadata */
function getField(metadata: AuditLogMetadata, key: string): unknown {
	return (metadata as Record<string, unknown>)[key];
}

function getStringField(
	metadata: AuditLogMetadata,
	key: string,
): string | undefined {
	const value = getField(metadata, key);
	return typeof value === "string" ? value : undefined;
}

/** reaction key params are `t:<unicode emoji>` or `c:<custom emoji id>` */
function resolveEmoji(
	api2: ReturnType<typeof useApi>,
	key: string | undefined,
): string {
	if (!key) return "unknown";
	if (key.startsWith("c:")) {
		const id = key.slice(2);
		return api2.emoji.cache.get(id)?.name ?? id;
	}
	return key.startsWith("t:") ? key.slice(2) : key;
}

/** the room a lookup should use: nicknames are keyed by the entry's own room */
function resolveRoomId(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): string {
	const firstEntry = "entries" in ent ? ent.entries[0] : ent;
	return firstEntry.room_id || room_id;
}

export function formatAuditLogEntry(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): string | JSX.Element {
	const { t } = useCtx();
	const api2 = useApi();

	const firstEntry = "entries" in ent ? ent.entries[0] : ent;
	const entryRoom = resolveRoomId(room_id, ent);
	const metadata = metadataOf(ent);

	const actor = resolveName(api2, entryRoom, firstEntry.user_id, "user");
	const messageIds = getField(metadata, "message_ids");
	const pruned = getField(metadata, "pruned");

	const params: Record<string, JSX.Element | string | number> = {
		actor,
		channel_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "channel_id"),
			"channel",
			getStringField(metadata, "channel_name"),
		),
		role_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "role_id"),
			"role",
			getStringField(metadata, "role_name"),
		),
		webhook_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "webhook_id"),
			"webhook",
			getStringField(metadata, "webhook_name"),
		),
		room_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "room_id"),
			"room",
			getStringField(metadata, "room_name"),
		),
		thread_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "thread_id"),
			"channel",
			getStringField(metadata, "thread_name"),
		),
		target: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "user_id") ??
				getStringField(metadata, "overwrite_id"),
			getStringField(metadata, "type") === "Role" ? "role" : "user",
			getStringField(metadata, "target_name"),
		),
		bot_name: resolveName(
			api2,
			entryRoom,
			getStringField(metadata, "bot_id"),
			"user",
			getStringField(metadata, "bot_name"),
		),
		emoji: resolveEmoji(api2, getStringField(metadata, "key")),
		invite_code: getStringField(metadata, "code") ?? "unknown",
		count: Array.isArray(messageIds)
			? messageIds.length
			: typeof pruned === "number"
				? pruned
				: 0,
	};

	// NOTE: never pass `params` to `t`; it stringifies JSX params and leaves
	// unresolved `{{placeholders}}` behind
	const template = t(`audit_log.${ent.type}`);
	if (!template) return `${actor} - ${ent.type}`;

	return renderTemplate(template, params);
}

// TODO: format users, roles, channels as mentions
export function formatChanges(
	room_id: string,
	ent: AuditLogEntry | MergedAuditLogEntry,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const api2 = useApi();
	const { t } = useCtx();

	const entryRoom = resolveRoomId(room_id, ent);
	const channelName = resolveName(
		api2,
		entryRoom,
		getStringField(metadataOf(ent), "channel_id"),
		"channel",
	);

	switch (ent.type) {
		case "MessageDelete":
		case "MessageVersionDelete":
		case "MessageDeleteBulk": {
			formatted.push(
				<li>{t("audit_log.changes.in_channel", { name: channelName })}</li>,
			);
			break;
		}
		case "InviteDelete": {
			formatted.push(
				<li>
					{renderTemplate(t("audit_log.changes.invite_deleted"), {
						invite_code: <em class="light">{ent.metadata.code}</em>,
					})}
				</li>,
			);
			break;
		}
		case "PermissionOverwriteCreate":
		case "PermissionOverwriteUpdate":
		case "PermissionOverwriteDelete": {
			const overwriteName = resolveName(
				api2,
				entryRoom,
				ent.metadata.overwrite_id,
				ent.metadata.type === "Role" ? "role" : "user",
			);
			formatted.push(
				<li>
					{t("audit_log.changes.permission_overwrite_for", {
						type: ent.metadata.type,
						target: overwriteName,
					})}
				</li>,
			);
			break;
		}
		case "RoleApply": {
			formatted.push(
				<li>
					{t("audit_log.changes.role_added", {
						role_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.role_id,
							"role",
						),
					})}
				</li>,
			);
			break;
		}
		case "RoleUnapply": {
			formatted.push(
				<li>
					{t("audit_log.changes.role_removed", {
						role_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.role_id,
							"role",
						),
					})}
				</li>,
			);
			break;
		}
		case "BotAdd": {
			formatted.push(
				<li>
					{t("audit_log.changes.bot_added", {
						bot_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.bot_id,
							"user",
						),
					})}
				</li>,
			);
			break;
		}
		case "MemberKick": {
			formatted.push(
				<li>
					{t("audit_log.changes.user_kicked", {
						user_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.user_id,
							"user",
						),
					})}
				</li>,
			);
			break;
		}
		case "MemberBan": {
			formatted.push(
				<li>
					{t("audit_log.changes.user_banned", {
						user_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.user_id,
							"user",
						),
					})}
				</li>,
			);
			break;
		}
		case "MemberUnban": {
			formatted.push(
				<li>
					{t("audit_log.changes.user_unbanned", {
						user_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.user_id,
							"user",
						),
					})}
				</li>,
			);
			break;
		}
		case "ThreadMemberAdd": {
			formatted.push(
				<li>
					{t("audit_log.changes.user_added_to_thread", {
						user_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.user_id,
							"user",
						),
					})}
				</li>,
			);
			formatted.push(
				<li>
					{t("audit_log.changes.to_thread", {
						channel_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.thread_id,
							"channel",
						),
					})}
				</li>,
			);
			break;
		}
		case "ThreadMemberRemove": {
			formatted.push(
				<li>
					{t("audit_log.changes.user_removed_from_thread", {
						user_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.user_id,
							"user",
						),
					})}
				</li>,
			);
			formatted.push(
				<li>
					{t("audit_log.changes.to_thread", {
						channel_name: resolveName(
							api2,
							entryRoom,
							ent.metadata.thread_id,
							"channel",
						),
					})}
				</li>,
			);
			break;
		}
	}

	const metadata = metadataOf(ent);
	const changes: AuditLogChange[] | undefined =
		("changes" in ent && ent.changes) ||
		("changes" in metadata ? metadata.changes : undefined);

	if (changes) {
		for (const c of changes) {
			if (ent.type === "RoleUpdate" && c.key === "allow") {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_granted",
						"permission_revoked",
					),
				);
			} else if (ent.type === "RoleUpdate" && c.key === "deny") {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_denied",
						"permission_unset",
					),
				);
			} else if (
				(ent.type === "PermissionOverwriteCreate" ||
					ent.type === "PermissionOverwriteUpdate") &&
				c.key === "allow"
			) {
				formatted.push(
					...renderPermissionDiff(
						(c.old ?? []) as Array<string>,
						(c.new ?? []) as Array<string>,
						"permission_granted",
						"permission_unset",
					),
				);
			} else if (
				(ent.type === "PermissionOverwriteCreate" ||
					ent.type === "PermissionOverwriteUpdate") &&
				c.key === "deny"
			) {
				formatted.push(
					...renderPermissionDiff(
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
							{t("audit_log.changes.role_added", {
								role_name: resolveName(api2, entryRoom, r, "role"),
							})}
						</li>,
					);
				}
				for (const r of removed) {
					formatted.push(
						<li>
							{t("audit_log.changes.role_removed", {
								role_name: resolveName(api2, entryRoom, r, "role"),
							})}
						</li>,
					);
				}
			} else if (c.new) {
				formatted.push(
					<li>
						{renderTemplate(t("audit_log.changes.set_field"), {
							field: <em class="light">{c.key}</em>,
							value: JSON.stringify(c.new) ?? "[null]",
						})}
					</li>,
				);
			} else {
				formatted.push(
					<li>
						{renderTemplate(t("audit_log.changes.removed_field"), {
							field: <em class="light">{c.key}</em>,
						})}
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
	addedLabel: keyof typeof import("@/i18n/en.json").audit_log.changes,
	removedLabel: keyof typeof import("@/i18n/en.json").audit_log.changes,
): Array<JSX.Element> {
	const formatted: Array<JSX.Element> = [];
	const { t } = useCtx();
	const diff = diffArrays([...oldValues].sort(), [...newValues].sort());
	const added = diff.flatMap((i) => (i.added ? i.value : []));
	const removed = diff.flatMap((i) => (i.removed ? i.value : []));

	for (const p of added) {
		formatted.push(
			<li>
				{renderTemplate(t(`audit_log.changes.${addedLabel}`), {
					permission: <em class="light">{p}</em>,
				})}
			</li>,
		);
	}
	for (const p of removed) {
		formatted.push(
			<li>
				{renderTemplate(t(`audit_log.changes.${removedLabel}`), {
					permission: <em class="light">{p}</em>,
				})}
			</li>,
		);
	}
	return formatted;
}
