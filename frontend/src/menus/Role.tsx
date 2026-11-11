import { Show } from "solid-js";
import { useApi, useRoles, useRoomMembers } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { useModals } from "@/contexts/modal";
import { useNavigate } from "@/contexts/router";
import { usePermissions } from "@/hooks/usePermissions";
import { Item, Menu, Separator } from "./Parts.tsx";

// the context menu for roles
export function RoleMenu(props: { role_id: string; room_id?: string }) {
	const api2 = useApi();
	const roles2 = useRoles();
	const roomMembers2 = useRoomMembers();
	const { setMenu } = useMenu();
	const nav = useNavigate();
	const [, modalCtl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;

	const role = () => roles2.cache.get(props.role_id);
	const room_id = () => role()?.room_id ?? props.room_id;
	const room = () => {
		const rid = room_id();
		return rid ? api2.rooms.cache.get(rid) : undefined;
	};

	const { has: hasPermission, permissions } = usePermissions(
		self_id,
		room_id,
		() => undefined,
	);

	const canManage = () => {
		const r = role();
		if (!r || !hasPermission("RoleManage")) return false;
		if (room()?.owner_id === self_id()) return true;
		return permissions().rank > (r.position ?? 0);
	};

	const selfMember = () => {
		const rid = room_id();
		const uid = self_id();
		return rid && uid ? roomMembers2.cache.get(`${rid}:${uid}`) : null;
	};

	const hasRole = () => selfMember()?.roles.includes(props.role_id) ?? false;

	const copyMention = () =>
		navigator.clipboard.writeText(`<&${props.role_id}>`);
	const copyId = () => navigator.clipboard.writeText(props.role_id);
	const logToConsole = () => console.log(JSON.parse(JSON.stringify(role())));

	const edit = () => {
		const rid = room_id();
		if (!rid) return;
		setMenu(null);
		nav(`/room/${rid}/settings/roles`);
	};

	const manageMembers = () => {
		const rid = room_id();
		const r = role();
		if (!rid || !r) return;
		modalCtl.open({ type: "role_members", room_id: rid, role_id: r.id });
	};

	const deleteRole = () => {
		const rid = room_id();
		const r = role();
		if (!rid || !r) return;
		modalCtl.confirm(
			`are you sure you want to delete "${r.name}"?`,
			async (confirmed) => {
				if (!confirmed) return;
				await roles2.deleteRole(rid, r.id);
			},
		);
	};

	const toggleSelf = () => {
		const rid = room_id();
		const uid = self_id();
		if (!rid || !uid) return;
		if (hasRole()) roles2.removeMember(rid, props.role_id, uid);
		else roles2.addMember(rid, props.role_id, uid);
	};

	return (
		<Menu>
			<Item onClick={copyMention}>copy role mention</Item>
			<Item onClick={copyId}>copy role id</Item>
			<Show when={role()?.is_self_applicable && hasPermission("RoleApply")}>
				<Item onClick={toggleSelf}>
					{hasRole() ? "remove role" : "apply role"}
				</Item>
			</Show>
			<Show when={hasPermission("RoleManage")}>
				<Separator />
				<Item onClick={edit} disabled={!canManage()}>
					edit role
				</Item>
				<Item onClick={manageMembers} disabled={!canManage()}>
					manage members
				</Item>
				<Show when={canManage() && role() && role()!.id !== role()!.room_id}>
					<Item onClick={deleteRole} color="danger">
						delete role
					</Item>
				</Show>
			</Show>
			<Separator />
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
