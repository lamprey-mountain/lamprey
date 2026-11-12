import type { Role } from "sdk";
import { For, Show } from "solid-js";
import { useApi, useRoomMembers, useUsers } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { usePermissions } from "@/hooks/usePermissions";
import { Item, Menu, Separator } from "./Parts.tsx";

// the context menu for managing a member's roles
export function MemberRolesMenu(props: { room_id: string; user_id: string }) {
	const api2 = useApi();
	const roomMembers2 = useRoomMembers();
	const users2 = useUsers();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;

	const member = () =>
		roomMembers2.cache.get(`${props.room_id}:${props.user_id}`);
	const user = () => users2.cache.get(props.user_id);
	const name = () => member()?.override_name ?? user()?.name ?? props.user_id;

	const { has: hasPermission, permissions } = usePermissions(
		self_id,
		() => props.room_id,
		() => undefined,
	);

	const canManage = (role: Role) =>
		hasPermission("RoleManage") && permissions().rank > (role.position ?? 0);

	const roles = () =>
		[...api2.roles.cache.values()]
			.filter(
				(role) => role.room_id === props.room_id && role.id !== role.room_id,
			)
			.sort((a, b) => b.position - a.position);

	const hasRole = (role: Role) => member()?.roles.includes(role.id) ?? false;

	const toggleRole = (role: Role) => (e: MouseEvent) => {
		// keep the menu open so multiple roles can be toggled at once
		e.stopPropagation();
		const m = member();
		if (!m || !canManage(role)) return;
		if (hasRole(role)) {
			api2.roles.removeMember(props.room_id, role.id, props.user_id);
		} else {
			api2.roles.addMember(props.room_id, role.id, props.user_id);
		}
	};

	return (
		<Menu>
			<Item disabled>{name()}</Item>
			<Separator />
			<Show when={roles().length > 0} fallback={<Item disabled>no roles</Item>}>
				<For each={roles()}>
					{(role) => (
						<Item
							onClick={toggleRole(role)}
							disabled={!canManage(role)}
							classList={{ selected: hasRole(role) }}
						>
							{role.name}
						</Item>
					)}
				</For>
			</Show>
		</Menu>
	);
}
