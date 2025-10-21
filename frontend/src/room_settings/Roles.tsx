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
import type { Pagination, Permission, Role } from "sdk";
import { Copyable } from "../util.tsx";
import { createStore, produce } from "solid-js/store";
import { moderatorPermissions, permissionGroups } from "../permissions.ts";
import { Resizable } from "../Resizable.tsx";
import { md } from "../markdown.tsx";

function isDirty(a: Role, b: Role): boolean {
	return a.name !== b.name ||
		a.description !== b.description ||
		a.is_self_applicable !== b.is_self_applicable ||
		a.is_mentionable !== b.is_mentionable ||
		new Set(a.permissions).symmetricDifference(new Set(b.permissions)).size !==
			0;
}

export function Roles(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const api = useApi();
	const roles = api.roles.list(() => props.room.id);

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
		ctx.dispatch({
			do: "modal.prompt",
			text: "role name?",
			cont(name) {
				if (!name) return;
				api.client.http.POST("/api/v1/room/{room_id}/role", {
					params: { path: { room_id: props.room.id } },
					body: { name },
				});
			},
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
						class="role-edit-resizable"
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
								<Match when={i.permissions.includes("Admin")}>
									<div class="perm-admin">admin!</div>
								</Match>
								<Match when={i.permissions.length === 0}>
									<div>cosmetic</div>
								</Match>
								<Match when={true}>
									<div>
										<span class="perm-safe">
											{i.permissions.filter((i) =>
												!moderatorPermissions.includes(i)
											).length}
										</span>
										+
										<span class="perm-mod">
											{i.permissions.filter((i) =>
												moderatorPermissions.includes(i)
											).length}
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

	const deleteRole = (role_id: string) => () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure?",
			cont(confirmed) {
				if (!confirmed) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/role/{role_id}", {
					params: { path: { room_id: props.room.id, role_id } },
				});
			},
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
				permissions: r.permissions,
				is_mentionable: r.is_mentionable,
				is_self_applicable: r.is_self_applicable,
			},
		});
	};

	// Filter permissions based on search query
	const filteredPermissionGroups = createMemo(() => {
		const searchQuery = permSearch().toLowerCase();
		const groups = props.room.type === "Default"
			? ["room", "members", "messages", "threads", "voice", "dangerous"]
			: [
				"server",
				"room",
				"server members",
				"messages",
				"threads",
				"dangerous",
			];

		const filtered: Record<
			string,
			{ id: string; name: string; description: string }[]
		> = {};

		for (const group of groups) {
			const allPerms = permissionGroups.get(group) || [];
			if (searchQuery) {
				filtered[group] = allPerms.filter((perm) =>
					perm.name.toLowerCase().includes(searchQuery) ||
					perm.description.toLowerCase().includes(searchQuery) ||
					perm.id.toLowerCase().includes(searchQuery)
				);
			} else {
				filtered[group] = allPerms;
			}
		}

		return { groups, filtered };
	});

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
				<button onClick={deleteRole(props.edit.role.id!)}>delete role</button>
			</div>
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
				] as const}
			>
				{(i) => (
					<div>
						<label>
							<input
								type="checkbox"
								checked={(props.edit.role as Role)[i.key]}
								onInput={(e) => {
									props.edit.setRole((r) => ({
										...r,
										[i.key]: (e.target as HTMLInputElement).checked,
									}));
								}}
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
					</div>
				)}
			</For>

			<div class="perm-search-container">
				<h3>permissions</h3>
				<input
					type="search"
					placeholder="Search permissions..."
					value={permSearch()}
					onInput={(e) => setPermSearch(e.target.value)}
					class="perm-search-input"
				/>
			</div>

			<For
				each={filteredPermissionGroups().groups}
			>
				{(group) => {
					const permsForGroup = () =>
						filteredPermissionGroups().filtered[group];
					return (
						<Show when={permsForGroup().length > 0}>
							<>
								<h3>{group} permissions</h3>
								<ul>
									<For each={permsForGroup()}>
										{(perm) => {
											const perms = () => props.edit.role.permissions ?? [];
											return (
												<li>
													<label>
														<input
															type="checkbox"
															checked={perms().includes(perm.id)}
															onInput={(e) => {
																const { checked } = e
																	.target as HTMLInputElement;
																props.edit.setRole((r) => {
																	const old = (r as Role).permissions;
																	return {
																		...r,
																		permissions: checked
																			? [...old, perm.id]
																			: old.filter((i) => i !== perm.id),
																	};
																});
															}}
														/>
														<div>
															<div class="name">
																{perm.name}
															</div>
															<div
																class="description"
																innerHTML={md.parseInline(
																	perm.description ?? "",
																) as string}
															/>
														</div>
													</label>
												</li>
											);
										}}
									</For>
								</ul>
							</>
						</Show>
					);
				}}
			</For>
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
