import { createMemo } from "solid-js";
import type { Permission } from "sdk";
import { useApi } from "../api";

export function usePermissions(
	user_id: () => string | undefined,
	room_id: () => string | undefined,
	thread_id: () => string | undefined,
) {
	const api = useApi();

	console.log("[perms] hook used");

	const permissions = createMemo(() => {
		if (!user_id()) return { permissions: new Set(), rank: 0 };

		const finalPermissions = new Set<Permission>();
		const rid = room_id();

		if (!rid) {
			console.log("[perms] no room id");
			const defaultPermissions: Permission[] = [
				"BotsAdd",
				"EmojiAdd",
				"EmojiUseExternal",
				"InviteCreate",
				"MessageCreate",
				"MessageEmbeds",
				"MessageMassMention",
				"MessageAttachments",
				"MessageMove",
				"MessagePin",
				"ReactionAdd",
				"ProfileAvatar",
				"ProfileOverride",
				"TagApply",
				"ThreadArchive",
				"ThreadCreateChat",
				"ThreadCreateDocument",
				"ThreadCreateEvent",
				"ThreadCreateForumLinear",
				"ThreadCreateForumTree",
				"ThreadCreateTable",
				"ThreadCreateVoice",
				"ThreadCreatePublic",
				"ThreadCreatePrivate",
				"ThreadEdit",
				"ThreadForward",
				"ViewAuditLog",
				"VoiceConnect",
				"VoiceSpeak",
				"VoiceVideo",
			];
			for (const p of defaultPermissions) {
				finalPermissions.add(p);
			}
			return { permissions: finalPermissions, rank: 0 };
		}

		console.log("[perms] in room", rid);

		const room = api.rooms.fetch(() => rid)();

		if (room?.owner_id === user_id()) {
			console.log("[perms] user is owner");
			return { permissions: new Set(["Admin"]), rank: Infinity };
		}

		const member = api.room_members.fetch(() => rid, user_id)();
		const rolesResource = api.roles.list(() => rid);

		if (!room || !member || !rolesResource() || member.membership !== "Join") {
			return { permissions: finalPermissions, rank: 0 };
		}

		const roles = rolesResource()!.items;

		const everyoneRole = roles.find((r) => r.id === rid);
		const memberRoles = roles.filter((role) => member.roles.includes(role.id));
		if (everyoneRole) {
			memberRoles.push(everyoneRole);
		}

		for (const role of memberRoles) {
			for (const p of role.permissions) {
				finalPermissions.add(p);
			}
		}

		const tid = thread_id();
		if (tid) {
			console.log("[perms] in thread", tid);
			const thread = api.threads.fetch(() => tid)();
			if (thread && thread.permission_overwrites) {
				const overwrites = thread.permission_overwrites;

				const memberRoleIds = new Set(member.roles);
				if (everyoneRole) memberRoleIds.add(everyoneRole.id);

				const roleOverwrites = overwrites.filter((o) =>
					o.type === "Role" && memberRoleIds.has(o.id)
				);

				for (const ow of roleOverwrites) {
					for (const p of ow.allow) finalPermissions.add(p);
				}
				for (const ow of roleOverwrites) {
					for (const p of ow.deny) finalPermissions.delete(p);
				}

				const userOverwrite = overwrites.find((o) =>
					o.type === "User" && o.id === user_id()
				);
				if (userOverwrite) {
					for (const p of userOverwrite.allow) finalPermissions.add(p);
					for (const p of userOverwrite.deny) finalPermissions.delete(p);
				}
			}
		}

		console.log("[perms] resolved perms", finalPermissions);

		const rank = roles.reduce(
			(max, role) =>
				member.roles.includes(role.id) ? Math.max(role.position, max) : max,
			0,
		);
		console.log("[perms] rank (highest role position)", rank);

		return { permissions: finalPermissions, rank };
	});

	const has = (wants: Permission) => {
		const perms = permissions().permissions;
		if (perms.has("Admin")) return true;
		return perms.has(wants);
	};

	return { permissions, has };
}
