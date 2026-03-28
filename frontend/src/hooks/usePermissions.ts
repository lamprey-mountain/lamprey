import { createEffect, createMemo, untrack } from "solid-js";
import type { Permission } from "sdk";
import { useApi2, useUsers2 } from "@/api";
import {
	calculatePermissions,
	type PermissionContext,
	type ResolvedPermissions,
} from "../permission-calculator";
import { logger } from "../logger";

const permHookLog = logger.for("permissions");

export function usePermissions(
	user_id: () => string | undefined,
	room_id: () => string | undefined,
	thread_id: () => string | undefined,
) {
	const api2 = useApi2();
	const users2 = useUsers2();

	const permissions = createMemo<ResolvedPermissions>(() => {
		const uid = user_id();
		const rid = room_id();
		const tid = thread_id();

		permHookLog.debug("usePermissions memo running", {
			user_id: uid,
			room_id: rid,
			thread_id: tid,
			has_user: !!uid,
		});

		if (!uid) {
			permHookLog.debug("no user_id, returning empty permissions");
			return {
				permissions: new Set(),
				rank: 0,
				timedOut: false,
				quarantined: false,
				lurker: false,
			};
		}

		const user = users2.cache.get(uid);

		if (user?.webhook) {
			const webhookPermissions = new Set<Permission>([
				"MessageCreate",
				"MessageEmbeds",
			]);
			permHookLog.debug("webhook user, returning webhook permissions");
			return {
				permissions: webhookPermissions,
				rank: 0,
				timedOut: false,
				quarantined: false,
				lurker: false,
			};
		}

		const permissionContext: PermissionContext = {
			api: api2,
			room_id: rid,
			channel_id: tid,
		};

		const result = calculatePermissions(permissionContext, uid);

		permHookLog.debug("calculatePermissions result", {
			has_message_create: result.permissions.has("MessageCreate"),
			permission_count: result.permissions.size,
			rank: result.rank,
		});

		return result;
	});

	const has = (wants: Permission) => {
		const perms = permissions().permissions;
		return perms.has("Admin") || perms.has(wants);
	};

	return { permissions, has };
}
