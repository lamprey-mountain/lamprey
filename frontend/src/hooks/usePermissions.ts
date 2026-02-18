import { createMemo } from "solid-js";
import type { Permission } from "sdk";
import { useApi } from "../api";
import {
	calculatePermissions,
	type PermissionContext,
	type ResolvedPermissions,
} from "../permission-calculator";

export function usePermissions(
	user_id: () => string | undefined,
	room_id: () => string | undefined,
	thread_id: () => string | undefined,
) {
	const api = useApi();

	console.log("[perms] hook used");

	const permissions = createMemo<ResolvedPermissions>(() => {
		if (!user_id()) {
			return {
				permissions: new Set(),
				rank: 0,
				timedOut: false,
				quarantined: false,
				lurker: false,
			};
		}

		const user = api.users.fetch(() => user_id()!)();
		if (user?.webhook) {
			const webhookPermissions = new Set<Permission>([
				"MessageCreate",
				"MessageEmbeds",
			]);
			return {
				permissions: webhookPermissions,
				rank: 0,
				timedOut: false,
				quarantined: false,
				lurker: false,
			};
		}

		const permissionContext: PermissionContext = {
			api,
			room_id: room_id(),
			channel_id: thread_id(),
		};

		return calculatePermissions(permissionContext, user_id()!);
	});

	const has = (wants: Permission) => {
		const perms = permissions().permissions;
		if (perms.has("Admin")) return true;
		return perms.has(wants);
	};

	return { permissions, has };
}
