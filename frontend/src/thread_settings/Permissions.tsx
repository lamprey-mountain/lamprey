import type { Permission, PermissionOverwrite, Thread } from "sdk";
import { batch, createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { createStore, produce } from "solid-js/store";
import { Copyable } from "../util.tsx";
import { permissions } from "../permissions.ts";

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

export function Permissions(props: VoidProps<{ thread: Thread }>) {
	const api = useApi();
	const roles = api.roles.list(() => props.thread.room_id);
	const users = api.room_members.list(() => props.thread.room_id);

	const [overwrites, setOverwrites] = createStore(
		structuredClone(props.thread.permission_overwrites),
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
			"/api/v1/thread/{thread_id}/permission/{overwrite_id}",
			{
				params: {
					path: {
						thread_id: props.thread.id,
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
			"/api/v1/thread/{thread_id}/permission/{overwrite_id}",
			{
				params: {
					path: {
						thread_id: props.thread.id,
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
		<>
			<h2>Permissions</h2>
			<div style="display:flex; gap: 1rem">
				<div style="display: flex; flex-direction: column; gap: 1rem;">
					<div>
						<ul>
							<For each={overwrites}>
								{(o) => (
									<li
										style={editingId() === o.id ? "font-weight: bold" : ""}
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
				<Show when={editingOverwrite()} keyed>
					{(overwrite) => (
						<div style="width: 500px">
							<div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 1rem">
								<h3 style="margin: 0">
									Editing{" "}
									{overwrite.type === "Role" ? roleName(overwrite.id) : "user"}
									{" "}
								</h3>
								<button onClick={() => setEditingId(null)}>close</button>
								<button onClick={save}>save</button>
								<button onClick={remove}>delete</button>
							</div>
							<ul>
								<For each={permissions}>
									{(p) => {
										const state = () => getPermState(overwrite, p.id);
										return (
											<li style="display: flex; justify-content: space-between; align-items: center; background-color: #111111; padding: 4px; margin-bottom: 4px">
												<span>{p.id}</span>
												<div style="display: flex; gap: 0.25rem;">
													<button
														style={state() === "allow"
															? "background-color: green"
															: ""}
														onClick={() => setPerm(p.id, "allow")}
													>
														✓
													</button>
													<button
														style={state() === "inherit"
															? "background-color: grey"
															: ""}
														onClick={() => setPerm(p.id, "inherit")}
													>
														/
													</button>
													<button
														style={state() === "deny"
															? "background-color: red"
															: ""}
														onClick={() => setPerm(p.id, "deny")}
													>
														X
													</button>
												</div>
											</li>
										);
									}}
								</For>
							</ul>
						</div>
					)}
				</Show>
			</div>
		</>
	);
}
