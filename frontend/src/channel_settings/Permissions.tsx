import type { Channel, Permission, PermissionOverwrite } from "sdk";
import {
	batch,
	createMemo,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { createStore, produce } from "solid-js/store";
import { Copyable } from "../util.tsx";
import { permissions } from "../permissions.ts";
import { PermissionSelector } from "../components/PermissionSelector";
import { Resizable } from "../Resizable";

type PermState = "allow" | "deny" | "inherit";

function setDifference<T>(a: Set<T>, b: Set<T>) {
	return new Set([...a].filter((x) => !b.has(x)));
}

function isOverwriteDirty(
	a: PermissionOverwrite,
	b: PermissionOverwrite,
): boolean {
	const allowA = new Set(a.allow);
	const allowB = new Set(b.allow);
	const denyA = new Set(a.deny);
	const denyB = new Set(b.deny);

	const allowDiff1 = setDifference(allowA, allowB);
	const allowDiff2 = setDifference(allowB, allowA);
	const denyDiff1 = setDifference(denyA, denyB);
	const denyDiff2 = setDifference(denyB, denyA);

	return allowDiff1.size + allowDiff2.size + denyDiff1.size + denyDiff2.size >
		0;
}

function getPermState(
	overwrite: PermissionOverwrite,
	perm: Permission,
): PermState {
	if (overwrite.allow.includes(perm)) return "allow";
	if (overwrite.deny.includes(perm)) return "deny";
	return "inherit";
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
	const users = api.room_members.list(() => props.channel.room_id);
	const room = api.rooms.fetch(() => props.channel.room_id);

	const originalOverwrites = structuredClone(
		props.channel.permission_overwrites,
	);
	const [overwrites, setOverwrites] = createStore(originalOverwrites);

	const [dirtyOverwrites, setDirtyOverwrites] = createStore<
		Record<string, boolean>
	>({});

	const [deletedOverwrites, setDeletedOverwrites] = createStore<
		Record<string, boolean>
	>({});

	const markOverwriteDirty = (id: string) => {
		setDirtyOverwrites(id, true);
	};

	const markOverwriteDeleted = (id: string) => {
		setDeletedOverwrites(id, true);
	};

	const isAnyDirty = createMemo(() => {
		return Object.values(dirtyOverwrites).some(Boolean) || Object.values(deletedOverwrites).some(Boolean);
	});

	const overwritesWithEveryone = createMemo(() => {
		const roomId = props.channel.room_id;
		const hasEveryone = overwrites.some((o) => isEveryoneRole(o.id, roomId));
		return hasEveryone
			? overwrites
			: [...overwrites, createDefaultOverwrite(roomId)];
	});

	const [editingId, setEditingId] = createSignal<string>(
		props.channel.room_id,
	);

	const editingOverwrite = () => {
		const id = editingId();
		if (id === null) return null;

		const overwrite = overwrites.find((o) => o.id === id);
		if (overwrite) return overwrite;

		if (isEveryoneRole(id, props.channel.room_id)) {
			return createDefaultOverwrite(id);
		}

		return null;
	};

	const setPerm = (perm: Permission, state: PermState) => {
		const id = editingId();
		if (!id) return;

		const existsInStore = overwrites.some((o) => o.id === id);

		if (existsInStore) {
			setOverwrites(
				(o) => o.id === id,
				produce((o) => {
					o.allow = o.allow.filter((p) => p !== perm);
					o.deny = o.deny.filter((p) => p !== perm);
					if (state === "allow") o.allow.push(perm);
					else if (state === "deny") o.deny.push(perm);
				}),
			);
			markOverwriteDirty(id);
		} else if (isEveryoneRole(id, props.channel.room_id)) {
			const currentOverwrite = editingOverwrite();
			if (currentOverwrite) {
				const newAllow = currentOverwrite.allow.filter((p) => p !== perm);
				const newDeny = currentOverwrite.deny.filter((p) => p !== perm);
				if (state === "allow") newAllow.push(perm);
				else if (state === "deny") newDeny.push(perm);

				const exists = overwrites.some((o) => o.id === id);
				if (exists) {
					setOverwrites(
						(o) => o.id === id,
						produce((o) => {
							o.allow = newAllow;
							o.deny = newDeny;
						}),
					);
					markOverwriteDirty(id);
				} else {
					setOverwrites(overwrites.length, {
						id,
						type: "Role",
						allow: newAllow,
						deny: newDeny,
					});
					markOverwriteDirty(id);
				}
			}
		}
	};

	const saveAll = async () => {
		// Save modified overwrites
		for (const [id, isDirty] of Object.entries(dirtyOverwrites)) {
			if (!isDirty) continue;

			const overwrite = overwrites.find((o) => o.id === id);
			if (overwrite) {
				await api.client.http.PUT(
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
		}

		// Delete removed overwrites
		for (const [id, isDeleted] of Object.entries(deletedOverwrites)) {
			if (!isDeleted) continue;

			await api.client.http.DELETE(
				"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
				{
					params: {
						path: {
							channel_id: props.channel.id,
							overwrite_id: id,
						},
					},
				},
			);
		}

		const newDirtyState: Record<string, boolean> = {};
		for (const key in dirtyOverwrites) {
			newDirtyState[key] = false;
		}
		setDirtyOverwrites(newDirtyState);

		const newDeletedState: Record<string, boolean> = {};
		for (const key in deletedOverwrites) {
			newDeletedState[key] = false;
		}
		setDeletedOverwrites(newDeletedState);
	};

	const cancelAll = () => {
		const newOverwrites = structuredClone(originalOverwrites);
		const newDirtyState: Record<string, boolean> = {};
		for (const key in dirtyOverwrites) {
			newDirtyState[key] = false;
		}
		const newDeletedState: Record<string, boolean> = {};
		for (const key in deletedOverwrites) {
			newDeletedState[key] = false;
		}

		setOverwrites(newOverwrites);
		setDirtyOverwrites(newDirtyState);
		setDeletedOverwrites(newDeletedState);
	};

	const remove = (id: string) => {
		if (!id) return;

		if (isEveryoneRole(id, props.channel.room_id)) {
			// For the everyone role, clear permissions instead of removing
			setOverwrites(
				(o) => o.id === id,
				produce((o) => {
					o.allow = [];
					o.deny = [];
				}),
			);
			// Mark as dirty instead of deleted for the everyone role
			markOverwriteDirty(id);
		} else {
			// Remove the overwrite by filtering it out
			setOverwrites(overwrites.filter((o) => o.id !== id));
			// Mark as deleted to trigger a DELETE request
			markOverwriteDeleted(id);
		}

		if (editingId() === id) {
			setEditingId(null);
		}
	};

	const addRole = (roleId: string) => {
		batch(() => {
			setOverwrites(overwrites.length, createDefaultOverwrite(roleId));
			setEditingId(roleId);
			markOverwriteDirty(roleId);
		});
	};

	const roleName = (id: string) =>
		roles()?.items.find((r) => r.id === id)?.name;

	const availableRoles = () =>
		roles()?.items.filter((r) =>
			!isEveryoneRole(r.id, props.channel.room_id) &&
			!overwrites.some((o) => o.id === r.id)
		);

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
											<Show when={dirtyOverwrites[o.id] || deletedOverwrites[o.id]}>
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
									<Show when={dirtyOverwrites[overwrite.id] || deletedOverwrites[overwrite.id]}>
										<span class="dirty-indicator">*</span>
									</Show>
								</h3>
								<Show when={overwrite.id !== room()?.id}>
									<button onClick={() => remove(overwrite.id)}>delete</button>
								</Show>
							</div>
							<PermissionSelector
								permissions={permissions}
								permStates={permissions.reduce((acc, p) => {
									acc[p.id] = getPermState(overwrite, p.id);
									return acc;
								}, {} as Record<Permission, PermState>)}
								onPermChange={setPerm}
								showDescriptions={true}
								roomType={room()?.type || "Default"}
							/>
						</div>
					</Resizable>
				)}
			</Show>
		</div>
	);
}
