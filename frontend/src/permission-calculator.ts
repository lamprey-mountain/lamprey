import type { Api } from "./api";
import type {
	Channel,
	Permission,
	PermissionOverwriteType,
	Role,
	Room,
	RoomMember,
} from "sdk";

export interface PermissionContext {
	api: Api;
	room_id?: string;
	channel_id?: string;
}

export interface ResolvedPermissions {
	permissions: Set<Permission>;
	rank: number;
	timedOut: boolean;
	quarantined: boolean;
	lurker: boolean;
}

/**
 * Check if a permission is allowed for timed out users
 */
function isAllowedForTimedOut(perm: Permission): boolean {
	return perm === "ViewChannel" || perm === "ViewAuditLog";
}

/**
 * Check if a permission is allowed for quarantined users
 */
function isAllowedForQuarantined(perm: Permission): boolean {
	return perm === "ViewChannel" || perm === "ViewAuditLog" ||
		perm === "MemberNickname";
}

/**
 * Check if a permission is allowed for lurkers (non-members in public rooms)
 */
function isAllowedForLurker(perm: Permission): boolean {
	return perm === "ViewChannel" || perm === "ViewAuditLog";
}

/**
 * Apply timeout restrictions to permissions
 */
function applyTimedOutRestrictions(perms: Set<Permission>): Set<Permission> {
	const restricted = new Set<Permission>();
	for (const perm of perms) {
		if (isAllowedForTimedOut(perm)) {
			restricted.add(perm);
		}
	}
	return restricted;
}

/**
 * Apply quarantine restrictions to permissions
 */
function applyQuarantinedRestrictions(perms: Set<Permission>): Set<Permission> {
	const restricted = new Set<Permission>();
	for (const perm of perms) {
		if (isAllowedForQuarantined(perm)) {
			restricted.add(perm);
		}
	}
	return restricted;
}

/**
 * Apply lurker restrictions to permissions
 */
function applyLurkerRestrictions(perms: Set<Permission>): Set<Permission> {
	const restricted = new Set<Permission>();
	for (const perm of perms) {
		if (isAllowedForLurker(perm)) {
			restricted.add(perm);
		}
	}
	return restricted;
}

/**
 * Calculate the final permissions for a user in a given context
 */
export function calculatePermissions(
	ctx: PermissionContext,
	user_id: string,
): ResolvedPermissions {
	if (!ctx.room_id) {
		// For non-room channels (DMs, GDMS), we'll allow some basic permissions
		const defaultPermissions: Permission[] = [
			"EmojiUseExternal",
			"InviteCreate",
			"MessageCreate",
			"MessageEmbeds",
			"MessageMassMention",
			"MessageAttachments",
			"MessageMove",
			"MessagePin",
			"ReactionAdd",
			"TagApply",
			"ThreadCreatePublic",
			"ThreadCreatePrivate",
			"ChannelEdit",
			"ViewAuditLog",
			"VoiceConnect",
			"VoiceSpeak",
			"VoiceVideo",
		];
		return { permissions: new Set(defaultPermissions), rank: 0 };
	}

	const room = ctx.api.rooms.fetch(() => ctx.room_id!)();
	if (room?.owner_id === user_id) {
		// owners have full permissions (ViewChannel and Admin)
		const ownerPerms = new Set<Permission>(["ViewChannel", "Admin"]);
		return { permissions: ownerPerms, rank: Infinity };
	}

	const member = ctx.api.room_members.fetch(
		() => ctx.room_id!,
		() => user_id,
	)();
	const rolesResource = ctx.api.roles.list(() => ctx.room_id!);

	// handle non-members
	if (!room || !member || !rolesResource) {
		if (room?.public) {
			const everyoneRole = rolesResource()?.items.find((r) =>
				r.id === ctx.room_id
			);
			if (everyoneRole) {
				const perms = new Set<Permission>();
				for (const p of everyoneRole.allow) {
					perms.add(p);
				}
				for (const p of everyoneRole.deny) {
					perms.delete(p);
				}
				// Apply lurker restrictions for non-members
				const restricted = applyLurkerRestrictions(perms);
				return {
					permissions: restricted,
					rank: 0,
					timedOut: false,
					quarantined: false,
					lurker: true,
				};
			}
		}
		return {
			permissions: new Set(),
			rank: 0,
			timedOut: false,
			quarantined: false,
			lurker: false,
		};
	}

	const roles = rolesResource()?.items;
	if (!roles) {
		return {
			permissions: new Set(),
			rank: 0,
			timedOut: false,
			quarantined: false,
			lurker: false,
		};
	}

	const allowed: Permission[] = [];
	const denied: Permission[] = [];

	const everyoneRoleId = ctx.room_id;

	for (const role of roles) {
		if (role.id === everyoneRoleId || member.roles.includes(role.id)) {
			allowed.push(...role.allow);
			denied.push(...role.deny);
		}
	}

	const perms = new Set<Permission>();

	for (const p of allowed) {
		perms.add(p);
	}

	if (perms.has("Admin")) {
		const rank = calculateRank(roles, member.roles);
		return {
			permissions: perms,
			rank,
			timedOut: false,
			quarantined: false,
			lurker: false,
		};
	}

	for (const p of denied) {
		perms.delete(p);
	}

	// Check if user is timed out
	const isTimedOut = member.timeout_until
		? new Date(member.timeout_until).getTime() > Date.now()
		: false;

	// Apply timeout restrictions
	if (isTimedOut) {
		const restricted = applyTimedOutRestrictions(perms);
		return {
			permissions: restricted,
			rank: calculateRank(roles, member.roles),
			timedOut: true,
			quarantined: false,
			lurker: false,
		};
	}

	// Apply quarantine restrictions
	if (member.quarantined) {
		const restricted = applyQuarantinedRestrictions(perms);
		return {
			permissions: restricted,
			rank: calculateRank(roles, member.roles),
			timedOut: false,
			quarantined: true,
			lurker: false,
		};
	}

	if (ctx.channel_id) {
		applyChannelPermissions(perms, ctx, member);
	}

	const rank = calculateRank(roles, member.roles);

	return {
		permissions: perms,
		rank,
		timedOut: false,
		quarantined: false,
		lurker: false,
	};
}

/**
 * Calculate the rank (highest role position) for a member
 */
function calculateRank(roles: Role[], memberRoleIds: string[]): number {
	let rank = 0;
	for (const roleId of memberRoleIds) {
		const role = roles.find((r) => r.id === roleId);
		if (role) {
			rank = Math.max(rank, role.position ?? 0);
		}
	}
	return rank;
}

/**
 * Apply channel-specific permission overwrites
 */
function applyChannelPermissions(
	perms: Set<Permission>,
	ctx: PermissionContext,
	member: RoomMember,
) {
	const channel = ctx.api.channels.fetch(() => ctx.channel_id!)();
	if (!channel) return;

	if (channel.parent_id) {
		const parentChannel = ctx.api.channels.fetch(() => channel.parent_id!)();
		if (parentChannel) {
			applyChannelOverwrites(perms, parentChannel, member, ctx.room_id!);
		}
	}

	applyChannelOverwrites(perms, channel, member, ctx.room_id!);
}

/**
 * Apply permission overwrites for a single channel
 * Order: everyone allow, everyone deny, role allow, role deny, user allow, user deny
 */
function applyChannelOverwrites(
	perms: Set<Permission>,
	channel: Channel,
	member: RoomMember,
	room_id: string,
) {
	// handle locked channels/threads
	if (channel.locked && typeof channel.locked === "object") {
		const locked = channel.locked;
		const isExpired = locked.until
			? new Date(locked.until).getTime() <= Date.now()
			: false;

		if (!isExpired) {
			// Channel is locked, check if user can bypass
			const canBypass = locked.allow_roles?.some((roleId) =>
				member.roles.includes(roleId)
			);

			if (canBypass) {
				perms.add("LockedBypass" as Permission);
			}
		}
	} else if (typeof channel.locked === "boolean" && channel.locked) {
		// Legacy boolean locked state
		perms.add("ChannelLocked" as Permission);
	}

	if (
		!channel.permission_overwrites || channel.permission_overwrites.length === 0
	) {
		return;
	}

	const memberRoleIds = new Set(member.roles);

	// 1. Apply everyone allows
	for (const ow of channel.permission_overwrites) {
		if (ow.id !== room_id || ow.type !== "Role") continue;
		for (const p of ow.allow) {
			perms.add(p);
		}
	}

	// 2. Apply everyone denies
	for (const ow of channel.permission_overwrites) {
		if (ow.id !== room_id || ow.type !== "Role") continue;
		for (const p of ow.deny) {
			perms.delete(p);
		}
	}

	// 3. Apply role allows
	for (const ow of channel.permission_overwrites) {
		if (ow.type !== "Role") continue;
		if (!memberRoleIds.has(ow.id)) continue;
		for (const p of ow.allow) {
			perms.add(p);
		}
	}

	// 4. Apply role denies
	for (const ow of channel.permission_overwrites) {
		if (ow.type !== "Role") continue;
		if (!memberRoleIds.has(ow.id)) continue;
		for (const p of ow.deny) {
			perms.delete(p);
		}
	}

	// 5. Apply user allows
	for (const ow of channel.permission_overwrites) {
		if (ow.type !== "User") continue;
		if (ow.id !== member.user_id) continue;
		for (const p of ow.allow) {
			perms.add(p);
		}
	}

	// 6. Apply user denies
	for (const ow of channel.permission_overwrites) {
		if (ow.type !== "User") continue;
		if (ow.id !== member.user_id) continue;
		for (const p of ow.deny) {
			perms.delete(p);
		}
	}
}

export function hasPermission(
	ctx: PermissionContext,
	user_id: string,
	permission: Permission,
): boolean {
	const { permissions } = calculatePermissions(ctx, user_id);
	if (permissions.has("Admin")) return true;
	return permissions.has(permission);
}

export function createPermissionChecker(
	ctx: PermissionContext,
	user_id: string,
) {
	const resolved = calculatePermissions(ctx, user_id);

	return {
		has: (permission: Permission): boolean => {
			if (resolved.permissions.has("Admin")) return true;
			return resolved.permissions.has(permission);
		},
		permissions: resolved.permissions,
		rank: resolved.rank,
	};
}

export function canUseCommand(
	ctx: PermissionContext,
	user_id: string,
	commandName: string,
	channel: any,
): boolean {
	const channelType = channel?.ty;

	switch (commandName) {
		// only usable in threads
		case "archive":
		case "unarchive":
			if (channelType !== "ThreadPublic" && channelType !== "ThreadPrivate") {
				return false;
			}
			break;

		// only usable in rooms
		case "nick":
		case "ban":
		case "kick":
		case "name-room":
		case "desc-room":
			if (!ctx.room_id) return false;
			break;

		default:
			break;
	}

	const checker = createPermissionChecker(ctx, user_id);
	switch (commandName) {
		case "thread":
			return checker.has("ThreadCreatePublic") ||
				checker.has("ThreadCreatePrivate");
		case "archive":
		case "unarchive":
			return checker.has("ThreadManage");
		case "ban":
			return checker.has("MemberBan");
		case "kick":
			return checker.has("MemberKick");
		case "timeout":
			return checker.has("MemberTimeout");
		case "lock":
		case "unlock":
			return checker.has("ThreadLock");
		case "name-room":
		case "desc-room":
			return checker.has("RoomManage");
		case "nick":
			return checker.has("MemberNickname");
		case "slowmode":
			return checker.has("ChannelManage");
		default:
			return true;
	}
}

/**
 * Check if a user can bypass channel locks
 */
export function canUseLockedChannel(
	ctx: PermissionContext,
	user_id: string,
): boolean {
	const checker = createPermissionChecker(ctx, user_id);
	return checker.has("ThreadManage") ||
		checker.has("ChannelManage") ||
		checker.has("ThreadLock") ||
		checker.permissions.has("LockedBypass" as Permission);
}
