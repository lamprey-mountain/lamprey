import type { Api } from "./api";

export type Permission = string;

export interface PermissionContext {
	api: Api;
	room_id?: string;
	channel_id?: string;
}

export interface ResolvedPermissions {
	permissions: Set<Permission>;
	rank: number;
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
		return { permissions: new Set(["Admin"]), rank: Infinity };
	}

	const member = ctx.api.room_members.fetch(
		() => ctx.room_id!,
		() => user_id,
	)();
	const rolesResource = ctx.api.roles.list(() => ctx.room_id!);

	if (!room || !member || !rolesResource || member.membership !== "Join") {
		return { permissions: new Set(), rank: 0 };
	}

	const roles = rolesResource()?.items;
	if (!roles) return { permissions: new Set(), rank: 0 };

	const finalPermissions = new Set<Permission>();
	const everyoneRole = roles.find((r) => r.id === ctx.room_id);
	const memberRoles = roles.filter((role) => member.roles.includes(role.id));
	if (everyoneRole) {
		memberRoles.push(everyoneRole);
	}

	for (const role of memberRoles) {
		for (const p of role.allow) {
			finalPermissions.add(p);
		}

		for (const p of role.deny) {
			finalPermissions.delete(p);
		}
	}

	if (ctx.channel_id) {
		const channel = ctx.api.channels.fetch(() => ctx.channel_id)();
		if (channel) {
			const memberRoleIds = new Set(member.roles);
			if (everyoneRole) memberRoleIds.add(everyoneRole.id);

			const applyOverwrites = (overwrites: any[] | undefined) => {
				if (!overwrites) return;

				const roleOverwrites = overwrites.filter((o: any) =>
					o.type === "Role" && memberRoleIds.has(o.id)
				);

				for (const ow of roleOverwrites) {
					for (const p of ow.allow) finalPermissions.add(p);
				}
				for (const ow of roleOverwrites) {
					for (const p of ow.deny) finalPermissions.delete(p);
				}

				const userOverwrite = overwrites.find((o: any) =>
					o.type === "User" && o.id === user_id
				);
				if (userOverwrite) {
					for (const p of userOverwrite.allow) finalPermissions.add(p);
					for (const p of userOverwrite.deny) finalPermissions.delete(p);
				}
			};

			if (channel.parent_id) {
				const parentChannel = ctx.api.channels.fetch(() => channel.parent_id)();
				applyOverwrites(parentChannel?.permission_overwrites);
			}

			applyOverwrites(channel.permission_overwrites);
		}
	}

	const rank = roles.reduce(
		(max, role) =>
			member.roles.includes(role.id) ? Math.max(role.position, max) : max,
		0,
	);

	return { permissions: finalPermissions, rank };
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
			return checker.has("RoomEdit");
		case "nick":
			return checker.has("MemberNickname");
		case "slowmode":
			return checker.has("ChannelManage");
		default:
			return true;
	}
}
