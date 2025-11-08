import type { ChannelType } from "sdk";
import type { Api } from "../api";

/**
 * Helper function to check user's permission in a channel/room
 */
export function hasPermission(api: Api, room_id: string | undefined, channel_id: string, permission: string): boolean {
	if (!room_id) {
		// For non-room channels (DMs, GDMS), we'll allow some basic permissions
		const defaultPermissions = [
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
		return defaultPermissions.includes(permission);
	}

	const self_id = api.users.cache.get("@self")?.id;
	if (!self_id) return false;

	const room = api.rooms.fetch(() => room_id)();
	if (room?.owner_id === self_id) {
		return true; // Owner has all permissions
	}

	const member = api.room_members.fetch(() => room_id, () => self_id)();
	const rolesResource = api.roles.list(() => room_id);

	if (!room || !member || !rolesResource || member.membership !== "Join") {
		return false;
	}

	const roles = rolesResource()?.items;
	if (!roles) return false;

	const finalPermissions = new Set<string>();
	const everyoneRole = roles.find((r) => r.id === room_id);
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

	// Check thread-specific permissions if in a thread
	if (channel_id) {
		const thread = api.channels.fetch(() => channel_id)();
		if (thread) {
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
					o.type === "User" && o.id === self_id
				);
				if (userOverwrite) {
					for (const p of userOverwrite.allow) finalPermissions.add(p);
					for (const p of userOverwrite.deny) finalPermissions.delete(p);
				}
			};

			if (thread.parent_id) {
				const parentThread = api.channels.fetch(() => thread.parent_id)();
				applyOverwrites(parentThread?.permission_overwrites);
			}

			applyOverwrites(thread.permission_overwrites);
		}
	}

	if (finalPermissions.has("Admin")) return true;
	return finalPermissions.has(permission);
}

/**
 * Check if a command should be available based on channel type and permissions
 */
export function canUseCommand(api: Api, room_id: string | undefined, channel: any, commandName: string): boolean {
	const channelType = channel?.ty;

	switch (commandName) {
		case "archive":
		case "unarchive":
			if (channelType !== "ThreadPublic" && channelType !== "ThreadPrivate") {
				return false;
			}
			break;

		case "nick":
		case "ban":
		case "kick":
		case "name-room":
		case "desc-room":
			if (!room_id) return false;
			break;

		default:
			break;
	}

	switch (commandName) {
		case "thread":
			return hasPermission(api, room_id, channel?.id, "ThreadCreatePublic");
		case "archive":
		case "unarchive":
			return hasPermission(api, room_id, channel?.id, "ThreadManage");
		case "ban":
			return hasPermission(api, room_id, channel?.id, "MemberBan");
		case "kick":
			return hasPermission(api, room_id, channel?.id, "MemberKick");
		case "timeout":
			return hasPermission(api, room_id, channel?.id, "MemberTimeout");
		case "lock":
		case "unlock":
			return hasPermission(api, room_id, channel?.id, "ThreadLock");
		case "name-room":
		case "desc-room":
			return hasPermission(api, room_id, channel?.id, "RoomEdit");
		case "nick":
			return hasPermission(api, room_id, channel?.id, "MemberNickname");
		default:
			return true;
	}
}
