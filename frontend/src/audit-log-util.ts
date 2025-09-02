import { type AuditLogChange, type AuditLogEntry } from "sdk";

export function getNotableChanges(ent: AuditLogEntry): AuditLogChange[] {
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
