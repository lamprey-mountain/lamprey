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
import { createStore } from "solid-js/store";
import { md } from "../Message.tsx";
import { moderatorPermissions, permissionGroups } from "../permissions.ts";

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
						<button class="big primary" onClick={createRole}>
							create role
						</button>
					</header>
					<Show when={roles()} fallback="loading...">
						<RoleList search={search()} roles={roles()!} edit={edit} />
					</Show>
				</div>
				<Show when={api.roles.cache.has(edit.role.id!)}>
					<RoleEditor room={props.room} edit={edit} />
				</Show>
			</div>
		</>
	);
}

const RoleList = (
	props: { search: string; roles: Pagination<Role>; edit: RoleEditState },
) => {
	const filteredRoles = createMemo(() => {
		return props.roles!.items.sort((a, b) => b.position - a.position)
			.filter((i) => i.name.includes(props.search));
	});

	return (
		<ul class="role-list">
			<For each={filteredRoles()}>
				{(i) => (
					<li
						onClick={() => {
							if (props.edit.role.id === i.id) {
								props.edit.setRole({ id: null } as unknown as Role);
							} else {
								props.edit.setRole(structuredClone(i));
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
			<For
				each={props.room.type === "Default"
					? [
						"room",
						"members",
						"messages",
						"threads",
						"voice",
						"dangerous",
					]
					: [
						"server",
						"room",
						"server members",
						"messages",
						"threads",
						"dangerous",
					]}
			>
				{(group) => {
					return (
						<>
							<h3>{group} permissions</h3>
							<ul>
								<For each={permissionGroups.get(group)}>
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
