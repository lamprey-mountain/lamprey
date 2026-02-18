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
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import type { Pagination, Permission, Role, RoomMember, User } from "sdk";
import { Copyable } from "../util.tsx";
import { createStore, produce } from "solid-js/store";
import { permissions } from "../permissions.ts";
import { Resizable } from "../Resizable";
import { md } from "../markdown.tsx";
import { PermissionSelector } from "../components/PermissionSelector";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons.tsx";
import { A } from "@solidjs/router";
import { Avatar } from "../User.tsx";

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

	return a.name !== b.name ||
		a.description !== b.description ||
		a.is_self_applicable !== b.is_self_applicable ||
		a.is_mentionable !== b.is_mentionable ||
		a.hoist !== b.hoist ||
		allowDiff1.size + allowDiff2.size + denyDiff1.size + denyDiff2.size > 0;
}

export function Roles(props: VoidProps<{ room: RoomT }>) {
	const api = useApi();
	const roles = api.roles.list(() => props.room.id);
	const [, modalCtl] = useModals();

	const [localRoles, setLocalRoles] = createStore<Role[]>([]);
	const [isOrderDirty, setIsOrderDirty] = createSignal(false);

	createEffect(() => {
		if (roles()) {
			const sortedRoles = [...roles()!.items].sort((a, b) =>
				b.position - a.position
			);
			setLocalRoles(sortedRoles);
		}
	});

	createEffect(() => {
		if (!roles()?.items) {
			return;
		}
		const originalSorted = [...roles()!.items].sort((a, b) =>
			b.position - a.position
		);
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
		modalCtl.prompt("role name?", (name) => {
			if (!name) return;
			api.client.http.POST("/api/v1/room/{room_id}/role", {
				params: { path: { room_id: props.room.id } },
				body: { name },
			});
		});
	};

	const saveOrder = () => {
		api.client.http.PATCH("/api/v1/room/{room_id}/role", {
			params: { path: { room_id: props.room.id } },
			body: {
				roles: localRoles
					.map((role, index) => ({ role, index }))
					.filter(({ role }) => role.id !== role.room_id)
					.map(({ role, index }) => ({
						role_id: role.id,
						position: localRoles.length - index,
					})),
			},
		});
	};

	const cancelOrder = () => {
		if (roles()) {
			const sortedRoles = [...roles()!.items].sort((a, b) =>
				b.position - a.position
			);
			setLocalRoles(sortedRoles);
		}
	};

	const [search, setSearch] = createSignal("");
	const edit = useRoleEditor(null);

	return (
		<>
			<div class="room-settings-roles">
				<div class="role-main">
					<h2>roles</h2>
					<header class="applications-header">
						<input
							type="search"
							placeholder="search"
							aria-label="search"
							onInput={(e) => setSearch(e.target.value)}
						/>
						<Show when={isOrderDirty()}>
							<div style="display: flex; gap: 8px; align-items: center; margin-left: auto">
								<span>order changed</span>
								<button class="big" onClick={cancelOrder}>cancel</button>
								<button class="big primary" onClick={saveOrder}>save</button>
							</div>
						</Show>
						<button class="big primary" onClick={createRole}>
							create role
						</button>
					</header>
					<Show when={roles()} fallback="loading...">
						<RoleList
							search={search()}
							roles={localRoles}
							setRoles={setLocalRoles}
							edit={edit}
						/>
					</Show>
				</div>
				<Show when={api.roles.cache.has(edit.role.id!)}>
					<Resizable
						storageKey="role-editor-width"
						initialWidth={400}
						minWidth={300}
						maxWidth={800}
						classList={{ "role-edit-resizable": true }}
					>
						<RoleEditor room={props.room} edit={edit} />
					</Resizable>
				</Show>
			</div>
		</>
	);
}

const RoleList = (
	props: {
		search: string;
		roles: Role[];
		setRoles: import("solid-js/store").SetStoreFunction<Role[]>;
		edit: RoleEditState;
	},
) => {
	const filteredRoles = createMemo(() => {
		return props.roles.filter((i) => i.name.includes(props.search));
	});

	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(
		null,
	);

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
		if (role?.id === role?.room_id) {
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
		if (toRole.id === toRole.room_id) return;

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
						draggable={i.id !== i.room_id}
						onDragStart={handleDragStart}
						onDragOver={handleDragOver}
						onDrop={handleDrop}
						onDragEnd={() => {
							setDragging(null);
							setTarget(null);
						}}
						classList={{
							dragging: dragging() === i.id,
						}}
						onClick={() => {
							if (props.edit.role.id === i.id) {
								props.edit.setRole({ id: null } as unknown as Role);
							} else {
								props.edit.setRole(JSON.parse(JSON.stringify(i)));
								props.edit.setName(i.name);
								props.edit.setDesc(i.description || undefined);
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
								<Match when={i.allow.length === 0 && i.deny.length === 0}>
									<div>cosmetic</div>
								</Match>
								<Match when={true}>
									<div>
										<span class="perm-safe">
											{i.allow.filter(
												(perm: Permission) => {
													const p = permissions.find((x) => x.id === perm);
													return !p?.moderator;
												},
											).length}
										</span>
										+
										<span class="perm-mod">
											{i.allow.filter((perm) => {
												const p = permissions.find((x) => x.id === perm);
												return p?.moderator;
											}).length}
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

const RoleEditor = (props: { room: RoomT; edit: RoleEditState }) => {
	const api = useApi();
	const ctx = useCtx();
	const [permSearch, setPermSearch] = createSignal("");
	const [, modalCtl] = useModals();
	const [activeTab, setActiveTab] = createSignal<"role" | "members">("role");
	const [memberSearch, setMemberSearch] = createSignal("");

	const members = api.roles.memberList(
		() => props.room.id,
		() => props.edit.role.id!,
	);

	const filteredMembers = createMemo(() => {
		const search = memberSearch().toLowerCase();
		const allMembers = members()?.items ?? [];
		if (!search) return allMembers;
		return allMembers.filter((m) => {
			const user = api.users.cache.get(m.user_id);
			const name = user?.name ?? m.user_id;
			return name.toLowerCase().includes(search);
		});
	});

	const addMember = () => {
		// TODO: make this modal nicer
		modalCtl.prompt("user id to add", (user_id) => {
			if (!user_id) return;
			api.roles.addMember(props.room.id, props.edit.role.id!, user_id);
		});
	};

	const removeMember = (user_id: string) => {
		api.roles.removeMember(props.room.id, props.edit.role.id!, user_id);
	};

	const deleteRole = (role_id: string) => () => {
		modalCtl.confirm("are you sure?", (confirmed) => {
			if (!confirmed) return;
			api.client.http.DELETE("/api/v1/room/{room_id}/role/{role_id}", {
				params: { path: { room_id: props.room.id, role_id } },
			});
		});
	};

	const saveRole = () => {
		if (
			!isDirty(
				props.edit.role as Role,
				api.roles.cache.get(props.edit.role.id!)!,
			)
		) return;
		const r = props.edit.role as Role;
		api.client.http.PATCH("/api/v1/room/{room_id}/role/{role_id}", {
			params: { path: { room_id: props.room.id, role_id: r.id } },
			body: {
				name: r.name,
				description: r.description,
				allow: r.allow,
				deny: r.deny,
				is_mentionable: r.is_mentionable,
				is_self_applicable: r.is_self_applicable,
				hoist: r.hoist,
			},
		});
	};

	return (
		<div class="role-edit">
			<div class="toolbar">
				<button
					onClick={() => {
						props.edit.setRole({ id: null } as unknown as Role);
					}}
				>
					close
				</button>
				<button
					disabled={!isDirty(
						props.edit.role as Role,
						api.roles.cache.get(props.edit.role.id!)!,
					)}
					onClick={saveRole}
				>
					save
				</button>
				<button class="danger" onClick={deleteRole(props.edit.role.id!)}>
					delete role
				</button>
			</div>
			<div class="tabs">
				<button
					classList={{ active: activeTab() === "role" }}
					onClick={() => setActiveTab("role")}
				>
					role
				</button>
				<button
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
					onInput={(e) => {
						props.edit.setRole((r) => ({
							...r,
							name: (e.target as HTMLInputElement).value,
						}));
					}}
				/>
				<h3>description</h3>
				<textarea
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
					each={[
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
					] as const}
				>
					{(i) => (
						<label class="option">
							<input
								type="checkbox"
								checked={(props.edit.role as Role)[i.key]}
								onInput={(e) => {
									props.edit.setRole((r) => ({
										...r,
										[i.key]: (e.target as HTMLInputElement).checked,
									}));
								}}
								style="display: none;"
							/>
							<Checkbox
								checked={(props.edit.role as Role)[i.key]}
							/>
							<div>
								<div class="name">
									{i.name}
								</div>
								<div
									class="description"
									innerHTML={md.parseInline(
										i.description ?? "",
									) as string}
								/>
							</div>
						</label>
					)}
				</For>

				<div class="perm-search-container">
					<h3>permissions</h3>
				</div>

				{() => {
					const { t } = useCtx();
					const searchQuery = permSearch().toLowerCase();
					const allPermissions = permissions.filter((perm) => {
						const isServer = props.room.type === "Server";
						if (isServer) {
							if (!perm.types?.includes("Server")) return false;
						} else {
							if (!perm.types?.includes("Room")) return false;
						}

						if (!searchQuery) return true;
						const name = t(`permissions.${perm.id}.name`) ?? perm.id;
						const description = t(`permissions.${perm.id}.description`) ?? "";
						return (
							name.toLowerCase().includes(searchQuery) ||
							description.toLowerCase().includes(searchQuery) ||
							perm.id.toLowerCase().includes(searchQuery)
						);
					});

					const permStates = allPermissions.reduce((acc, perm) => {
						const role = props.edit.role;
						if (role.allow?.includes(perm.id)) acc[perm.id] = "allow";
						else if (role.deny?.includes(perm.id)) acc[perm.id] = "deny";
						else acc[perm.id] = "inherit";
						return acc;
					}, {} as Record<Permission, "allow" | "deny" | "inherit">);

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
							permissions={allPermissions}
							permStates={permStates}
							onPermChange={handlePermChange}
							showDescriptions={true}
							roomType={props.room.type}
						/>
					);
				}}
			</Show>
			<Show when={activeTab() === "members"}>
				<div class="members-tab">
					<div class="members-header">
						<input
							type="search"
							placeholder="search members..."
							value={memberSearch()}
							onInput={(e) => setMemberSearch(e.currentTarget.value)}
						/>
						<button onClick={addMember}>add member</button>
					</div>
					<ul class="members-list">
						<For each={filteredMembers()}>
							{(member) => {
								const user = api.users.fetch(() => member.user_id);
								return (
									<li class="member-item">
										<Avatar user={user()} pad={4} />
										<A href={`/user/${member.user_id}`}>
											{user()?.name ?? member.user_id}
										</A>
										<div style="flex:1"></div>
										<button onClick={() => removeMember(member.user_id)}>
											remove
										</button>
									</li>
								);
							}}
						</For>
					</ul>
				</div>
			</Show>
		</div>
	);
};

type RoleEditState = ReturnType<typeof useRoleEditor>;

function useRoleEditor(initial: Role | null) {
	const [role, setRole] = createStore(
		initial ?? { id: null } as unknown as Role,
	);
	const [name, setName] = createSignal(initial?.name ?? "");
	const [desc, setDesc] = createSignal(initial?.description ?? undefined);

	return { role, setRole, name, setName, desc, setDesc };
}
