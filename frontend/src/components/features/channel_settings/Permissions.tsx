import type {
	Channel,
	ChannelType,
	Permission,
	PermissionOverwrite,
	Role,
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
import { useApi, useRoles, useRooms } from "@/api";
import { Resizable } from "@/atoms/Resizable";
import { Savebar } from "@/atoms/Savebar";
import { OverwriteDropdown } from "@/components/shared/OverwriteDropdown";
import { PermissionSelector } from "@/components/shared/PermissionSelector";
import { useMenu } from "@/contexts/mod.tsx";
import { permissions } from "@/lib/permissions";

function filterPermissionsByChannelType(
	permList: typeof permissions,
	channelType?: ChannelType,
): typeof permissions {
	if (!channelType) return permList;

	return permList.filter((perm) => {
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
	const api2 = useApi();
	const rooms2 = useRooms();
	const roles2 = useRoles();
	const { setMenu } = useMenu();

	const roomId = () => props.channel.room_id ?? "";
	const roles = createMemo(() =>
		[...roles2.cache.values()].filter((r) => r.room_id === roomId()),
	);
	const room = rooms2.use(roomId);

	const [overwrites, setOverwrites] = createStore(
		structuredClone(props.channel.permission_overwrites ?? []),
	);
	const [editingId, setEditingId] = createSignal(roomId());
	const [permSearch, setPermSearch] = createSignal("");
	const [saving, setSaving] = createSignal(false);

	const dirtyIds = createMemo(() =>
		overwrites
			.filter(
				(o) =>
					!areOverwritesEqual(
						o,
						props.channel.permission_overwrites?.find(
							(orig) => orig.id === o.id,
						),
					),
			)
			.map((o) => o.id),
	);

	const deletedIds = createMemo(() =>
		(props.channel.permission_overwrites ?? [])
			.filter((orig) => !overwrites.some((o) => o.id === orig.id))
			.map((o) => o.id),
	);

	const isAnyDirty = createMemo(
		() => dirtyIds().length > 0 || deletedIds().length > 0,
	);

	const overwritesWithEveryone = createMemo(() => {
		const rId = roomId();
		const hasEveryone = overwrites.some((o) => isEveryoneRole(o.id, rId));
		return hasEveryone
			? overwrites
			: [...overwrites, createDefaultOverwrite(rId)];
	});

	const editingOverwrite = createMemo(() => {
		const id = editingId();

		const overwrite = overwrites.find((o) => o.id === id);
		if (overwrite) return overwrite;

		if (isEveryoneRole(id, roomId())) {
			return createDefaultOverwrite(id);
		}

		return null;
	});

	const setPerm = (perm: Permission, state: PermState) => {
		const id = editingId();

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

		const existsInStore = overwrites.some((o) => o.id === id);

		const isEveryone = isEveryoneRole(id, roomId());
		const channelPerms = permissions.filter((p) => p.overwrite_group);
		const isAllInherit = channelPerms.every(
			(p) =>
				!updatedOverwrite.allow.includes(p.id) &&
				!updatedOverwrite.deny.includes(p.id),
		);

		if (isEveryone && isAllInherit) {
			if (existsInStore) {
				// remove pending everyone overwrite
				setOverwrites((prev) => prev.filter((o) => o.id !== id));
			} else {
				// everyone permission can't be deleted anyways
			}
		} else {
			if (existsInStore) {
				// update existing overwrite
				setOverwrites(
					(o) => o.id === id,
					produce((o) => {
						o.allow = newAllow;
						o.deny = newDeny;
					}),
				);
			} else {
				// create new overwrite
				setOverwrites(overwrites.length, {
					id,
					type: "Role",
					allow: newAllow,
					deny: newDeny,
				});
			}
		}
	};

	const saveAll = async () => {
		const putPromises = dirtyIds().flatMap((id) => {
			const o = overwrites.find((o) => o.id === id);
			if (!o) return [];
			return [
				api2.client.http.PUT(
					"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
					{
						params: {
							path: {
								channel_id: props.channel.id,
								overwrite_id: o.id,
							},
						},
						body: {
							type: o.type,
							allow: o.allow,
							deny: o.deny,
						},
					},
				),
			];
		});

		const deletePromises = deletedIds().map((id) =>
			api2.client.http.DELETE(
				"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
				{
					params: {
						path: {
							channel_id: props.channel.id,
							overwrite_id: id,
						},
					},
				},
			),
		);

		setSaving(true);
		await Promise.all([...putPromises, ...deletePromises]).finally(() =>
			setSaving(false),
		);

		// ui updates after receiving ChannelUpdate sync event
	};

	// show latest overwrites unless editing
	createEffect(
		on(
			() => props.channel.permission_overwrites ?? [],
			(newOverwrites) => {
				// TODO: automatically update/refresh overwrites that haven't been edited
				// currently, editing a single overwrite prevents all overwrites from being updated on sync
				if (isAnyDirty()) return;
				setOverwrites(structuredClone(newOverwrites));
			},
		),
	);

	const cancelAll = () => {
		setOverwrites(structuredClone(props.channel.permission_overwrites ?? []));
	};

	const remove = (id: string) => {
		setOverwrites((prev) => prev.filter((o) => o.id !== id));
		if (editingId() === id) {
			setEditingId(roomId());
		}
	};

	const overwriteName = (ow: PermissionOverwrite) => {
		if (isEveryoneRole(ow.id, roomId())) {
			return "@everyone";
		}

		const role = roles().find((r: Role) => r.id === ow.id);
		if (role) return role.name;

		const user = api2.users.cache.get(ow.id);
		if (user) return user.name;

		return "unknown";
	};

	const openOverwriteMenu = (
		e: MouseEvent,
		overwriteId: string,
		overwriteType: "Role" | "User" | "Everyone",
	) => {
		e.preventDefault();
		setMenu({
			type: "permission_overwrite",
			channel_id: props.channel.id,
			overwrite_id: overwriteId,
			overwrite_type: overwriteType,
			x: e.clientX,
			y: e.clientY,
			onDelete: () => remove(overwriteId),
		});
	};

	const addOverwrite = (id: string, type: "Role" | "User") => {
		batch(() => {
			setOverwrites(overwrites.length, {
				id,
				type,
				allow: [],
				deny: [],
			});
			setEditingId(id);
		});
	};

	const filteredPermissions = createMemo(() => {
		return filterPermissionsByChannelType(
			permissions,
			props.channel.type,
		).filter((p: (typeof permissions)[number]) => p.overwrite_group);
	});

	const isDirty = (id: string) =>
		dirtyIds().includes(id) || deletedIds().includes(id);

	return (
		<div class="channel-settings-permissions">
			<div class="wrapper">
				<div class="main">
					<h2>Permissions</h2>
					<div class="permission-overwrites">
						<div class="permissions-layout">
							<OverwriteDropdown
								room_id={roomId()}
								excludeIds={overwrites.map((o) => o.id)}
								onSelect={(id, type) => addOverwrite(id, type)}
							/>
							<div>
								<ul>
									<For each={overwritesWithEveryone()}>
										{(o) => {
											const isEveryone = isEveryoneRole(o.id, roomId());
											const overwriteType: "Role" | "User" | "Everyone" =
												isEveryone ? "Everyone" : o.type;
											return (
												<li
													class="overwrite"
													classList={{ editing: editingId() === o.id }}
													onClick={() => setEditingId(o.id)}
													onContextMenu={(e) =>
														openOverwriteMenu(e, o.id, overwriteType)
													}
												>
													{overwriteName(o)}
													<Show when={isDirty(o.id)}>
														<span class="dirty-indicator">*</span>
													</Show>
												</li>
											);
										}}
									</For>
								</ul>
							</div>
						</div>
					</div>
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
										Editing {overwriteName(overwrite)}
										<Show when={isDirty(overwrite.id)}>
											<span class="dirty-indicator">*</span>
										</Show>
									</h3>
									<Show when={!isEveryoneRole(overwrite.id, roomId())}>
										<button
											type="button"
											class="button danger"
											onClick={() => remove(overwrite.id)}
										>
											delete
										</button>
									</Show>
								</div>
								<PermissionSelector
									search={permSearch()}
									onSearch={setPermSearch}
									seed={props.channel.id + overwrite.id}
									permissions={filteredPermissions()}
									permStates={filteredPermissions().reduce(
										(
											acc: Record<Permission, PermState>,
											p: (typeof permissions)[number],
										) => {
											acc[p.id] = getPermState(overwrite, p.id);
											return acc;
										},
										{} as Record<Permission, PermState>,
									)}
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
			<Savebar
				show={isAnyDirty()}
				onCancel={cancelAll}
				onSave={saveAll}
				cancelText="Cancel"
				saveText="Save All"
				saving={saving()}
			/>
		</div>
	);
}
