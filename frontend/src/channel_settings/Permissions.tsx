import type { Channel, Permission, PermissionOverwrite } from "sdk";
import { batch, createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { createStore, produce } from "solid-js/store";
import { Copyable } from "../util.tsx";
import { permissions } from "../permissions.ts";
import { PermissionSelector } from "../components/PermissionSelector";
import { Resizable } from "../Resizable";

type PermState = "allow" | "deny" | "inherit";

function getPermState(
	overwrite: PermissionOverwrite,
	perm: Permission,
): PermState {
	if (overwrite.allow.includes(perm)) {
		return "allow";
	}
	if (overwrite.deny.includes(perm)) {
		return "deny";
	}
	return "inherit";
}

export function Permissions(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();
	const roles = api.roles.list(() => props.channel.room_id);
	const users = api.room_members.list(() => props.channel.room_id);

	const [overwrites, setOverwrites] = createStore(
		structuredClone(props.channel.permission_overwrites),
	);
	const [editingId, setEditingId] = createSignal<string | null>(null);

	const editingOverwrite = () => {
		const id = editingId();
		if (id === null) return null;
		return overwrites.find((o) => o.id === id);
	};

	const setPerm = (perm: Permission, state: PermState) => {
		const id = editingId();
		if (!id) return;
		setOverwrites(
			(o) => o.id === id,
			produce((o) => {
				o.allow = o.allow.filter((p) => p !== perm);
				o.deny = o.deny.filter((p) => p !== perm);
				if (state === "allow") {
					o.allow.push(perm);
				} else if (state === "deny") {
					o.deny.push(perm);
				}
			}),
		);
	};

	const save = () => {
		const overwrite = editingOverwrite();
		if (!overwrite) return;
		api.client.http.PUT(
			"/api/v1/channel/{channel_id}/permission/{overwrite_id}",
			{
				params: {
					path: {
						channel_id: props.channel.id,
						overwrite_id: overwrite.id,
					},
				},
				body: {
					type: "Role",
					allow: overwrite.allow,
					deny: overwrite.deny,
				},
			},
		);
	};

	const remove = () => {
		const id = editingId();
		if (!id) return;
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
		);
		setEditingId(null);
	};

	const addRole = (roleId: string) => {
		batch(() => {
			setOverwrites(overwrites.length, {
				id: roleId,
				type: "Role",
				allow: [],
				deny: [],
			});
			setEditingId(roleId);
		});
	};

	const roleName = (id: string) =>
		roles()?.items.find((r) => r.id === id)?.name;

	const availableRoles = () =>
		roles()?.items.filter((r) => !overwrites.some((o) => o.id === r.id));

	return (
		<div class="channel-settings-permissions">
			<div class="main">
				<h2>Permissions</h2>
				<div class="permission-overwrites">
					<div class="permissions-layout">
						<div>
							<ul>
								<For each={overwrites}>
									{(o) => (
										<li
											class={editingId() === o.id ? "editing" : ""}
											onClick={() => setEditingId(o.id)}
										>
											{roleName(o.id) ?? <Copyable>{o.id}</Copyable>}
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
								</h3>
								<button onClick={() => setEditingId(null)}>close</button>
								<button onClick={save}>save</button>
								<button onClick={remove}>delete</button>
							</div>
							<PermissionSelector
								permissions={permissions}
								permStates={permissions.reduce((acc, p) => {
									acc[p.id] = getPermState(overwrite, p.id);
									return acc;
								}, {} as Record<Permission, PermState>)}
								onPermChange={setPerm}
								showDescriptions={true}
							/>
						</div>
					</Resizable>
				)}
			</Show>
		</div>
	);
}
