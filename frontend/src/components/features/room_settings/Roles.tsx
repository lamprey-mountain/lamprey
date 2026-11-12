import { A } from "@solidjs/router";
import type { Permission, Role, RoomMember } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
	type VoidProps,
} from "solid-js";
import { createStore, produce } from "solid-js/store";
import { useApi, useRoles, useRoomMembers, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Markdown } from "@/atoms/Markdown.tsx";
import { Resizable } from "@/atoms/Resizable";
import { Savebar } from "@/atoms/Savebar";
import { Search } from "@/atoms/Search";
import { PermissionSelector } from "@/components/shared/PermissionSelector";
import { Avatar } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { useModals } from "@/contexts/modal";
import { useSearchParams } from "@/contexts/router";
import { usePermissions } from "@/hooks/usePermissions";
import { permissions } from "@/lib/permissions";
import type { RoomT } from "@/types";
import { Copyable } from "@/utils/general";

function setDifference<T>(a: Set<T>, b: Set<T>) {
	return new Set([...a].filter((x) => !b.has(x)));
}

function isDirty(a: Role, b: Role): boolean {
	const allowA = new Set(a.allow);
	const allowB = new Set(b.allow);
	const denyA = new Set(a.deny);
	const denyB = new Set(b.deny);

	const allowDiff1 = setDifference(allowA, allowB);
	const allowDiff2 = setDifference(allowB, allowA);
	const denyDiff1 = setDifference(denyA, denyB);
	const denyDiff2 = setDifference(denyB, denyA);

	return (
		a.name !== b.name ||
		a.description !== b.description ||
		a.is_self_applicable !== b.is_self_applicable ||
		a.is_mentionable !== b.is_mentionable ||
		a.hoist !== b.hoist ||
		allowDiff1.size + allowDiff2.size + denyDiff1.size + denyDiff2.size > 0
	);
}

export function Roles(props: VoidProps<{ room: RoomT }>) {
	const api2 = useApi();
	const roles2 = useRoles();
	const [, modalCtl] = useModals();
	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(
		user_id,
		() => props.room.id,
		() => undefined,
	);

	const canEditRole = (role: Role) =>
		perms.has("RoleManage") &&
		(props.room.owner_id === user_id() ||
			perms.permissions().rank > (role.position ?? 0));

	const [localRoles, setLocalRoles] = createStore<Role[]>([]);
	const [isOrderDirty, setIsOrderDirty] = createSignal(false);
	const [savingOrder, setSavingOrder] = createSignal(false);

	// Get roles for this room from cache
	const roomRoles = createMemo(() => {
		const roles: Role[] = [];
		for (const role of roles2.cache.values()) {
			if (role.room_id === props.room.id) {
				roles.push(role);
			}
		}
		return roles.sort((a, b) => b.position - a.position);
	});

	createEffect(() => {
		const sortedRoles = roomRoles();
		if (sortedRoles.length > 0) {
			setLocalRoles(sortedRoles);
		}
	});

	createEffect(() => {
		const originalSorted = roomRoles();
		if (originalSorted.length === 0) {
			return;
		}
		if (originalSorted.length !== localRoles.length) {
			setIsOrderDirty(true);
			return;
		}
		for (let i = 0; i < localRoles.length; i++) {
			if (localRoles[i].id !== originalSorted[i].id) {
				setIsOrderDirty(true);
				return;
			}
		}
		setIsOrderDirty(false);
	});

	const createRole = () => {
		if (!perms.has("RoleManage")) return;
		modalCtl.prompt("role name?", (name) => {
			if (!name) return;
			api2.client.http.POST("/api/v1/room/{room_id}/role", {
				params: { path: { room_id: props.room.id } },
				body: { name },
			});
		});
	};

	const saveOrder = async () => {
		setSavingOrder(true);
		try {
			const reordered = localRoles
				.map((role, index) => ({
					role,
					position: localRoles.length - index,
				}))
				.filter(({ role }) => role.id !== role.room_id);
			const { error } = await api2.client.http.PATCH(
				"/api/v1/room/{room_id}/role",
				{
					params: { path: { room_id: props.room.id } },
					body: {
						roles: reordered.map(({ role, position }) => ({
							role_id: role.id,
							position,
						})),
					},
				},
			);
			if (error) {
				console.error("failed to save role order", error);
				return;
			}
			roles2.upsertBulk(
				reordered.map(({ role, position }) => ({ ...role, position })),
			);
		} finally {
			setSavingOrder(false);
		}
	};

	const cancelOrder = () => {
		const sortedRoles = [...roomRoles()].sort(
			(a, b) => b.position - a.position,
		);
		setLocalRoles(sortedRoles);
	};

	const [search, setSearch] = createSignal("");
	const edit = useRoleEditor(null);

	const [searchParams] = useSearchParams<{ role: string }>();
	const [appliedRoleParam, setAppliedRoleParam] = createSignal<string | null>(
		null,
	);

	createEffect(() => {
		const role_id = searchParams.role;
		if (!role_id || appliedRoleParam() === role_id) return;
		const role = roles2.cache.get(role_id);
		if (!role || role.room_id !== props.room.id) return;
		setAppliedRoleParam(role_id);
		edit.setRole(JSON.parse(JSON.stringify(role)));
		edit.setName(role.name);
		edit.setDesc(role.description || undefined);
	});

	return (
		<div class="room-settings-roles">
			<div class="role-main">
				<h2>roles</h2>
				<header class="roles-header">
					<Search placeholder="Search roles..." onInput={setSearch} />
					<button
						type="button"
						class="button big primary"
						disabled={!perms.has("RoleManage")}
						onClick={createRole}
					>
						create role
					</button>
				</header>
				<Show when={roomRoles().length > 0} fallback="loading...">
					<RoleList
						search={search()}
						roles={localRoles}
						setRoles={setLocalRoles}
						edit={edit}
						canEditRole={canEditRole}
					/>
				</Show>
				<Savebar
					onCancel={cancelOrder}
					onSave={saveOrder}
					show={isOrderDirty()}
					saving={savingOrder()}
					disabled={!perms.has("RoleManage")}
				/>
			</div>
			<Show when={roles2.cache.has(edit.role.id!)}>
				<Resizable
					storageKey="role-editor-width"
					initialWidth={400}
					minWidth={300}
					maxWidth={800}
					classList={{ "role-edit-resizable": true }}
				>
					<RoleEditor
						room={props.room}
						edit={edit}
						canEditRole={canEditRole}
						canChangePermission={(p) => perms.has(p)}
					/>
				</Resizable>
			</Show>
		</div>
	);
}

const RoleList = (props: {
	search: string;
	roles: Role[];
	setRoles: import("solid-js/store").SetStoreFunction<Role[]>;
	edit: RoleEditState;
	canEditRole: (role: Role) => boolean;
}) => {
	const { setMenu } = useMenu();
	const [, updateParams] = useSearchParams<{ role: string }>();

	const filteredRoles = createMemo(() => {
		return props.roles.filter((i) => i.name.includes(props.search));
	});

	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<{
		id: string;
		after: boolean;
	} | null>(null);

	const getRoleId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.roleId;

	const handleDragStart = (e: DragEvent) => {
		const id = getRoleId(e);
		if (id) {
			setDragging(id);
			e.dataTransfer!.effectAllowed = "move";
		}
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		const id = getRoleId(e);
		if (!id || id === dragging()) {
			return;
		}
		const role = props.roles.find((r) => r.id === id);
		if (!role || role.id === role.room_id || !props.canEditRole(role)) {
			setTarget(null);
			return;
		}
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;

		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) {
			return;
		}

		const fromIndex = props.roles.findIndex((r) => r.id === fromId);
		let toIndex = props.roles.findIndex((r) => r.id === toId);

		if (fromIndex === -1 || toIndex === -1) {
			return;
		}

		const toRole = props.roles[toIndex];
		if (toRole.id === toRole.room_id || !props.canEditRole(toRole)) return;

		if (after) toIndex++;
		if (fromIndex < toIndex) toIndex--;

		const originalIds = props.roles.map((r) => r.id);
		const reorderedCheck = [...props.roles];
		const [movedCheck] = reorderedCheck.splice(fromIndex, 1);
		reorderedCheck.splice(toIndex, 0, movedCheck);

		if (
			JSON.stringify(originalIds) ===
			JSON.stringify(reorderedCheck.map((r) => r.id))
		) {
			return;
		}

		props.setRoles(
			produce((roles) => {
				const [moved] = roles.splice(fromIndex, 1);
				roles.splice(toIndex, 0, moved);
			}),
		);
	};

	const previewedRoles = createMemo(() => {
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;
		const roles = filteredRoles();

		if (!fromId || !toId || fromId === toId) {
			return roles;
		}

		const fromIndex = roles.findIndex((r) => r.id === fromId);
		let toIndex = roles.findIndex((r) => r.id === toId);

		if (fromIndex === -1 || toIndex === -1) {
			return roles;
		}

		const toRole = roles[toIndex];
		if (toRole.id === toRole.room_id) return roles;

		if (after) toIndex++;
		if (fromIndex < toIndex) toIndex--;

		const reordered = [...roles];
		const [moved] = reordered.splice(fromIndex, 1);
		reordered.splice(toIndex, 0, moved);

		return reordered;
	});

	return (
		<ul class="role-list">
			<For each={previewedRoles()}>
				{(i) => (
					<li
						data-role-id={i.id}
						draggable={i.id !== i.room_id && props.canEditRole(i)}
						onDragStart={handleDragStart}
						onDragOver={handleDragOver}
						onDrop={handleDrop}
						onDragEnd={() => {
							setDragging(null);
							setTarget(null);
						}}
						onContextMenu={(e) => {
							e.preventDefault();
							e.stopPropagation();
							setMenu({
								x: e.clientX,
								y: e.clientY,
								type: "role",
								role_id: i.id,
								room_id: i.room_id,
							});
						}}
						classList={{
							dragging: dragging() === i.id,
						}}
						onClick={() => {
							if (props.edit.role.id === i.id) {
								props.edit.setRole({ id: null } as unknown as Role);
								updateParams({ role: null });
							} else {
								props.edit.setRole(JSON.parse(JSON.stringify(i)));
								props.edit.setName(i.name);
								props.edit.setDesc(i.description || undefined);
								updateParams({ role: i.id });
							}
						}}
					>
						<div class="info">
							<h3 class="name">{i.name}</h3>
							<Show when={i.description}>
								<div class="divider"></div>
								<div class="description">{i.description}</div>
							</Show>
						</div>
						<div class="info">
							<div class="member-count">{i.member_count} members</div>
							<div class="divider"></div>
							<Switch>
								<Match when={i.allow.includes("Admin")}>
									<div class="perm-admin">admin!</div>
								</Match>
								<Match
									when={i.allow.length === 0 && (i.deny?.length ?? 0) === 0}
								>
									<div>cosmetic</div>
								</Match>
								<Match when={true}>
									<div>
										<span class="perm-safe">
											{
												i.allow.filter((perm: Permission) => {
													const p = permissions.find((x) => x.id === perm);
													return !p?.moderator;
												}).length
											}
										</span>
										+
										<span class="perm-mod">
											{
												i.allow.filter((perm) => {
													const p = permissions.find((x) => x.id === perm);
													return p?.moderator;
												}).length
											}
										</span>{" "}
										permissions
									</div>
								</Match>
							</Switch>
							<Show when={i.is_self_applicable}>
								<div class="divider"></div>
								<div class="self-applicable">self applicable</div>
							</Show>
							<Show when={i.is_mentionable}>
								<div class="divider"></div>
								<div class="mentionable">@mentionable</div>
							</Show>
							<Show when={i.hoist}>
								<div class="divider"></div>
								<div class="hoist">hoist</div>
							</Show>
						</div>
					</li>
				)}
			</For>
		</ul>
	);
};

const RoleEditor = (props: {
	room: RoomT;
	edit: RoleEditState;
	canEditRole: (role: Role) => boolean;
	canChangePermission: (perm: Permission) => boolean;
}) => {
	const api2 = useApi();
	const roles2 = useRoles();
	const users2 = useUsers();
	const roomMembers2 = useRoomMembers();
	const [, modalCtl] = useModals();
	const [activeTab, setActiveTab] = createSignal<"role" | "members">("role");
	const canEdit = createMemo(() => props.canEditRole(props.edit.role as Role));
	const [memberSearch, setMemberSearch] = createSignal("");
	const [savingRole, setSavingRole] = createSignal(false);

	// Get members with this role from cache
	const members = createMemo(() => {
		const roleId = props.edit.role.id!;
		const memberList: RoomMember[] = [];
		for (const member of roomMembers2.cache.values()) {
			if (member.room_id === props.room.id && member.roles.includes(roleId)) {
				memberList.push(member);
			}
		}
		return { items: memberList };
	});

	const filteredMembers = createMemo(() => {
		const search = memberSearch().toLowerCase();
		const allMembers = members()?.items ?? [];
		if (!search) return allMembers;
		return allMembers.filter((m) => {
			const user = users2.cache.get(m.user_id);
			const name = user?.name ?? m.user_id;
			return name.toLowerCase().includes(search);
		});
	});

	const addMember = () => {
		if (!canEdit()) return;
		modalCtl.open({
			type: "role_members",
			room_id: props.room.id,
			role_id: props.edit.role.id!,
		});
	};

	const removeMember = (user_id: string) => {
		if (!canEdit()) return;
		roles2.removeMember(props.room.id, props.edit.role.id!, user_id);
	};

	const deleteRole = (role_id: string) => () => {
		if (!canEdit()) return;
		modalCtl.confirm("are you sure?", async (confirmed) => {
			if (!confirmed) return;
			await roles2.deleteRole(props.room.id, role_id);
		});
	};

	const saveRole = async () => {
		const r = props.edit.role as Role;
		if (!canEdit()) return;
		if (!isDirty(r, roles2.cache.get(props.edit.role.id!)!)) return;
		setSavingRole(true);
		try {
			await roles2.update(props.room.id, r.id, {
				name: r.name,
				description: r.description,
				allow: r.allow,
				deny: r.deny,
				is_mentionable: r.is_mentionable,
				is_self_applicable: r.is_self_applicable,
				hoist: r.hoist,
			});
		} finally {
			setSavingRole(false);
		}
	};

	return (
		<div class="role-edit">
			<div class="toolbar">
				<button
					type="button"
					class="button danger"
					disabled={!canEdit()}
					onClick={deleteRole(props.edit.role.id!)}
				>
					delete role
				</button>
			</div>
			<div class="tabs">
				<button
					type="button"
					class="button"
					classList={{ active: activeTab() === "role" }}
					onClick={() => setActiveTab("role")}
				>
					role
				</button>
				<button
					type="button"
					class="button"
					classList={{ active: activeTab() === "members" }}
					onClick={() => setActiveTab("members")}
				>
					members
				</button>
			</div>
			<Show when={activeTab() === "role"}>
				<div>
					id <Copyable>{props.edit.role.id!}</Copyable>
				</div>
				<h3>name</h3>
				<input
					type="text"
					value={props.edit.name()}
					disabled={!canEdit()}
					onInput={(e) => {
						props.edit.setRole((r) => ({
							...r,
							name: (e.target as HTMLInputElement).value,
						}));
					}}
				/>
				<h3>description</h3>
				<textarea
					disabled={!canEdit()}
					onInput={(e) => {
						props.edit.setRole((r) => ({
							...r,
							description: (e.target as HTMLTextAreaElement).value || null,
						}));
					}}
				>
					{props.edit.desc()}
				</textarea>
				<br />
				<br />
				<For
					each={
						[
							/* TODO: move these to i18n */
							{
								key: "is_mentionable",
								name: "Mentionable",
								description: "Anyone can mention this role",
							},
							{
								key: "is_self_applicable",
								name: "Self applicable",
								description: "Anyone can apply this role to themselves",
							},
							{
								key: "hoist",
								name: "Hoisted",
								description: "Display this role separately from other members",
							},
						] as const
					}
				>
					{(i) => (
						<CheckboxOption
							id={`role-${props.edit.role.id}-${i.key}`}
							checked={(props.edit.role as Role)[i.key]}
							disabled={!canEdit()}
							onChange={(checked) => {
								props.edit.setRole((r) => ({
									...r,
									[i.key]: checked,
								}));
							}}
							seed={`role-${props.edit.role.id}-${i.key}`}
						>
							<Checkbox
								checked={(props.edit.role as Role)[i.key]}
								seed={`role-${props.edit.role.id}-${i.key}`}
							/>
							<div>
								<div class="name">{i.name}</div>
								<Markdown class="description" content={i.description ?? ""} />
							</div>
						</CheckboxOption>
					)}
				</For>

				<div class="perm-search-container">
					<h3>permissions</h3>
				</div>

				<RolePermissionSelector
					room={props.room}
					edit={props.edit}
					canEdit={canEdit()}
					canChangePermission={props.canChangePermission}
				/>
			</Show>
			<Show when={activeTab() === "members"}>
				<div class="members-tab">
					<div class="members-header">
						<Search
							placeholder="search members..."
							value={memberSearch}
							onInput={setMemberSearch}
						/>
						<button
							type="button"
							class="button primary"
							disabled={!canEdit()}
							onClick={addMember}
						>
							add member
						</button>
					</div>
					<ul class="members-list">
						<For each={filteredMembers()}>
							{(member) => {
								const user = users2.use(() => member.user_id);
								return (
									<li class="member-item">
										<Avatar user={user()} pad={4} />
										<A href={`/user/${member.user_id}`}>
											{user()?.name ?? member.user_id}
										</A>
										<div style="flex:1"></div>
										<button
											type="button"
											class="button"
											disabled={!canEdit()}
											onClick={() => removeMember(member.user_id)}
										>
											remove
										</button>
									</li>
								);
							}}
						</For>
					</ul>
				</div>
			</Show>
			<Savebar
				show={isDirty(
					props.edit.role as Role,
					roles2.cache.get(props.edit.role.id!)!,
				)}
				saving={savingRole()}
				disabled={!canEdit()}
				onCancel={() => {
					props.edit.setRole({ id: null } as unknown as Role);
				}}
				onSave={saveRole}
			/>
		</div>
	);
};

type RoleEditState = ReturnType<typeof useRoleEditor>;

function useRoleEditor(initial: Role | null) {
	const [role, setRole] = createStore(
		initial ?? ({ id: null } as unknown as Role),
	);
	const [name, setName] = createSignal(initial?.name ?? "");
	const [desc, setDesc] = createSignal(initial?.description ?? undefined);

	return { role, setRole, name, setName, desc, setDesc };
}

const RolePermissionSelector = (props: {
	room: RoomT;
	edit: RoleEditState;
	canEdit: boolean;
	canChangePermission: (perm: Permission) => boolean;
}) => {
	const { t } = useCtx();
	const [permSearch, setPermSearch] = createSignal("");

	const allPermissions = createMemo(() => {
		const searchQuery = permSearch().toLowerCase();
		return permissions.filter((perm) => {
			const isServer = props.room.type === "Server";
			if (isServer) {
				if (!perm.types?.includes("Server")) return false;
			} else {
				if (!perm.types?.includes("Room")) return false;
			}

			if (!searchQuery) return true;
			const name = t(`permissions.${perm.id}.name` as any) ?? perm.id;
			const description = t(`permissions.${perm.id}.description` as any) ?? "";
			return (
				(name as string).toLowerCase().includes(searchQuery) ||
				(description as string).toLowerCase().includes(searchQuery) ||
				perm.id.toLowerCase().includes(searchQuery)
			);
		});
	});

	const permStates = createMemo(() => {
		return allPermissions().reduce(
			(acc, perm) => {
				const role = props.edit.role;
				if (role.allow?.includes(perm.id)) acc[perm.id] = "allow";
				else if (role.deny?.includes(perm.id)) acc[perm.id] = "deny";
				else acc[perm.id] = "inherit";
				return acc;
			},
			{} as Record<Permission, "allow" | "deny" | "inherit">,
		);
	});

	const handlePermChange = (
		permId: Permission,
		newState: "allow" | "deny" | "inherit",
	) => {
		props.edit.setRole((prev) => {
			const newRole = { ...prev };
			newRole.allow = (newRole.allow || []).filter((p) => p !== permId);
			newRole.deny = (newRole.deny || []).filter((p) => p !== permId);

			if (newState === "allow") {
				newRole.allow.push(permId);
			} else if (newState === "deny") {
				newRole.deny.push(permId);
			}

			return newRole;
		});
	};

	return (
		<PermissionSelector
			search={permSearch()}
			onSearch={setPermSearch}
			seed={props.edit.role.id!}
			permissions={allPermissions()}
			permStates={permStates()}
			onPermChange={handlePermChange}
			showDescriptions={true}
			roomType={props.room.type}
			canEdit={props.canEdit}
			canChange={props.canChangePermission}
		/>
	);
};
