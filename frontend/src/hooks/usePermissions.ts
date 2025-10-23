import { createMemo } from "solid-js";
import type { Permission, PermissionOverwrite } from "sdk";
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

		const member = api.room_members.fetch(() => rid, user_id as () => string)();
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
			const thread = api.channels.fetch(() => tid)();

			const memberRoleIds = new Set(member.roles);
			if (everyoneRole) memberRoleIds.add(everyoneRole.id);

			const applyOverwrites = (
				overwrites: PermissionOverwrite[] | undefined,
			) => {
				if (!overwrites) return;
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
			};

			if (thread?.parent_id) {
				console.log("[perms] has parent thread", thread.parent_id);
				const parentThread = api.channels.fetch(
					() => thread.parent_id!,
				)();
				applyOverwrites(parentThread?.permission_overwrites);
			}

			applyOverwrites(thread?.permission_overwrites);
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
