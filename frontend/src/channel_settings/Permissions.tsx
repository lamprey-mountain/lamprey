import type {
	Channel,
	ChannelType,
	Permission,
	PermissionOverwrite,
} from "sdk";
import {
	batch,
	createEffect,
	createMemo,
	createSignal,
	For,
	on,
	Show,
	type VoidProps,
} from "solid-js";
import { createStore, produce } from "solid-js/store";
import { useApi } from "../api.tsx";
import { PermissionSelector } from "../components/PermissionSelector";
import { permissions } from "../permissions.ts";
import { Resizable } from "../Resizable";
import { Copyable } from "../util.tsx";
import { useCtx } from "../context.ts";

function filterPermissionsByChannelType(
	permissions: typeof permissions,
	channelType?: ChannelType,
): typeof permissions {
	if (!channelType) return permissions;

	return permissions.filter((perm) => {
		if (!perm.types) return true;
		return perm.types.includes(channelType);
	});
}

type PermState = "allow" | "deny" | "inherit";

function getPermState(
	overwrite: PermissionOverwrite,
	perm: Permission,
): PermState {
	if (overwrite.allow.includes(perm)) return "allow";
	if (overwrite.deny.includes(perm)) return "deny";
	return "inherit";
}

function isAllInherit(
	overwrite: PermissionOverwrite,
	allPermissions: Permission[],
): boolean {
	return allPermissions.every((perm) =>
		getPermState(overwrite, perm) === "inherit"
	);
}

function areOverwritesEqual(
	o1?: PermissionOverwrite,
	o2?: PermissionOverwrite,
): boolean {
	if (!o1 && !o2) return true;
	if (!o1 || !o2) return false;
	if (o1.id !== o2.id || o1.type !== o2.type) return false;

	const allow1 = new Set(o1.allow);
	const allow2 = new Set(o2.allow);
	if (allow1.size !== allow2.size || ![...allow1].every((p) => allow2.has(p))) {
		return false;
	}

	const deny1 = new Set(o1.deny);
	const deny2 = new Set(o2.deny);
	if (deny1.size !== deny2.size || ![...deny1].every((p) => deny2.has(p))) {
		return false;
	}

	return true;
}

const isEveryoneRole = (id: string, roomId: string) => id === roomId;

const createDefaultOverwrite = (id: string): PermissionOverwrite => ({
	id,
	type: "Role",
	allow: [],
	deny: [],
});

export function Permissions(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();
	const roles = api.roles.list(() => props.channel.room_id);
	const room = api.rooms.fetch(() => props.channel.room_id);

	const [overwrites, setOverwrites] = createStore(
		structuredClone(props.channel.permission_overwrites),
	);
	const [dirtyOverwrites, setDirtyOverwrites] = createStore<
		Record<string, boolean>
	>({});
	const [deletedOverwrites, setDeletedOverwrites] = createStore<
		Record<string, boolean>
	>({});
	const [editingId, setEditingId] = createSignal<string>(
		props.channel.room_id!,
	);

	const resetChangeTracking = () => {
		setDirtyOverwrites({});
		setDeletedOverwrites({});
	};

	createEffect(on(() => props.channel.id, () => {
		setOverwrites(structuredClone(props.channel.permission_overwrites));
		resetChangeTracking();
	}));

	createEffect(
		on(() => props.channel.permission_overwrites, (serverOverwrites) => {
			if (!serverOverwrites) return;
			batch(() => {
				const dirtyIds = Object.keys(dirtyOverwrites).filter((id) =>
					dirtyOverwrites[id]
				);
				for (const id of dirtyIds) {
					const localOverwrite = overwrites.find((o) => o.id === id);
					const serverOverwrite = serverOverwrites.find((o) => o.id === id);
					if (areOverwritesEqual(localOverwrite, serverOverwrite)) {
						setDirtyOverwrites(id, false);
					}
				}

				const deletedIds = Object.keys(deletedOverwrites).filter((id) =>
					deletedOverwrites[id]
				);
				for (const id of deletedIds) {
					if (!serverOverwrites.some((o) => o.id === id)) {
						setDeletedOverwrites(id, false);
					}
				}
			});
		}, { defer: true }),
	);

	const updateChangeState = (id: string) => {
		const localOverwrite = overwrites.find((o) => o.id === id);
		const originalOverwrite = props.channel.permission_overwrites?.find((o) =>
			o.id === id
		);

		if (areOverwritesEqual(localOverwrite, originalOverwrite)) {
			setDirtyOverwrites(id, false);
			setDeletedOverwrites(id, false);
			return;
		}

		if (localOverwrite && !originalOverwrite) { // Added
			setDirtyOverwrites(id, true);
			setDeletedOverwrites(id, false);
		} else if (!localOverwrite && originalOverwrite) { // Deleted
			setDirtyOverwrites(id, false);
			setDeletedOverwrites(id, true);
		} else { // Modified
			setDirtyOverwrites(id, true);
			setDeletedOverwrites(id, false);
		}
	};

	const isAnyDirty = createMemo(() => {
		return Object.values(dirtyOverwrites).some(Boolean) ||
			Object.values(deletedOverwrites).some(Boolean);
	});

	const overwritesWithEveryone = createMemo(() => {
		const roomId = props.channel.room_id!;
		const hasEveryone = overwrites.some((o) => isEveryoneRole(o.id, roomId));
		return hasEveryone
			? overwrites
			: [...overwrites, createDefaultOverwrite(roomId)];
	});

	const editingOverwrite = createMemo(() => {
		const id = editingId();
		if (id === null) return null;

		const overwrite = overwrites.find((o) => o.id === id);
		if (overwrite) return overwrite;

		if (isEveryoneRole(id, props.channel.room_id)) {
			return createDefaultOverwrite(id);
		}

		return null;
	});

	const setPerm = (perm: Permission, state: PermState) => {
		const id = editingId();
		if (!id) return;

		const currentOverwrite = editingOverwrite();
		if (!currentOverwrite) return;

		const newAllow = currentOverwrite.allow.filter((p) => p !== perm);
		const newDeny = currentOverwrite.deny.filter((p) => p !== perm);
		if (state === "allow") newAllow.push(perm);
		else if (state === "deny") newDeny.push(perm);

		const updatedOverwrite: PermissionOverwrite = {
			...currentOverwrite,
			allow: newAllow,
			deny: newDeny,
		};

		const isEveryone = isEveryoneRole(id, props.channel.room_id);
		const channelPerms = permissions.filter((p) => p.overwrite_group);
		const shouldBeRemoved = isEveryone &&
			isAllInherit(updatedOverwrite, channelPerms.map((p) => p.id));
		const existsInStore = overwrites.some((o) => o.id === id);

		if (shouldBeRemoved) {
			if (existsInStore) {
				setOverwrites((prev) => prev.filter((o) => o.id !== id));
			}
		} else {
			if (existsInStore) {
				setOverwrites(
					(o) => o.id === id,
					produce((o) => {
						o.allow = newAllow;
						o.deny = newDeny;
					}),
				);
			} else {
				setOverwrites(overwrites.length, {
					id,
					type: "Role",
					allow: newAllow,
					deny: newDeny,
				});
			}
		}
		queueMicrotask(() => updateChangeState(id));
	};

	const saveAll = async () => {
		const dirtyIds = Object.keys(dirtyOverwrites).filter((id) =>
			dirtyOverwrites[id]
		);
		const deletedIds = Object.keys(deletedOverwrites).filter((id) =>
			deletedOverwrites[id]
		);

		const putPromises = dirtyIds.map((id) => {
			const overwrite = overwrites.find((o) => o.id === id);
			if (overwrite) {
				return api.client.http.PUT(
					"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
					{
						params: {
							path: {
								channel_id: props.channel.id,
								overwrite_id: overwrite.id,
							},
						},
						body: {
							type: overwrite.type,
							allow: overwrite.allow,
							deny: overwrite.deny,
						},
					},
				);
			}
			return null;
		}).filter((p) => p !== null) as Promise<unknown>[];

		const deletePromises = deletedIds.map((id) =>
			api.client.http.DELETE(
				"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
				{
					params: {
						path: {
							channel_id: props.channel.id,
							overwrite_id: id,
						},
					},
				},
			)
		);

		await Promise.all([...putPromises, ...deletePromises]);

		resetChangeTracking();
	};

	const cancelAll = () => {
		setOverwrites(structuredClone(props.channel.permission_overwrites));
		resetChangeTracking();
	};

	const remove = (id: string) => {
		if (!id) return;

		setOverwrites((prev) => prev.filter((o) => o.id !== id));
		queueMicrotask(() => updateChangeState(id));

		if (editingId() === id) {
			setEditingId(props.channel.room_id);
		}
	};

	const addRole = (roleId: string) => {
		batch(() => {
			setOverwrites(overwrites.length, createDefaultOverwrite(roleId));
			setEditingId(roleId);
			queueMicrotask(() => updateChangeState(roleId));
		});
	};

	const { t } = useCtx();

	const roleName = (id: string) => {
		if (isEveryoneRole(id, props.channel.room_id)) {
			return "@everyone";
		}
		return roles()?.items.find((r) => r.id === id)?.name;
	};

	const availableRoles = () =>
		roles()?.items.filter((r) =>
			!isEveryoneRole(r.id, props.channel.room_id) &&
			!overwrites.some((o) => o.id === r.id)
		);

	const filteredPermissions = createMemo(() => {
		return filterPermissionsByChannelType(
			permissions,
			props.channel.type,
		).filter((p) => p.overwrite_group);
	});

	return (
		<div class="channel-settings-permissions">
			<div class="main">
				<h2>Permissions</h2>
				<div class="permission-overwrites">
					<div class="permissions-layout">
						<div>
							<ul>
								<For each={overwritesWithEveryone()}>
									{(o) => (
										<li
											class={editingId() === o.id ? "editing" : ""}
											onClick={() => setEditingId(o.id)}
										>
											{roleName(o.id) ?? <Copyable>{o.id}</Copyable>}
											<Show
												when={dirtyOverwrites[o.id] || deletedOverwrites[o.id]}
											>
												<span class="dirty-indicator">*</span>
											</Show>
										</li>
									)}
								</For>
							</ul>
						</div>
						<div>
							<Show when={availableRoles()?.length}>
								<select
									onChange={(e) => {
										if (e.currentTarget.value) addRole(e.currentTarget.value);
										e.currentTarget.value = "";
									}}
								>
									<option value="">Add role...</option>
									<For each={availableRoles()}>
										{(r) => <option value={r.id}>{r.name}</option>}
									</For>
								</select>
							</Show>
							{/* <button onClick={()}>add user</button> */}
						</div>
					</div>
				</div>
				<Show when={isAnyDirty()}>
					<div class="savebar">
						<div class="inner">
							<div class="warning">you have unsaved changes</div>
							<button class="reset" onClick={cancelAll}>Cancel</button>
							<button class="save" onClick={saveAll}>Save All</button>
						</div>
					</div>
				</Show>
			</div>
			<Show when={editingOverwrite()} keyed>
				{(overwrite) => (
					<Resizable
						storageKey="channel-permissions-panel-width"
						initialWidth={500}
						minWidth={300}
						maxWidth={800}
					>
						<div class="edit">
							<div class="permissions-header">
								<h3 class="editing-title">
									Editing{" "}
									{overwrite.type === "Role" ? roleName(overwrite.id) : "user"}
									{" "}
									<Show
										when={dirtyOverwrites[overwrite.id] ||
											deletedOverwrites[overwrite.id]}
									>
										<span class="dirty-indicator">*</span>
									</Show>
								</h3>
								<Show
									when={!isEveryoneRole(overwrite.id, props.channel.room_id!)}
								>
									<button onClick={() => remove(overwrite.id)}>delete</button>
								</Show>
							</div>
							<PermissionSelector
								seed={props.channel.id + overwrite.id}
								permissions={filteredPermissions()}
								permStates={filteredPermissions().reduce((acc, p) => {
									acc[p.id] = getPermState(overwrite, p.id);
									return acc;
								}, {} as Record<Permission, PermState>)}
								onPermChange={setPerm}
								showDescriptions={true}
								roomType={room()?.type || "Default"}
								context="overwrite"
							/>
						</div>
					</Resizable>
				)}
			</Show>
		</div>
	);
}
