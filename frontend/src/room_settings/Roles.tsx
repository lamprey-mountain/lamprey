import {
	createEffect,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import type { Permission, Role } from "sdk";
import { Copyable } from "../util.tsx";
import { createStore } from "solid-js/store";

function isDirty(a: Role, b: Role): boolean {
	console.log(
		new Set(a.permissions).symmetricDifference(new Set(b.permissions)),
	);
	return a.name !== b.name ||
		a.description !== b.description ||
		a.is_default !== b.is_default ||
		a.is_self_applicable !== b.is_self_applicable ||
		a.is_mentionable !== b.is_mentionable ||
		new Set(a.permissions).symmetricDifference(new Set(b.permissions)).size !==
			0;
}

// unused permissions commented out for now
export const permissions: Array<{ id: Permission }> = [
	{ id: "Admin" },
	{ id: "BotsAdd" },
	{ id: "BotsManage" },
	{ id: "EmojiAdd" },
	{ id: "EmojiManage" },
	{ id: "EmojiUseExternal" },
	{ id: "InviteCreate" },
	{ id: "InviteManage" },
	{ id: "MemberBan" },
	{ id: "MemberBanManage" },
	{ id: "MemberBridge" },
	{ id: "MemberKick" },
	{ id: "MemberManage" },
	{ id: "MessageAttachments" },
	{ id: "MessageCreate" },
	{ id: "MessageDelete" },
	{ id: "MessageEmbeds" },
	// { id: "MessageMassMention" },
	// { id: "MessageMove" },
	{ id: "MessagePin" },
	{ id: "ProfileAvatar" },
	{ id: "ProfileOverride" },
	{ id: "ReactionAdd" },
	{ id: "ReactionClear" },
	{ id: "RoleApply" },
	{ id: "RoleManage" },
	{ id: "RoomManage" },
	// { id: "TagApply" },
	// { id: "TagManage" },
	{ id: "ThreadArchive" },
	{ id: "ThreadCreateChat" },
	// { id: "ThreadCreateDocument" },
	// { id: "ThreadCreateEvent" },
	// { id: "ThreadCreateForumLinear" },
	// { id: "ThreadCreateForumTree" },
	// { id: "ThreadCreatePrivate" },
	// { id: "ThreadCreatePublic" },
	// { id: "ThreadCreateTable" },
	// { id: "ThreadCreateVoice" },
	{ id: "ThreadDelete" },
	{ id: "ThreadEdit" },
	// { id: "ThreadForward" },
	{ id: "ThreadLock" },
	{ id: "ThreadPin" },
	// { id: "ThreadPublish" },
	{ id: "ViewAuditLog" },
	{ id: "VoiceConnect" },
	{ id: "VoiceDeafen" },
	{ id: "VoiceDisconnect" },
	{ id: "VoiceMove" },
	{ id: "VoiceMute" },
	{ id: "VoicePriority" },
	{ id: "VoiceSpeak" },
	{ id: "VoiceVideo" },
];

export function Roles(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const api = useApi();
	const roles = api.roles.list(() => props.room.id);
	const [editing, setEditing] = createStore(
		{ id: null } as Role | { id: null },
	);
	const [editName, setEditName] = createSignal<string>();
	const [editDesc, setEditDesc] = createSignal<string>();

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
		if (!isDirty(editing as Role, api.roles.cache.get(editing.id!)!)) return;
		const r = editing as Role;
		api.client.http.PATCH("/api/v1/room/{room_id}/role/{role_id}", {
			params: { path: { room_id: props.room.id, role_id: r.id } },
			body: {
				name: r.name,
				description: r.description,
				permissions: r.permissions,
				is_default: r.is_default,
				is_mentionable: r.is_mentionable,
				is_self_applicable: r.is_self_applicable,
			},
		});
	};

	createEffect(() => console.log("editName", editName()));

	return (
		<>
			<div class="room-settings-roles">
				<div class="role-main">
					<h2>roles</h2>
					<button onClick={api.roles.list(() => props.room.id)}>
						fetch more
					</button>
					<br />
					<button onClick={createRole}>create role</button>
					<br />
					<Show when={roles()}>
						<ul class="role-list">
							<For each={roles()!.items}>
								{(i) => (
									<li
										onClick={() => {
											setEditing(structuredClone(i));
											setEditName(i.name);
											setEditDesc(i.description || undefined);
										}}
									>
										<div class="info">
											<h3 class="name">{i.name}</h3>
											<div class="spacer"></div>
										</div>
										<details>
											<summary>json</summary>
											<pre>{JSON.stringify(i, null, 2)}</pre>
										</details>
									</li>
								)}
							</For>
						</ul>
					</Show>
				</div>
				<Show when={api.roles.cache.has(editing.id ?? "")}>
					<div class="role-edit">
						<div class="toolbar">
							<button
								onClick={() => {
									setEditing({ id: null });
								}}
							>
								close
							</button>
							<button
								disabled={!isDirty(
									editing as Role,
									api.roles.cache.get(editing.id!)!,
								)}
								onClick={saveRole}
							>
								save
							</button>
							<button onClick={deleteRole(editing.id!)}>delete role</button>
						</div>
						<div>
							id <Copyable>{editing.id!}</Copyable>
						</div>
						<h3>name</h3>
						<input
							type="text"
							value={editName()}
							onInput={(e) => {
								console.log("arst");
								setEditing((r) => ({
									...r,
									name: (e.target as HTMLInputElement).value,
								}));
							}}
						/>
						<h3>description</h3>
						<textarea
							onInput={(e) => {
								setEditing((r) => ({
									...r,
									description: (e.target as HTMLTextAreaElement).value || null,
								}));
							}}
						>
							{editDesc()}
						</textarea>
						<h3>ticky boxes</h3>
						<For
							each={[
								"is_mentionable",
								"is_default",
								"is_self_applicable",
							] as const}
						>
							{(i) => (
								<div>
									<label>
										<input
											type="checkbox"
											checked={(editing as Role)[i]}
											onInput={(e) => {
												setEditing((r) => ({
													...r,
													[i]: (e.target as HTMLInputElement).checked,
												}));
											}}
										/>
										{i}
									</label>
								</div>
							)}
						</For>
						<h3>permissions</h3>
						<ul>
							<For each={permissions}>
								{(perm) => {
									const perms = () => (editing as Role).permissions ?? [];
									return (
										<li>
											<label>
												<input
													type="checkbox"
													checked={perms().includes(perm.id)}
													onInput={(e) => {
														const { checked } = e.target as HTMLInputElement;
														setEditing((r) => {
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
												{perm.id}
											</label>
										</li>
									);
								}}
							</For>
						</ul>
					</div>
				</Show>
			</div>
		</>
	);
}
